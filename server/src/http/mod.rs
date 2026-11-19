//! camada HTTP com o mesmo contrato observável do Spring Boot 3.5 sobre
//! Tomcat 10.1: context-path `/api`, firewall, cabeçalhos do Spring Security,
//! CORS, negociação de conteúdo, `ProblemDetail` e compressão

mod base_calculo;
mod calculadora;
mod dados_abertos;
mod erro;
mod gestao;
mod midia;
mod nfse;
mod pedagio;
mod pedido;
mod recurso;
mod resposta;
mod rfd;
mod rotas;
mod springdoc;
mod tabela;
mod tomcat;
mod versao;
mod xml;

use std::convert::Infallible;
use std::sync::Arc;
use std::time::Instant;

use bytes::Bytes;
use chrono::{SecondsFormat, Utc};
use http_body_util::BodyExt as _;
use hyper::body::Incoming;
use hyper::ext::ReasonPhrase;
use hyper::header::{self, HeaderMap, HeaderName, HeaderValue};
use hyper::http::request::Parts;
use hyper::{Method, Request, Response, StatusCode};
use percent_encoding::percent_decode_str;
use rand::Rng as _;
use rtc_core::db::Pool;
use rtc_core::erro::{Falha, Resultado};
use rtc_core::servico::Servicos;

use crate::log;
use crate::metricas::{Metricas, Rotulos};
use erro::ErroHttp;
pub use gestao::atender as atender_gestao;
use midia::{Midia, OCTET_STREAM};
use pedido::Pedido;
use resposta::{Corpo, Problema, Resposta};
use rotas::Rota;
pub use tomcat::CorpoHttp;
pub use versao::verificar_na_inicializacao;

pub const CONTEXTO: &str = "/api";
pub const VERSAO_APLICACAO: &str = "1.6.1";
const LOGGER_HANDLER: &str = "b.g.s.r.a.e.ApiExceptionHandler";
const URI_DESCONHECIDA: &str = "UNKNOWN";
const URI_RECURSO_ESTATICO: &str = "/**";
const SEM_EXCECAO: &str = "none";
const EXCECAO_FIREWALL: &str = "RequestRejectedException";
const CORS_INVALIDO: &str = "Invalid CORS request";
const TAM_TRACE: usize = 32;
const TAM_SPAN: usize = 16;

pub struct App {
    pub pool: Pool,
    versao_db: Option<String>,
    rotas: Vec<Rota>,
    pub metricas: Arc<Metricas>,
    pub url_versao: String,
    pub plataformas_download: Vec<(String, String)>,
    pub porta: u16,
}

/// configuração externa que os controllers consultam
pub struct Ambiente {
    pub url_versao: String,
    pub plataformas_download: Vec<(String, String)>,
    pub porta: u16,
}

impl App {
    pub fn new(pool: Pool, metricas: Arc<Metricas>, ambiente: Ambiente) -> Resultado<Self> {
        let versao_db = pool.base()?.ultima_versao()?.map(|v| v.numero);
        Ok(Self {
            pool,
            versao_db,
            rotas: tabela::rotas(),
            metricas,
            url_versao: ambiente.url_versao,
            plataformas_download: ambiente.plataformas_download,
            porta: ambiente.porta,
        })
    }

    pub fn com_servicos<T>(&self, f: impl FnOnce(&Servicos<'_>) -> Resultado<T>) -> Resultado<T> {
        let base = self.pool.base()?;
        f(&Servicos::new(&base))
    }

    pub fn cabecalhos_versao(&self, r: Resposta) -> Result<Resposta, Falha> {
        let db = self.versao_db.as_deref().ok_or_else(|| {
            Falha::sistema("Cannot invoke \"br.gov.serpro.rtc.domain.model.entity.VersaoBaseDados.getNumeroVersao()\" because the return value of \"br.gov.serpro.rtc.domain.service.VersaoBaseDadosService.getUltimaVersao()\" is null")
        })?;
        Ok(r.com("x-calc-app-version", VERSAO_APLICACAO)
            .com("x-calc-db-version", db))
    }
}

/// resposta já decidida e o que os filtros externos devem acrescentar
struct Saida {
    resposta: Resposta,
    /// passou pelo Spring Security (cabeçalhos de segurança e `Vary` do CORS)
    filtrada: bool,
    uri: String,
    excecao: &'static str,
}

impl Saida {
    fn tomcat(resposta: Resposta) -> Self {
        Self {
            resposta,
            filtrada: false,
            uri: String::new(),
            excecao: SEM_EXCECAO,
        }
    }

