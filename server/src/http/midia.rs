//! media types com as regras do `MimeType`/`MediaType` do Spring: tipo e
//! subtipo sem caixa, parâmetros preservados e compatibilidade por curinga
//! e sufixo estruturado (`*+json`)

pub const JSON: &str = "application/json";
pub const XML: &str = "application/xml";
pub const XML_UTF8: &str = "application/xml;charset=UTF-8";
pub const PROBLEMA_JSON: &str = "application/problem+json";
pub const OCTET_STREAM: &str = "application/octet-stream";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Midia {
    pub tipo: String,
    pub subtipo: String,
    pub parametros: Vec<(String, String)>,
}

fn token_valido(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_graphic() && !b"()<>@,;:\\\"/[]?={}".contains(&b))
}

fn sem_aspas(v: &str) -> &str {
    v.strip_prefix('"')
        .and_then(|s| s.strip_suffix('"'))
        .unwrap_or(v)
}

impl Midia {
    pub fn parse(s: &str) -> Option<Self> {
        let mut partes = s.split(';');
        let essencial = partes.next()?.trim();
        let essencial = if essencial == "*" { "*/*" } else { essencial };
        let (tipo, subtipo) = essencial.split_once('/')?;
        if !token_valido(tipo) || !token_valido(subtipo) {
            return None;
        }
        let mut parametros = Vec::new();
        for p in partes {
            let p = p.trim();
            if p.is_empty() {
                continue;
            }
            let (k, v) = p.split_once('=')?;
            let (k, v) = (k.trim(), v.trim());
            if !token_valido(k) || v.is_empty() {
                return None;
            }
            parametros.push((k.to_string(), v.to_string()));
        }
        Some(Self {
            tipo: tipo.to_ascii_lowercase(),
            subtipo: subtipo.to_ascii_lowercase(),
            parametros,
        })
    }

    pub fn parametro(&self, nome: &str) -> Option<&str> {
        self.parametros
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(nome))
            .map(|(_, v)| sem_aspas(v))
    }

    pub fn charset(&self) -> Option<&str> {
        self.parametro("charset")
    }

    fn sufixo(&self) -> Option<&str> {
        self.subtipo.rsplit_once('+').map(|(_, s)| s)
    }

    fn subtipo_curinga(&self) -> bool {
        self.subtipo == "*" || self.subtipo.starts_with("*+")
    }

    pub fn compativel(&self, outro: &Self) -> bool {
        if self.tipo == "*" || outro.tipo == "*" {
            return true;
        }
        if self.tipo != outro.tipo {
            return false;
        }
        if self.subtipo == outro.subtipo {
            return true;
        }
        if !self.subtipo_curinga() && !outro.subtipo_curinga() {
            return false;
        }
        if self.subtipo == "*" || outro.subtipo == "*" {
            return true;
        }
        let casa = |curinga: &Self, outro: &Self| {
            curinga.subtipo_curinga()
                && curinga
                    .sufixo()
                    .is_some_and(|s| Some(s) == outro.sufixo() || s == outro.subtipo)
        };
        casa(self, outro) || casa(outro, self)
    }

    /// `MimeType.includes` no sentido "este (do mapeamento) inclui o pedido"
    pub fn inclui(&self, outro: &Self) -> bool {
        if self.tipo == "*" {
            return true;
        }
        if self.tipo != outro.tipo {
            return false;
        }
        if self.subtipo == outro.subtipo || self.subtipo == "*" {
            return true;
        }
        self.subtipo.starts_with("*+")
            && self
                .sufixo()
                .is_some_and(|s| Some(s) == outro.sufixo() || s == outro.subtipo)
    }

    pub fn json() -> Self {
        Self {
            tipo: "application".into(),
            subtipo: "json".into(),
            parametros: Vec::new(),
        }
    }
}

impl std::fmt::Display for Midia {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.tipo, self.subtipo)?;
        for (k, v) in &self.parametros {
            write!(f, ";{k}={v}")?;
        }
        Ok(())
    }
}

/// negociação com `Accept`: `None` quando o cabeçalho não é parseável;
/// ausente ou vazio aceita tudo
pub fn aceita(accept: Option<&str>, produzido: &Midia) -> Option<bool> {
    let Some(accept) = accept.filter(|a| !a.trim().is_empty()) else {
        return Some(true);
    };
    let mut algum = false;
    for parte in accept.split(',').map(str::trim).filter(|p| !p.is_empty()) {
        algum |= Midia::parse(parte)?.compativel(produzido);
    }
    Some(algum)
}

// charsets que o Charset.forName da JVM resolve e que aparecem na prática
const CHARSETS: [&str; 16] = [
    "utf-8",
    "utf8",
    "utf-16",
    "utf-16be",
    "utf-16le",
    "utf-32",
    "utf-32be",
    "utf-32le",
    "iso-8859-1",
    "latin1",
    "l1",
    "iso8859_1",
    "iso_8859_1",
    "cp819",
    "us-ascii",
    "ascii",
];

pub fn charset_conhecido(cs: &str) -> bool {
    CHARSETS.contains(&cs.to_ascii_lowercase().as_str())
        || cs.to_ascii_lowercase().starts_with("windows-125")
}

