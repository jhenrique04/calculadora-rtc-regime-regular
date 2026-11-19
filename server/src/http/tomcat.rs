//! comportamento do conector do Tomcat 10.1: compressão (`CompressionConfig`),
//! `Vary`, fechamento de conexão por status e a página 404 padrão

use std::convert::Infallible;
use std::io::Write as _;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use flate2::Compression;
use flate2::write::GzEncoder;
use http_body::{Body, Frame, SizeHint};
use hyper::StatusCode;
use hyper::header::{self, HeaderMap, HeaderValue};

pub const MIME_COMPRESSIVEL: &str = "application/json";
pub const TAMANHO_MINIMO_COMPRESSAO: usize = 1024;
const VARY_TODOS: &str = "*";
const CAMPO_ACCEPT_ENCODING: &str = "accept-encoding";

/// corpo de resposta: tamanho exato vira `Content-Length`, senão `chunked`
pub struct CorpoHttp {
    dados: Option<Bytes>,
    exato: bool,
}

impl CorpoHttp {
    pub fn fixo(dados: Bytes) -> Self {
        Self {
            dados: Some(dados),
            exato: true,
        }
    }

    pub fn fluxo(dados: Bytes) -> Self {
        Self {
            dados: Some(dados),
            exato: false,
        }
    }

    pub fn vazio() -> Self {
        Self::fixo(Bytes::new())
    }

    pub fn tamanho(&self) -> usize {
        self.dados.as_ref().map_or(0, Bytes::len)
    }
}

impl Body for CorpoHttp {
    type Data = Bytes;
    type Error = Infallible;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Bytes>, Infallible>>> {
        Poll::Ready(
            self.dados
                .take()
                .filter(|d| !d.is_empty())
                .map(|d| Ok(Frame::data(d))),
        )
    }

    fn is_end_stream(&self) -> bool {
        self.exato && self.dados.as_ref().is_none_or(Bytes::is_empty)
    }

    fn size_hint(&self) -> SizeHint {
        if self.exato {
            SizeHint::with_exact(self.tamanho() as u64)
        } else {
            SizeHint::default()
        }
    }
}

pub fn fecha_conexao(s: StatusCode) -> bool {
    matches!(s.as_u16(), 400 | 408 | 411 | 413 | 414 | 500 | 503 | 501)
}

/// `TokenList.parseTokenList`: tokens em minúsculas, sem duplicatas, na ordem
fn tokens_vary(h: &HeaderMap) -> Vec<String> {
    let mut v: Vec<String> = Vec::new();
    for valor in h.get_all(header::VARY) {
        let Ok(s) = valor.to_str() else { continue };
        for t in s.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            let t = t.to_ascii_lowercase();
            if !v.contains(&t) {
                v.push(t);
            }
        }
    }
    v
}

fn adicionar_vary(h: &mut HeaderMap) {
    let atuais: Vec<HeaderValue> = h.get_all(header::VARY).iter().cloned().collect();
    if atuais.len() == 1 && atuais[0].to_str().is_ok_and(|s| s.trim() == VARY_TODOS) {
        return;
    }
    if atuais.is_empty() {
        h.insert(
            header::VARY,
            HeaderValue::from_static(CAMPO_ACCEPT_ENCODING),
        );
        return;
    }
    let mut tokens = tokens_vary(h);
    if tokens.iter().any(|t| t == VARY_TODOS) {
        h.insert(header::VARY, HeaderValue::from_static(VARY_TODOS));
        return;
    }
    if !tokens.iter().any(|t| t == CAMPO_ACCEPT_ENCODING) {
        tokens.push(CAMPO_ACCEPT_ENCODING.into());
    }
    if let Ok(v) = HeaderValue::from_str(&tokens.join(",")) {
        h.insert(header::VARY, v);
    }
}

fn aceita_gzip(pedido: &HeaderMap) -> bool {
    pedido
        .get_all(header::ACCEPT_ENCODING)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|s| s.split(','))
        .filter_map(|e| e.split(';').next())
        .any(|e| e.trim().eq_ignore_ascii_case("gzip"))
}

fn gzip(dados: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut enc = GzEncoder::new(Vec::new(), Compression::default());
    enc.write_all(dados)?;
    enc.finish()
}

/// `CompressionConfig.useCompression` + aplicação do `GzipOutputFilter`
pub fn comprimir(pedido: &HeaderMap, resposta: &mut HeaderMap, corpo: CorpoHttp) -> CorpoHttp {
    if corpo.tamanho() == 0 || resposta.contains_key(header::CONTENT_ENCODING) {
        return corpo;
    }
    if corpo.exato && corpo.tamanho() < TAMANHO_MINIMO_COMPRESSAO {
        return corpo;
    }
    let compressivel = resposta
        .get(header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|t| t.starts_with(MIME_COMPRESSIVEL));
    if !compressivel {
        return corpo;
    }
    adicionar_vary(resposta);
    if !aceita_gzip(pedido) {
        return corpo;
    }
    let Some(dados) = corpo.dados.as_ref() else {
        return corpo;
    };
    match gzip(dados) {
        Ok(z) => {
            resposta.remove(header::CONTENT_LENGTH);
            resposta.insert(header::CONTENT_ENCODING, HeaderValue::from_static("gzip"));
            CorpoHttp::fluxo(Bytes::from(z))
        }
        // compressão em memória não falha na prática; se falhar, vai sem gzip
        Err(_) => corpo,
    }
}

/// `ErrorReportValve` do Tomcat para caminhos fora do context-path
pub fn pagina_404() -> &'static str {
    concat!(
        "<!doctype html><html lang=\"en\"><head><title>HTTP Status 404 – Not Found</title>",
        "<style type=\"text/css\">body {font-family:Tahoma,Arial,sans-serif;} h1, h2, h3, b ",
        "{color:white;background-color:#525D76;} h1 {font-size:22px;} h2 {font-size:16px;} ",
        "h3 {font-size:14px;} p {font-size:12px;} a {color:black;} .line {height:1px;",
        "background-color:#525D76;border:none;}</style></head><body><h1>HTTP Status 404 – ",
        "Not Found</h1></body></html>"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vary_unificado_como_tomcat() {
        let mut h = HeaderMap::new();
        for v in [
            "Origin",
            "Access-Control-Request-Method",
            "Access-Control-Request-Headers",
        ] {
            h.append(header::VARY, HeaderValue::from_static(v));
        }
        adicionar_vary(&mut h);
        let todos: Vec<_> = h.get_all(header::VARY).iter().collect();
        assert_eq!(todos.len(), 1);
        assert_eq!(
            todos[0],
            "origin,access-control-request-method,access-control-request-headers,accept-encoding"
        );
    }

    #[test]
    fn pagina_404_com_tamanho_do_tomcat() {
        assert_eq!(pagina_404().len(), 431);
    }
}