    fn spring(resposta: Resposta, uri: impl Into<String>) -> Self {
        Self {
            resposta,
            filtrada: true,
            uri: uri.into(),
            excecao: SEM_EXCECAO,
        }
    }
}

fn hex_aleatorio(n: usize) -> String {
    let mut r = rand::rng();
    (0..n)
        .map(|_| char::from(b"0123456789abcdef"[r.random_range(0..16)]))
        .collect()
}

/// W3C `traceparent` recebido continua o trace; senão abre um novo
fn ids_trace(h: &HeaderMap) -> (String, String) {
    let herdado = h
        .get("traceparent")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split('-').nth(1))
        .filter(|t| t.len() == TAM_TRACE && t.bytes().all(|b| b.is_ascii_hexdigit()))
        .map(str::to_ascii_lowercase);
    (
        herdado.unwrap_or_else(|| hex_aleatorio(TAM_TRACE)),
        hex_aleatorio(TAM_SPAN),
    )
}

pub async fn atender(
    app: Arc<App>,
    req: Request<Incoming>,
) -> Result<Response<CorpoHttp>, Infallible> {
    let inicio = Instant::now();
    let (partes, corpo) = req.into_parts();
    let caminho = partes.uri.path().to_string();
    let corpo = match corpo.collect().await {
        Ok(c) => c.to_bytes(),
        Err(e) => {
            log::debug(
                LOGGER_HANDLER,
                &format!("falha ao ler corpo da requisição: {e}"),
            );
            return Ok(finalizar(
                Saida::tomcat(Resposta::new(StatusCode::BAD_REQUEST)),
                &partes.headers,
                &caminho,
            ));
        }
    };
    let cabecalhos = partes.headers.clone();
    let metodo = partes.method.clone();
    let app_bloqueante = Arc::clone(&app);
    let saida = tokio::task::spawn_blocking(move || {
        let (trace, span) = ids_trace(&partes.headers);
        let _escopo = log::Escopo::entrar(&trace, &span);
        processar(&app_bloqueante, &partes, &corpo)
    })
    .await;
    let saida = match saida {
        Ok(s) => s,
        Err(e) => {
            log::erro(LOGGER_HANDLER, &format!("Erro interno: \n{e}"));
            Saida::tomcat(Resposta::new(StatusCode::INTERNAL_SERVER_ERROR))
        }
    };
    if saida.filtrada {
        app.metricas.registrar(
            Rotulos {
                excecao: saida.excecao.to_string(),
                metodo: metodo.to_string(),
                status: saida.resposta.status.as_u16(),
                uri: saida.uri.clone(),
            },
            inicio.elapsed(),
        );
    }
    Ok(finalizar(saida, &cabecalhos, &caminho))
}

/// `CacheControlHeadersWriter` não escreve nada se o controller já definiu
/// cache, nem em 304
fn cabecalhos_seguranca(h: &mut HeaderMap, status: StatusCode) {
    let cache_do_controller = status == StatusCode::NOT_MODIFIED
        || [header::CACHE_CONTROL, header::PRAGMA, header::EXPIRES]
            .iter()
            .any(|k| h.contains_key(k));
    for (k, v, de_cache) in [
        ("x-content-type-options", "nosniff", false),
        ("x-xss-protection", "0", false),
        (
            "cache-control",
            "no-cache, no-store, max-age=0, must-revalidate",
            true,
        ),
        ("pragma", "no-cache", true),
        ("expires", "0", true),
        ("x-frame-options", "SAMEORIGIN", false),
    ] {
        if !(de_cache && cache_do_controller) {
            h.append(HeaderName::from_static(k), HeaderValue::from_static(v));
        }
    }
}

