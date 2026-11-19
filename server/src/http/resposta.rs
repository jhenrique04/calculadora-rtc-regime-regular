//! resposta montada pelos handlers antes dos filtros (segurança, CORS,
//! compressão), e os corpos `application/problem+json` do Spring

use bytes::Bytes;
use chrono::{DateTime, Timelike, Utc};
use hyper::StatusCode;
use hyper::header::{HeaderName, HeaderValue};
use rtc_core::json::{ErroEscrita, Escrever, Escritor};

use super::midia::{JSON, PROBLEMA_JSON, XML_UTF8};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Corpo {
    /// `Content-Length: 0`, sem `Content-Type`
    Vazio,
    /// tamanho conhecido (`Content-Length`)
    Fixo(Bytes),
    /// escrito em streaming pelo Jackson (`Transfer-Encoding: chunked`)
    Fluxo(Bytes),
}

#[derive(Debug, Clone)]
pub struct Resposta {
    pub status: StatusCode,
    pub cabecalhos: Vec<(HeaderName, HeaderValue)>,
    pub tipo: Option<String>,
    pub corpo: Corpo,
    /// escrita por um `HttpMessageConverter` (sujeita à proteção contra RFD)
    pub conversor: bool,
}

impl Resposta {
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            cabecalhos: Vec::new(),
            tipo: None,
            corpo: Corpo::Vazio,
            conversor: false,
        }
    }

    pub fn json(status: StatusCode, corpo: String) -> Self {
        Self {
            status,
            cabecalhos: Vec::new(),
            tipo: Some(JSON.to_string()),
            corpo: Corpo::Fluxo(Bytes::from(corpo)),
            conversor: true,
        }
    }

    /// `String` escrita pelo `StringHttpMessageConverter`: tamanho conhecido
    pub fn xml(status: StatusCode, corpo: String) -> Self {
        Self {
            status,
            cabecalhos: Vec::new(),
            tipo: Some(XML_UTF8.to_string()),
            corpo: Corpo::Fixo(Bytes::from(corpo)),
            conversor: true,
        }
    }

    /// serializa um DTO como o `MappingJackson2HttpMessageConverter`
    pub fn serializar<T: Escrever>(status: StatusCode, v: &T) -> Result<Self, ErroEscrita> {
        let mut e = Escritor::new();
        v.escrever(&mut e)?;
        Ok(Self::json(status, e.finalizar()))
    }

    pub fn com(mut self, nome: &'static str, valor: &str) -> Self {
        if let Ok(v) = HeaderValue::from_str(valor) {
            self.cabecalhos.push((HeaderName::from_static(nome), v));
        }
        self
    }
}

/// `Instant.toString()`: fração em grupos de 3 dígitos, sem zeros finais de grupo
pub fn instante_java(t: DateTime<Utc>) -> String {
    let base = t.format("%Y-%m-%dT%H:%M:%S");
    let nanos = t.nanosecond() % 1_000_000_000;
    if nanos == 0 {
        format!("{base}Z")
    } else if nanos.is_multiple_of(1_000_000) {
        format!("{base}.{:03}Z", nanos / 1_000_000)
    } else if nanos.is_multiple_of(1_000) {
        format!("{base}.{:06}Z", nanos / 1_000)
    } else {
        format!("{base}.{nanos:09}Z")
    }
}

/// `ProblemDetail` serializado pelo Jackson: campos fixos e depois as propriedades
#[derive(Debug, Clone)]
pub struct Problema {
    pub tipo: String,
    pub titulo: String,
    pub status: StatusCode,
    pub detalhe: Option<String>,
    pub instancia: String,
    pub com_timestamp: bool,
    pub erros: Option<String>,
}

impl Problema {
    pub fn padrao(status: StatusCode, detalhe: impl Into<String>, instancia: &str) -> Self {
        Self {
            tipo: "about:blank".into(),
            titulo: status.canonical_reason().unwrap_or_default().to_string(),
            status,
            detalhe: Some(detalhe.into()),
            instancia: instancia.to_string(),
            com_timestamp: false,
            erros: None,
        }
    }

    pub fn corpo(&self, agora: DateTime<Utc>) -> String {
        let mut e = Escritor::new();
        e.inicio_objeto();
        e.nome("type");
        e.valor_string(&self.tipo);
        e.nome("title");
        e.valor_string(&self.titulo);
        e.nome("status");
        e.valor_bruto(self.status.as_u16());
        if let Some(d) = &self.detalhe {
            e.nome("detail");
            e.valor_string(d);
        }
        e.nome("instance");
        e.valor_string(&self.instancia);
        if self.com_timestamp {
            e.nome("timestamp");
            e.valor_string(&instante_java(agora));
        }
        if let Some(erros) = &self.erros {
            e.nome("errors");
            e.valor_bruto(erros);
        }
        e.fim_objeto();
        e.finalizar()
    }

    pub fn resposta(&self, agora: DateTime<Utc>) -> Resposta {
        Resposta {
            status: self.status,
            cabecalhos: Vec::new(),
            tipo: Some(PROBLEMA_JSON.to_string()),
            corpo: Corpo::Fluxo(Bytes::from(self.corpo(agora))),
            conversor: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn instante_como_java() {
        let t = |n| match Utc.timestamp_opt(1_700_000_000, n) {
            chrono::LocalResult::Single(t) => t,
            _ => panic!("data inválida"),
        };
        assert_eq!(instante_java(t(0)), "2023-11-14T22:13:20Z");
        assert_eq!(instante_java(t(120_000_000)), "2023-11-14T22:13:20.120Z");
        assert_eq!(instante_java(t(120_500_000)), "2023-11-14T22:13:20.120500Z");
        assert_eq!(
            instante_java(t(120_500_001)),
            "2023-11-14T22:13:20.120500001Z"
        );
    }

    #[test]
    fn problema_padrao_sem_timestamp() {
        let p = Problema::padrao(
            StatusCode::NOT_FOUND,
            "No static resource nada.",
            "/api/nada",
        );
        assert_eq!(
            p.corpo(Utc::now()),
            r#"{"type":"about:blank","title":"Not Found","status":404,"detail":"No static resource nada.","instance":"/api/nada"}"#
        );
    }
}