fn latin1(corpo: &[u8]) -> String {
    corpo.iter().map(|&b| char::from(b)).collect()
}

fn utf32(corpo: &[u8], big_endian: bool) -> String {
    corpo
        .chunks(4)
        .map(|c| match <[u8; 4]>::try_from(c) {
            Ok(b) if big_endian => u32::from_be_bytes(b),
            Ok(b) => u32::from_le_bytes(b),
            Err(_) => u32::MAX,
        })
        .map(|u| char::from_u32(u).unwrap_or(char::REPLACEMENT_CHARACTER))
        .collect()
}

/// UTF-16/UTF-32 sem ordem explícita: BOM decide e é consumido; sem BOM, big-endian
fn com_bom(corpo: &[u8], largura: usize) -> String {
    let (le, be): (&[u8], &[u8]) = if largura == 2 {
        (&[0xFF, 0xFE], &[0xFE, 0xFF])
    } else {
        (&[0xFF, 0xFE, 0, 0], &[0, 0, 0xFE, 0xFF])
    };
    let (resto, big_endian) = match (corpo.strip_prefix(le), corpo.strip_prefix(be)) {
        (Some(r), _) => (r, false),
        (None, Some(r)) => (r, true),
        (None, None) => (corpo, true),
    };
    match (largura, big_endian) {
        (2, true) => encoding_rs::UTF_16BE
            .decode_without_bom_handling(resto)
            .0
            .into_owned(),
        (2, false) => encoding_rs::UTF_16LE
            .decode_without_bom_handling(resto)
            .0
            .into_owned(),
        (_, b) => utf32(resto, b),
    }
}

/// corpo lido como `String` pelo `StringHttpMessageConverter`: charset do
/// Content-Type ou UTF-8, com bytes inválidos trocados por U+FFFD
pub fn texto(corpo: &[u8], charset: Option<&str>) -> String {
    let cs = charset.map_or_else(|| "utf-8".to_string(), str::to_ascii_lowercase);
    match cs.as_str() {
        "iso-8859-1" | "latin1" | "l1" | "iso8859_1" | "iso_8859_1" | "cp819" => latin1(corpo),
        "us-ascii" | "ascii" => corpo
            .iter()
            .map(|&b| {
                if b.is_ascii() {
                    char::from(b)
                } else {
                    char::REPLACEMENT_CHARACTER
                }
            })
            .collect(),
        "utf-16" => com_bom(corpo, 2),
        "utf-32" => com_bom(corpo, 4),
        "utf-16be" => encoding_rs::UTF_16BE
            .decode_without_bom_handling(corpo)
            .0
            .into_owned(),
        "utf-16le" => encoding_rs::UTF_16LE
            .decode_without_bom_handling(corpo)
            .0
            .into_owned(),
        "utf-32be" => utf32(corpo, true),
        "utf-32le" => utf32(corpo, false),
        outro => match encoding_rs::Encoding::for_label(outro.as_bytes()) {
            Some(e) if outro.starts_with("windows-125") => {
                e.decode_without_bom_handling(corpo).0.into_owned()
            }
            _ => String::from_utf8_lossy(corpo).into_owned(),
        },
    }
}

/// decodifica o corpo como o `AbstractJackson2HttpMessageConverter`: charsets
/// fora da família UTF passam por um `Reader` antes do parser
pub fn decodificar(corpo: &[u8], charset: Option<&str>) -> Vec<u8> {
    let Some(cs) = charset.map(str::to_ascii_lowercase) else {
        return corpo.to_vec();
    };
    match cs.as_str() {
        "iso-8859-1" | "latin1" | "l1" | "iso8859_1" | "iso_8859_1" | "cp819" => corpo
            .iter()
            .map(|&b| char::from(b))
            .collect::<String>()
            .into_bytes(),
        "us-ascii" | "ascii" => corpo
            .iter()
            .map(|&b| {
                if b.is_ascii() {
                    char::from(b)
                } else {
                    '\u{FFFD}'
                }
            })
            .collect::<String>()
            .into_bytes(),
        _ => corpo.to_vec(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(s: &str) -> Midia {
        match Midia::parse(s) {
            Some(m) => m,
            None => panic!("media type inválido: {s}"),
        }
    }

    #[test]
    fn compatibilidade_como_spring() {
        let json = Midia::json();
        assert!(m("*/*").compativel(&json));
        assert!(m("application/*").compativel(&json));
        assert!(m("application/*+json").compativel(&json));
        assert!(!m("application/problem+json").compativel(&json));
        assert!(!m("text/html").compativel(&json));
        assert_eq!(
            aceita(Some("text/plain, application/json;q=0.1"), &json),
            Some(true)
        );
    }

    #[test]
    fn inclusao_para_consumes() {
        let json = Midia::json();
        assert!(json.inclui(&m("APPLICATION/JSON")));
        assert!(json.inclui(&m("application/json; charset=ISO-8859-1")));
        assert!(!json.inclui(&m("application/*+json")));
        assert!(!json.inclui(&m("application/merge-patch+json")));
    }

    #[test]
    fn exibicao_normalizada() {
        assert_eq!(
            m("Text/Plain; charset=utf-8").to_string(),
            "text/plain;charset=utf-8"
        );
    }
}
