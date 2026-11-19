//! `ResourceHttpRequestHandler` do Spring 6.2 para os arquivos do webjar:
//! pré-condições (`checkNotModified`), `Range` e `multipart/byteranges`

use std::sync::OnceLock;
use std::time::UNIX_EPOCH;

use bytes::Bytes;
use chrono::{DateTime, NaiveDateTime, Utc};
use hyper::header::{self, HeaderMap};
use hyper::{Method, StatusCode};
use rand::Rng as _;

use super::erro_boot;
use super::resposta::{Corpo, Problema, Resposta};
use crate::log;

const LOGGER: &str = "o.s.w.s.r.ResourceHttpRequestHandler";
const PREFIXO_RANGE: &str = "bytes=";
const MAX_FAIXAS: usize = 100;
const FORMATO_DATA_HTTP: &str = "%a, %d %b %Y %H:%M:%S GMT";
// formatos do FastHttpDateFormat do Tomcat, sem o dia da semana: o
// SimpleDateFormat leniente não confere se ele bate com a data
const FORMATOS_ENTRADA: [&str; 3] = [
    "%d %b %Y %H:%M:%S GMT",
    "%d-%b-%y %H:%M:%S GMT",
    "%b %e %H:%M:%S %Y",
];
const CARACTERES_BOUNDARY: &[u8] =
    b"-_1234567890abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const TAMANHO_BOUNDARY: std::ops::RangeInclusive<usize> = 30..=40;

pub struct Recurso {
    pub conteudo: &'static [u8],
    pub tipo: &'static str,
    /// `/swagger-ui/swagger-initializer.js` tem mapeamento próprio no springdoc:
    /// transformer com `no-store` e padrão terminado na extensão (seguro contra RFD)
    pub mapeamento_proprio: bool,
}

/// faixa do `HttpRange`: `a-b`, `a-` ou sufixo `-n`
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Faixa {
    Desde(u64, Option<u64>),
    Sufixo(u64),
}

/// trecho já resolvido contra o tamanho do recurso (`ResourceRegion`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Regiao {
    inicio: usize,
    fim: usize,
}

impl Regiao {
    fn tamanho(self) -> usize {
        self.fim - self.inicio + 1
    }

    fn content_range(self, total: usize) -> String {
        format!("bytes {}-{}/{total}", self.inicio, self.fim)
    }
}

/// O Spring usa a data do jar; aqui, a do executável que embute os arquivos
fn ultima_modificacao() -> Option<i64> {
    static DATA: OnceLock<Option<i64>> = OnceLock::new();
    *DATA.get_or_init(|| {
        let data = std::env::current_exe()
            .and_then(|exe| exe.metadata())
            .and_then(|m| m.modified())
            .map_err(|e| e.to_string())
            .and_then(|t| t.duration_since(UNIX_EPOCH).map_err(|e| e.to_string()));
        match data {
            Ok(d) => i64::try_from(d.as_secs()).ok(),
            Err(e) => {
                log::debug(
                    LOGGER,
                    &format!("sem data de modificação dos recursos: {e}"),
                );
                None
            }
        }
    })
}

fn formatar_data(segundos: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp(segundos, 0).map(|d| d.format(FORMATO_DATA_HTTP).to_string())
}

/// `parseDateHeader`: inválida conta como ausente; `;` e o que segue são ignorados
fn data_cabecalho(h: &HeaderMap, nome: header::HeaderName) -> Option<i64> {
    let valor = h.get(nome)?.to_str().ok()?;
    let valor = valor.split(';').next().unwrap_or_default().trim();
    let (_, valor) = valor.split_once(", ").or_else(|| valor.split_once(' '))?;
    FORMATOS_ENTRADA
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(valor, f).ok())
        .map(|d| d.and_utc().timestamp())
}

enum Precondicao {
    Segue,
    NaoModificado,
    Falhou,
}

/// `ServletWebRequest.checkNotModified(null, lastModified)` para GET/HEAD:
/// sem ETag, `If-Match` é ignorado e `If-None-Match` só anula o `If-Modified-Since`
fn precondicao(h: &HeaderMap, modificado: Option<i64>) -> Precondicao {
    let Some(modificado) = modificado else {
        return Precondicao::Segue;
    };
    if let Some(desde) = data_cabecalho(h, header::IF_UNMODIFIED_SINCE) {
        return if desde < modificado {
            Precondicao::Falhou
        } else {
            Precondicao::Segue
        };
    }
    if h.contains_key(header::IF_NONE_MATCH) {
        return Precondicao::Segue;
    }
    match data_cabecalho(h, header::IF_MODIFIED_SINCE) {
        Some(desde) if desde >= modificado => Precondicao::NaoModificado,
        _ => Precondicao::Segue,
    }
}

/// `Long.parseLong`: aceita sinal; negativos são recusados depois
fn numero(texto: &str) -> Option<u64> {
    texto
        .parse::<i64>()
        .ok()
        .and_then(|n| u64::try_from(n).ok())
}