fn vary_cors(h: &mut HeaderMap) {
    for v in [
        "Origin",
        "Access-Control-Request-Method",
        "Access-Control-Request-Headers",
    ] {
        h.append(header::VARY, HeaderValue::from_static(v));
    }
}

fn finalizar(saida: Saida, pedido: &HeaderMap, caminho: &str) -> Response<CorpoHttp> {
    let r = saida.resposta;
    let rfd = rfd::exige(caminho, &r);
    let mut h = HeaderMap::new();
    if saida.filtrada {
        vary_cors(&mut h);
    }
    for (k, v) in r.cabecalhos {
        h.append(k, v);
    }
    if saida.filtrada {
        cabecalhos_seguranca(&mut h, r.status);
    }
    if rfd && !h.contains_key(header::CONTENT_DISPOSITION) {
        h.append(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_static(rfd::CABECALHO),
        );
    }
    if let Some(t) = r
        .tipo
        .as_deref()
        .and_then(|t| HeaderValue::from_str(t).ok())
    {
        h.insert(header::CONTENT_TYPE, t);
    }
    let corpo = match r.corpo {
        Corpo::Vazio => CorpoHttp::vazio(),
        Corpo::Fixo(b) => CorpoHttp::fixo(b),
        Corpo::Fluxo(b) => CorpoHttp::fluxo(b),
    };
    let corpo = tomcat::comprimir(pedido, &mut h, corpo);
    if tomcat::fecha_conexao(r.status) {
        h.insert(header::CONNECTION, HeaderValue::from_static("close"));
    }
    let mut resp = Response::new(corpo);
    *resp.status_mut() = r.status;
    *resp.headers_mut() = h;
    // o Tomcat não envia reason phrase
    if let Ok(motivo) = ReasonPhrase::try_from(Bytes::new()) {
        resp.extensions_mut().insert(motivo);
    }
    resp
}

/// `HttpUtils.getBaseURL`: esquema e host do pedido, sem porta
fn base_url(h: &HeaderMap) -> String {
    let host = h
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    let host = if host.starts_with('[') {
        host.split_once(']').map_or(host, |(h, _)| h).to_string() + "]"
    } else {
        host.split(':').next().unwrap_or(host).to_string()
    };
    format!("http://{host}")
}

/// `StrictHttpFirewall`: caminhos não normalizados ou com caracteres proibidos
fn rejeitado_pelo_firewall(caminho: &str, metodo: &Method) -> bool {
    const PROIBIDOS: [&str; 9] = ["//", "/./", "/../", ";", "\\", "%25", "%2e", "%2f", "%5c"];
    const METODOS: [Method; 7] = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::OPTIONS,
        Method::HEAD,
        Method::PATCH,
    ];
    let minusculo = caminho.to_ascii_lowercase();
    PROIBIDOS.iter().any(|p| minusculo.contains(p))
        || minusculo.ends_with("/.")
        || minusculo.ends_with("/..")
        || minusculo.contains("%3b")
        || minusculo.contains("%00")
        || caminho.bytes().any(|b| !(0x20..0x7f).contains(&b))
        || !METODOS.contains(metodo)
}

/// `HttpStatus.getReasonPhrase` onde difere do texto da RFC 9110
fn motivo_spring(status: StatusCode) -> &'static str {
    match status {
        StatusCode::RANGE_NOT_SATISFIABLE => "Requested range not satisfiable",
        _ => status.canonical_reason().unwrap_or_default(),
    }
}

/// página de erro do Spring Boot (`BasicErrorController`) para pedidos rejeitados
fn erro_boot(status: StatusCode, caminho: &str) -> Resposta {
    let corpo = format!(
        "{{\"timestamp\":\"{}\",\"status\":{},\"error\":\"{}\",\"path\":\"{}\"}}",
        Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true),
        status.as_u16(),
        motivo_spring(status),
        caminho.replace('\\', "\\\\").replace('"', "\\\"")
    );
    Resposta::json(status, corpo)
}

