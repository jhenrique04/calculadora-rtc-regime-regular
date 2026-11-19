//! requisição já roteada, com os resolvedores de argumento do Spring MVC
//! (`@RequestBody`, `@RequestParam`, `@PathVariable`)

use chrono::NaiveDate;
use rtc_core::json::{LerJson, ler_documento};

use super::erro::ErroHttp;

pub struct Pedido<'a> {
    /// `scheme://host`, sem porta (`HttpUtils.getBaseURL`)
    pub base_url: &'a str,
    pub cabecalhos: &'a hyper::HeaderMap,
    pub consulta: &'a [(String, String)],
    pub variaveis: super::rotas::Variaveis,
    /// corpo já convertido para UTF-8 conforme o charset do `Content-Type`
    pub corpo: &'a [u8],
}

impl Pedido<'_> {
    /// `@RequestBody` obrigatório: vazio ou `null` vira "Required request body is missing"
    pub fn corpo_json<T: LerJson>(&self, assinatura: &'static str) -> Result<T, ErroHttp> {
        if self.corpo.is_empty() {
            return Err(ErroHttp::CorpoAusente(assinatura));
        }
        match ler_documento::<T>(self.corpo) {
            Ok(Some(v)) => Ok(v),
            Ok(None) => Err(ErroHttp::CorpoAusente(assinatura)),
            Err(e) => Err(ErroHttp::Json(e)),
        }
    }

    /// valores repetidos chegam unidos por vírgula, como na conversão para `String`
    pub fn parametro(&self, nome: &str) -> Option<String> {
        let valores: Vec<&str> = self
            .consulta
            .iter()
            .filter(|(k, _)| k == nome)
            .map(|(_, v)| v.as_str())
            .collect();
        (!valores.is_empty()).then(|| valores.join(","))
    }

    pub fn obrigatorio(&self, nome: &'static str) -> Result<String, ErroHttp> {
        self.parametro(nome).ok_or(ErroHttp::ParametroAusente(nome))
    }

    pub fn variavel(&self, nome: &str) -> Option<&str> {
        self.variaveis
            .iter()
            .find(|(k, _)| *k == nome)
            .map(|(_, v)| v.as_str())
    }

    /// `@RequestParam LocalDate` com `@DateTimeFormat(iso = DATE)`; vazio vira ausente
    pub fn data(&self, nome: &'static str) -> Result<Option<NaiveDate>, ErroHttp> {
        let Some(v) = self.parametro(nome).filter(|v| !v.trim().is_empty()) else {
            return Ok(None);
        };
        NaiveDate::parse_from_str(v.trim(), "%Y-%m-%d")
            .map(Some)
            .map_err(|_| ErroHttp::TipoInvalido { nome, valor: v })
    }

    pub fn data_obrigatoria(&self, nome: &'static str) -> Result<NaiveDate, ErroHttp> {
        self.data(nome)?.ok_or(ErroHttp::ParametroAusente(nome))
    }

    /// `@RequestParam Long`: em branco vira ausente
    pub fn longo(&self, nome: &'static str) -> Result<Option<i64>, ErroHttp> {
        match self.parametro(nome).filter(|v| !v.trim().is_empty()) {
            None => Ok(None),
            Some(v) => converter_longo(nome, v).map(Some),
        }
    }

    /// `@RequestParam Integer`: estouro de `int` também é tipo inválido
    pub fn inteiro(&self, nome: &'static str) -> Result<Option<i32>, ErroHttp> {
        let Some(n) = self.longo(nome)? else {
            return Ok(None);
        };
        i32::try_from(n)
            .map(Some)
            .map_err(|_| ErroHttp::TipoInvalido {
                nome,
                valor: self.parametro(nome).unwrap_or_default(),
            })
    }

    pub fn longo_obrigatorio(&self, nome: &'static str) -> Result<i64, ErroHttp> {
        self.longo(nome)?.ok_or(ErroHttp::ParametroAusente(nome))
    }

    /// `@PathVariable`: o roteador só casa a rota com o segmento presente
    pub fn texto_variavel(&self, nome: &str) -> String {
        self.variavel(nome).unwrap_or_default().to_string()
    }

    pub fn longo_variavel(&self, nome: &'static str) -> Result<i64, ErroHttp> {
        converter_longo(nome, self.texto_variavel(nome))
    }

    /// `@RequestParam` de enum: em branco vira ausente, o resto passa por `Enum.valueOf(trim)`
    pub fn enumerado<E>(
        &self,
        nome: &'static str,
        por_nome: fn(&str) -> Option<E>,
    ) -> Result<Option<E>, ErroHttp> {
        match self.parametro(nome).filter(|v| !v.trim().is_empty()) {
            None => Ok(None),
            Some(v) => match por_nome(v.trim()) {
                Some(e) => Ok(Some(e)),
                None => Err(ErroHttp::TipoInvalido { nome, valor: v }),
            },
        }
    }
}

/// `NumberUtils.parseNumber(text, Long.class)`: tira todo espaço e aceita hexadecimal
fn converter_longo(nome: &'static str, valor: String) -> Result<i64, ErroHttp> {
    let limpo: String = valor.chars().filter(|c| !c.is_whitespace()).collect();
    match parse_longo(&limpo) {
        Some(n) => Ok(n),
        None => Err(ErroHttp::TipoInvalido { nome, valor }),
    }
}

const RADIX_HEX: u32 = 16;

fn parse_longo(s: &str) -> Option<i64> {
    let (negativo, resto) = match s.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, s),
    };
    let hex = resto
        .strip_prefix("0x")
        .or_else(|| resto.strip_prefix("0X"))
        .or_else(|| resto.strip_prefix('#'));
    match hex {
        // long.decode não aceita sinal depois do prefixo
        Some(h) if h.starts_with(['-', '+']) => None,
        Some(h) => {
            let texto = if negativo {
                format!("-{h}")
            } else {
                h.to_string()
            };
            i64::from_str_radix(&texto, RADIX_HEX).ok()
        }
        None => s.parse().ok(),
    }
}

/// `@Pattern("^\\d{n}$")` num `@RequestParam`: nulo passa
pub fn exigir_digitos(valor: Option<&str>, tamanho: usize) -> Result<(), ErroHttp> {
    match valor {
        Some(v) if v.len() != tamanho || !v.bytes().all(|b| b.is_ascii_digit()) => {
            Err(ErroHttp::ValidacaoMetodo)
        }
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::parse_longo;

    #[test]
    fn converte_long_como_spring() {
        assert_eq!(parse_longo("43"), Some(43));
        assert_eq!(parse_longo("+43"), Some(43));
        assert_eq!(parse_longo("-0x1F"), Some(-31));
        assert_eq!(parse_longo("#10"), Some(16));
        assert_eq!(parse_longo("0x-1"), None);
        assert_eq!(parse_longo("abc"), None);
        assert_eq!(parse_longo(""), None);
    }
}