fn faixa(token: &str) -> Option<Faixa> {
    match token.find('-')? {
        0 => numero(&token[1..]).map(Faixa::Sufixo),
        traco => {
            let inicio = numero(&token[..traco])?;
            let resto = &token[traco + 1..];
            if resto.is_empty() {
                return Some(Faixa::Desde(inicio, None));
            }
            let fim = numero(resto)?;
            (fim >= inicio).then_some(Faixa::Desde(inicio, Some(fim)))
        }
    }
}

/// `HttpRange.parseRanges`: `None` vira 416
fn faixas(valor: &str) -> Option<Vec<Faixa>> {
    let lista = valor.strip_prefix(PREFIXO_RANGE)?;
    let tokens: Vec<&str> = lista
        .split(',')
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .collect();
    if tokens.len() > MAX_FAIXAS {
        return None;
    }
    tokens.into_iter().map(faixa).collect()
}

fn regiao(f: Faixa, total: usize) -> Option<Regiao> {
    let total64 = total as u64;
    let (inicio, fim) = match f {
        Faixa::Desde(inicio, fim) => (
            inicio,
            fim.filter(|&f| f < total64)
                .unwrap_or(total64.saturating_sub(1)),
        ),
        Faixa::Sufixo(n) => (total64 - n.min(total64), total64.saturating_sub(1)),
    };
    if inicio >= total64 {
        return None;
    }
    Some(Regiao {
        inicio: usize::try_from(inicio).ok()?,
        fim: usize::try_from(fim).ok()?,
    })
}

/// `HttpRange.toResourceRegions`: com mais de uma faixa, a soma tem de ficar abaixo do total
fn regioes(faixas: &[Faixa], total: usize) -> Option<Vec<Regiao>> {
    let regioes: Vec<Regiao> = faixas
        .iter()
        .map(|&f| regiao(f, total))
        .collect::<Option<_>>()?;
    let soma: usize = regioes.iter().map(|r| r.tamanho()).sum();
    (regioes.len() <= 1 || soma < total).then_some(regioes)
}

fn boundary() -> String {
    let mut r = rand::rng();
    let tamanho = r.random_range(TAMANHO_BOUNDARY);
    (0..tamanho)
        .map(|_| char::from(CARACTERES_BOUNDARY[r.random_range(0..CARACTERES_BOUNDARY.len())]))
        .collect()
}

fn multipart(recurso: &Recurso, regioes: &[Regiao], boundary: &str) -> Vec<u8> {
    let total = recurso.conteudo.len();
    let mut corpo = Vec::new();
    for r in regioes {
        corpo.extend_from_slice(
            format!(
                "\r\n--{boundary}\r\nContent-Type: {}\r\nContent-Range: {}\r\n\r\n",
                recurso.tipo,
                r.content_range(total)
            )
            .as_bytes(),
        );
        corpo.extend_from_slice(&recurso.conteudo[r.inicio..=r.fim]);
    }
    corpo.extend_from_slice(format!("\r\n--{boundary}--").as_bytes());
    corpo
}

fn parcial(recurso: &Recurso, regioes: &[Regiao], base: Resposta) -> Resposta {
    let total = recurso.conteudo.len();
    match regioes {
        [unica] => Resposta {
            tipo: Some(recurso.tipo.into()),
            corpo: Corpo::Fixo(Bytes::from_static(
                &recurso.conteudo[unica.inicio..=unica.fim],
            )),
            ..base
        }
        .com("content-range", &unica.content_range(total)),
        _ => {
            let b = boundary();
            Resposta {
                tipo: Some(format!("multipart/byteranges; boundary={b}")),
                corpo: Corpo::Fluxo(Bytes::from(multipart(recurso, regioes, &b))),
                ..base
            }
        }
    }
}

fn com_data(r: Resposta, modificado: Option<i64>) -> Resposta {
    match modificado.and_then(formatar_data) {
        Some(d) => r.com("last-modified", &d),
        None => r,
    }
}

fn metodo_nao_suportado(metodo: &Method, caminho: &str) -> Resposta {
    Problema::padrao(
        StatusCode::METHOD_NOT_ALLOWED,
        format!("Method '{metodo}' is not supported."),
        caminho,
    )
    .resposta(Utc::now())
    .com("allow", "GET, HEAD")
}

fn intervalo(recurso: &Recurso, valor: &str, base: Resposta, caminho: &str) -> Resposta {
    let total = recurso.conteudo.len();
    match faixas(valor).and_then(|f| regioes(&f, total)) {
        Some(r) => parcial(recurso, &r, base),
        None => {
            let mut erro = erro_boot(StatusCode::RANGE_NOT_SATISFIABLE, caminho);
            erro.cabecalhos.extend(base.cabecalhos);
            erro.com("content-range", &format!("bytes */{total}"))
        }
    }
}