fn tomcat_404() -> Resposta {
    Resposta {
        status: StatusCode::NOT_FOUND,
        cabecalhos: vec![(header::CONTENT_LANGUAGE, HeaderValue::from_static("en"))],
        tipo: Some("text/html;charset=utf-8".into()),
        corpo: Corpo::Fixo(Bytes::from_static(tomcat::pagina_404().as_bytes())),
        conversor: false,
    }
}

fn eh_preflight(h: &HeaderMap, metodo: &Method, base: &str) -> bool {
    let origem = h.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let mesma_origem = origem.is_some_and(|o| {
        h.get(header::HOST)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|host| o == format!("http://{host}") || o == base)
    });
    *metodo == Method::OPTIONS
        && origem.is_some()
        && !mesma_origem
        && h.contains_key(header::ACCESS_CONTROL_REQUEST_METHOD)
}

fn decodificar_segmentos(relativo: &str) -> Vec<String> {
    relativo
        .trim_start_matches('/')
        .split('/')
        .map(|s| percent_decode_str(s).decode_utf8_lossy().into_owned())
        .collect()
}

fn consulta(q: Option<&str>) -> Vec<(String, String)> {
    q.map(|q| form_urlencoded::parse(q.as_bytes()).into_owned().collect())
        .unwrap_or_default()
}

fn processar(app: &App, partes: &Parts, corpo: &[u8]) -> Saida {
    let caminho = partes.uri.path();
    if caminho == CONTEXTO {
        let r = Resposta {
            corpo: Corpo::Fluxo(Bytes::new()),
            ..Resposta::new(StatusCode::FOUND)
        }
        .com("location", "/api/");
        return Saida::tomcat(r);
    }
    let Some(relativo) = caminho
        .strip_prefix(CONTEXTO)
        .filter(|r| r.starts_with('/'))
    else {
        return Saida::tomcat(tomcat_404());
    };
    if rejeitado_pelo_firewall(caminho, &partes.method) {
        return Saida {
            excecao: EXCECAO_FIREWALL,
            uri: URI_DESCONHECIDA.into(),
            filtrada: false,
            resposta: erro_boot(StatusCode::BAD_REQUEST, caminho),
        };
    }
    let base = base_url(&partes.headers);
    if eh_preflight(&partes.headers, &partes.method, &base) {
        let r = Resposta {
            corpo: Corpo::Fluxo(Bytes::from_static(CORS_INVALIDO.as_bytes())),
            ..Resposta::new(StatusCode::FORBIDDEN)
        };
        return Saida::spring(r, URI_DESCONHECIDA);
    }
    let segmentos = decodificar_segmentos(relativo);
    let Some(casamento) = rotas::resolver(&app.rotas, &segmentos) else {
        if let Some(r) = springdoc::recurso(&segmentos) {
            let resposta = recurso::servir(&r, &partes.method, &partes.headers, caminho);
            return Saida::spring(resposta, URI_RECURSO_ESTATICO);
        }
        let recurso = springdoc::relativo_ao_mapeamento(&segmentos).join("/");
        let recurso = recurso.trim_end_matches('/');
        let p = Problema::padrao(
            StatusCode::NOT_FOUND,
            format!("No static resource {recurso}."),
            caminho,
        );
        return Saida::spring(p.resposta(Utc::now()), URI_RECURSO_ESTATICO);
    };
    let contexto = Contexto {
        app,
        partes,
        corpo,
        base: &base,
    };
    contexto.despachar(casamento)
}

struct Contexto<'a> {
    app: &'a App,
    partes: &'a Parts,
    corpo: &'a [u8],
    base: &'a str,
}

