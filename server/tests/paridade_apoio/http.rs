//! cliente HTTP/1.1 mínimo que manda o mesmo que o `urllib.request` mandou na
//! gravação (cabeçalhos implícitos, `Connection: close`, redirecionamentos)

use std::io::{BufRead as _, BufReader, Read as _, Write as _};
use std::net::TcpStream;
use std::time::Duration;

use super::{Ambiente, Caso};

const TEMPO_LIMITE: Duration = Duration::from_secs(60);
const MAX_REDIRECIONAMENTOS: usize = 10;
const REDIRECIONAMENTOS: [u16; 5] = [301, 302, 303, 307, 308];
// o urllib marca corpo sem tipo como formulário
const TIPO_PADRAO_CORPO: &str = "application/x-www-form-urlencoded";

pub struct Obtida {
    pub status: u16,
    pub cabecalhos: Vec<(String, String)>,
    pub corpo: Vec<u8>,
}

struct Pedido<'a> {
    metodo: &'a str,
    caminho: String,
    cabecalhos: &'a [(String, String)],
    corpo: Option<&'a [u8]>,
}

fn tem(cabecalhos: &[(String, String)], nome: &str) -> bool {
    cabecalhos.iter().any(|(k, _)| k.eq_ignore_ascii_case(nome))
}

fn montar(porta: u16, p: &Pedido<'_>) -> Vec<u8> {
    let mut linhas = vec![format!("{} {} HTTP/1.1", p.metodo, p.caminho)];
    let mut implicitos = vec![("Accept-Encoding".to_string(), "identity".to_string())];
    if let Some(c) = p.corpo {
        implicitos.push(("Content-Length".into(), c.len().to_string()));
        if !tem(p.cabecalhos, "content-type") {
            implicitos.push(("Content-Type".into(), TIPO_PADRAO_CORPO.into()));
        }
    }
    if !tem(p.cabecalhos, "host") {
        implicitos.push(("Host".into(), format!("localhost:{porta}")));
    }
    implicitos.push(("User-Agent".into(), "Python-urllib/3.13".into()));
    for (k, v) in implicitos.iter().chain(p.cabecalhos) {
        linhas.push(format!("{k}: {v}"));
    }
    linhas.push("Connection: close".into());
    let mut bruto = (linhas.join("\r\n") + "\r\n\r\n").into_bytes();
    bruto.extend_from_slice(p.corpo.unwrap_or_default());
    bruto
}

fn ler_chunked(leitor: &mut impl std::io::BufRead) -> Vec<u8> {
    let mut corpo = Vec::new();
    loop {
        let mut linha = String::new();
        if leitor.read_line(&mut linha).unwrap_or(0) == 0 {
            return corpo;
        }
        let tamanho = usize::from_str_radix(linha.trim().split(';').next().unwrap_or("0"), 16)
            .unwrap_or_else(|e| panic!("chunk {linha:?}: {e}"));
        if tamanho == 0 {
            return corpo;
        }
        let mut pedaco = vec![0; tamanho + 2];
        leitor
            .read_exact(&mut pedaco)
            .unwrap_or_else(|e| panic!("chunk: {e}"));
        corpo.extend_from_slice(&pedaco[..tamanho]);
    }
}

fn sem_corpo(metodo: &str, status: u16) -> bool {
    metodo == "HEAD" || status == 204 || status == 304 || (100..200).contains(&status)
}

fn uma_vez(porta: u16, p: &Pedido<'_>) -> Obtida {
    let mut conexao =
        TcpStream::connect(("127.0.0.1", porta)).unwrap_or_else(|e| panic!("conectar: {e}"));
    conexao
        .set_read_timeout(Some(TEMPO_LIMITE))
        .unwrap_or_else(|e| panic!("timeout: {e}"));
    conexao
        .write_all(&montar(porta, p))
        .unwrap_or_else(|e| panic!("enviar: {e}"));
    let mut leitor = BufReader::new(conexao);
    let mut linha = String::new();
    leitor
        .read_line(&mut linha)
        .unwrap_or_else(|e| panic!("status: {e}"));
    let status = linha
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .unwrap_or_else(|| panic!("linha de status {linha:?}"));
    let mut cabecalhos = Vec::new();
    loop {
        linha.clear();
        leitor
            .read_line(&mut linha)
            .unwrap_or_else(|e| panic!("cabeçalho: {e}"));
        let l = linha.trim_end_matches(['\r', '\n']);
        if l.is_empty() {
            break;
        }
        if let Some((k, v)) = l.split_once(':') {
            cabecalhos.push((k.trim().to_ascii_lowercase(), v.trim().to_string()));
        }
    }
    let valor = |nome: &str| {
        cabecalhos
            .iter()
            .find(|(k, _)| k == nome)
            .map(|(_, v)| v.clone())
    };
    let corpo = if sem_corpo(p.metodo, status) {
        Vec::new()
    } else if valor("transfer-encoding").is_some_and(|t| t.eq_ignore_ascii_case("chunked")) {
        ler_chunked(&mut leitor)
    } else if let Some(n) = valor("content-length").and_then(|n| n.parse::<usize>().ok()) {
        let mut c = vec![0; n];
        leitor
            .read_exact(&mut c)
            .unwrap_or_else(|e| panic!("corpo: {e}"));
        c
    } else {
        let mut c = Vec::new();
        leitor
            .read_to_end(&mut c)
            .unwrap_or_else(|e| panic!("corpo: {e}"));
        c
    };
    Obtida {
        status,
        cabecalhos,
        corpo,
    }
}

/// caminho do `Location`, absoluto ou relativo à raiz
fn destino(location: &str) -> String {
    match location.split_once("://") {
        Some((_, resto)) => resto
            .find('/')
            .map_or_else(|| "/".to_string(), |i| resto[i..].to_string()),
        None => location.to_string(),
    }
}

/// `HTTPRedirectHandler`: segue redirecionamento só de GET e HEAD
pub fn enviar(ambiente: &Ambiente, caso: &Caso, seguir: bool) -> Obtida {
    let mut pedido = Pedido {
        metodo: &caso.metodo,
        caminho: caso.caminho.clone(),
        cabecalhos: &caso.cabecalhos,
        corpo: caso.corpo.as_deref(),
    };
    for _ in 0..MAX_REDIRECIONAMENTOS {
        let r = uma_vez(ambiente.porta, &pedido);
        let location = r
            .cabecalhos
            .iter()
            .find(|(k, _)| k == "location")
            .map(|(_, v)| v.clone());
        let redireciona = seguir
            && REDIRECIONAMENTOS.contains(&r.status)
            && matches!(pedido.metodo, "GET" | "HEAD");
        match location {
            Some(l) if redireciona => pedido.caminho = destino(&l),
            _ => return r,
        }
    }
    panic!("{}: redirecionamentos demais", caso.id)
}