fn entregar(recurso: &Recurso, h: &HeaderMap, caminho: &str) -> Resposta {
    let modificado = ultima_modificacao();
    match precondicao(h, modificado) {
        Precondicao::Falhou => return Resposta::new(StatusCode::PRECONDITION_FAILED),
        Precondicao::NaoModificado => {
            return com_data(Resposta::new(StatusCode::NOT_MODIFIED), modificado);
        }
        Precondicao::Segue => {}
    }
    let base = com_data(Resposta::new(StatusCode::OK), modificado).com("accept-ranges", "bytes");
    match h.get(header::RANGE) {
        None => Resposta {
            tipo: Some(recurso.tipo.into()),
            corpo: Corpo::Fixo(Bytes::from_static(recurso.conteudo)),
            ..base
        },
        Some(v) => {
            let base = Resposta {
                status: StatusCode::PARTIAL_CONTENT,
                ..base
            };
            intervalo(recurso, v.to_str().unwrap_or_default(), base, caminho)
        }
    }
}

pub fn servir(recurso: &Recurso, metodo: &Method, h: &HeaderMap, caminho: &str) -> Resposta {
    if *metodo == Method::OPTIONS {
        return Resposta::new(StatusCode::OK).com("allow", "GET,HEAD,OPTIONS");
    }
    if *metodo != Method::GET && *metodo != Method::HEAD {
        let r = metodo_nao_suportado(metodo, caminho);
        return Resposta {
            conversor: r.conversor && !recurso.mapeamento_proprio,
            ..r
        };
    }
    let r = entregar(recurso, h, caminho);
    // o transformer marca o recurso antes das pré-condições
    if recurso.mapeamento_proprio {
        r.com("cache-control", "no-store")
    } else {
        r
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn faixas_como_http_range() {
        assert_eq!(faixas("bytes=0-9"), Some(vec![Faixa::Desde(0, Some(9))]));
        assert_eq!(
            faixas("bytes= 0-1 , 3-,-7,"),
            Some(vec![
                Faixa::Desde(0, Some(1)),
                Faixa::Desde(3, None),
                Faixa::Sufixo(7)
            ])
        );
        assert_eq!(faixas("bytes=+1-2"), Some(vec![Faixa::Desde(1, Some(2))]));
        assert_eq!(faixas("bytes="), Some(vec![]));
        for invalida in [
            "BYTES=0-1",
            "bites=0-1",
            "bytes=9-2",
            "bytes=-1-2",
            "bytes=abc",
            "bytes=0 -1",
        ] {
            assert_eq!(faixas(invalida), None, "{invalida}");
        }
    }

    #[test]
    fn regioes_contra_o_tamanho() {
        let r = |v: &str| faixas(v).and_then(|f| regioes(&f, 202));
        assert_eq!(
            r("bytes=0-9999"),
            Some(vec![Regiao {
                inicio: 0,
                fim: 201
            }])
        );
        assert_eq!(
            r("bytes=-9999"),
            Some(vec![Regiao {
                inicio: 0,
                fim: 201
            }])
        );
        assert_eq!(
            r("bytes=-7"),
            Some(vec![Regiao {
                inicio: 195,
                fim: 201
            }])
        );
        assert_eq!(r("bytes=300-400"), None);
        assert_eq!(r("bytes=-0"), None);
        assert_eq!(r("bytes=0-100,101-201"), None);
        assert!(r("bytes=0-99,101-201").is_some());
    }

    #[test]
    fn corpo_multipart() {
        let recurso = Recurso {
            conteudo: b"abcdef",
            tipo: "text/css",
            mapeamento_proprio: false,
        };
        let regioes = [Regiao { inicio: 0, fim: 0 }, Regiao { inicio: 2, fim: 3 }];
        assert_eq!(
            multipart(&recurso, &regioes, "B"),
            b"\r\n--B\r\nContent-Type: text/css\r\nContent-Range: bytes 0-0/6\r\n\r\na\
              \r\n--B\r\nContent-Type: text/css\r\nContent-Range: bytes 2-3/6\r\n\r\ncd\r\n--B--"
        );
        let b = boundary();
        assert!(TAMANHO_BOUNDARY.contains(&b.len()));
    }

    #[test]
    fn datas_http() {
        let mut h = HeaderMap::new();
        h.insert(
            header::IF_MODIFIED_SINCE,
            "Mon, 18 Nov 2026 18:16:58 GMT; length=3"
                .parse()
                .unwrap_or_else(|_| unreachable!()),
        );
        let lido = data_cabecalho(&h, header::IF_MODIFIED_SINCE);
        assert_eq!(
            lido.and_then(formatar_data).as_deref(),
            Some("Wed, 18 Nov 2026 18:16:58 GMT")
        );
        assert!(matches!(precondicao(&h, lido), Precondicao::NaoModificado));
        h.insert(
            header::IF_NONE_MATCH,
            "*".parse().unwrap_or_else(|_| unreachable!()),
        );
        assert!(matches!(precondicao(&h, lido), Precondicao::Segue));
        h.insert(
            header::IF_UNMODIFIED_SINCE,
            "Mon, 01 Jan 2001 00:00:00 GMT"
                .parse()
                .unwrap_or_else(|_| unreachable!()),
        );
        assert!(matches!(precondicao(&h, lido), Precondicao::Falhou));
    }
}