impl Contexto<'_> {
    fn caminho(&self) -> &str {
        self.partes.uri.path()
    }

    fn problema(&self, status: StatusCode, detalhe: String) -> Resposta {
        Problema::padrao(status, detalhe, self.caminho()).resposta(Utc::now())
    }

    fn despachar(&self, casamento: rotas::Casamento<'_>) -> Saida {
        let metodo = &self.partes.method;
        let uri = casamento
            .rotas
            .first()
            .map_or(URI_DESCONHECIDA, |r| r.padrao);
        let efetivo = if *metodo == Method::HEAD {
            &Method::GET
        } else {
            metodo
        };
        let Some(rota) = casamento.rotas.iter().find(|r| r.metodo == *efetivo) else {
            if *metodo == Method::OPTIONS {
                let r = Resposta::new(StatusCode::OK)
                    .com("allow", &rotas::metodos_options(&casamento.rotas))
                    .com("accept-patch", "");
                return Saida::spring(r, URI_DESCONHECIDA);
            }
            let r = self
                .problema(
                    StatusCode::METHOD_NOT_ALLOWED,
                    format!("Method '{metodo}' is not supported."),
                )
                .com("allow", &rotas::metodos_permitidos(&casamento.rotas));
            return Saida::spring(r, uri);
        };
        let corpo = match self.negociar(rota) {
            Ok(c) => c,
            Err(r) => return Saida::spring(r, uri),
        };
        let consulta = consulta(self.partes.uri.query());
        let pedido = Pedido {
            base_url: self.base,
            cabecalhos: &self.partes.headers,
            consulta: &consulta,
            variaveis: casamento.variaveis,
            corpo: &corpo,
        };
        let resposta = match (rota.manipulador)(self.app, &pedido) {
            Ok(r) => r,
            Err(e) => self.tratar(&e),
        };
        Saida::spring(resposta, rota.padrao)
    }

    /// `consumes`/`produces` do `RequestMappingInfo` e charset do corpo
    fn negociar(&self, rota: &Rota) -> Result<Vec<u8>, Resposta> {
        let h = &self.partes.headers;
        let mut charset = None;
        if let Some(consome) = rota.consome {
            let bruto = h
                .get(header::CONTENT_TYPE)
                .and_then(|v| v.to_str().ok())
                .filter(|v| !v.is_empty())
                .unwrap_or(OCTET_STREAM);
            let Some(m) =
                Midia::parse(bruto).filter(|m| m.charset().is_none_or(midia::charset_conhecido))
            else {
                return Err(self.nao_suportado("Could not parse Content-Type.".into(), consome));
            };
            if !Midia::parse(consome).is_some_and(|c| c.inclui(&m)) {
                return Err(
                    self.nao_suportado(format!("Content-Type '{m}' is not supported."), consome)
                );
            }
            charset = m.charset().map(str::to_string);
        }
        let produzido = Midia::parse(rota.produz).unwrap_or_else(Midia::json);
        let accept = h.get(header::ACCEPT).map(|v| v.to_str().unwrap_or("\u{0}"));
        match midia::aceita(accept, &produzido) {
            Some(true) if rota.consome == Some(midia::XML) => {
                Ok(midia::texto(self.corpo, charset.as_deref()).into_bytes())
            }
            Some(true) => Ok(midia::decodificar(self.corpo, charset.as_deref())),
            Some(false) => Err(self
                .problema(
                    StatusCode::NOT_ACCEPTABLE,
                    format!("Acceptable representations: [{produzido}]."),
                )
                .com("accept", rota.produz)),
            // accept ilegível sai antes do ProblemDetail, com corpo vazio
            None => Err(Resposta::new(StatusCode::NOT_ACCEPTABLE).com("accept", rota.produz)),
        }
    }

    fn nao_suportado(&self, detalhe: String, consome: &str) -> Resposta {
        self.problema(StatusCode::UNSUPPORTED_MEDIA_TYPE, detalhe)
            .com("accept", consome)
    }

    fn tratar(&self, e: &ErroHttp) -> Resposta {
        let p = e.problema(self.base, self.caminho());
        if p.status.is_server_error() {
            log::erro(LOGGER_HANDLER, &format!("Erro interno: \n{}", e.mensagem()));
        } else {
            log::debug(
                LOGGER_HANDLER,
                &format!(
                    "Erro cliente [{}]: {} | URI: uri={}",
                    p.status.as_u16(),
                    e.mensagem(),
                    self.caminho()
                ),
            );
        }
        p.resposta(Utc::now())
    }
}
