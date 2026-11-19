//! facet `pattern`: a sintaxe de expressão regular do XML Schema traduzida
//! para o crate `regex`, sempre ancorada nas duas pontas

use regex::Regex;

const ESPACO_XSD: &str = r"\x20\t\n\r";
const NAO_PALAVRA: &str = r"\p{P}\p{Z}\p{C}";

#[derive(Debug)]
pub struct Padrao {
    /// texto como escrito no XSD, que aparece nas mensagens
    pub texto: String,
    regex: Regex,
}

impl Padrao {
    pub fn novo(texto: &str) -> Result<Self, String> {
        let traduzido = traduzir(texto)?;
        let regex = Regex::new(&format!(r"\A(?:{traduzido})\z"))
            .map_err(|e| format!("pattern '{texto}' inválido: {e}"))?;
        Ok(Self {
            texto: texto.to_string(),
            regex,
        })
    }

    pub fn casa(&self, valor: &str) -> bool {
        self.regex.is_match(valor)
    }
}

fn traduzir(padrao: &str) -> Result<String, String> {
    let cs: Vec<char> = padrao.chars().collect();
    let mut saida = String::new();
    let mut classes = 0usize;
    let mut k = 0;
    while k < cs.len() {
        let c = cs[k];
        k += 1;
        if c == '\\' {
            let Some(&e) = cs.get(k) else {
                return Err(format!("pattern '{padrao}' termina com '\\'"));
            };
            k += 1;
            saida.push_str(&escape(e, classes > 0, padrao)?);
            continue;
        }
        if classes == 0 {
            match c {
                '.' => saida.push_str(r"[^\n\r]"),
                '^' | '$' => {
                    saida.push('\\');
                    saida.push(c);
                }
                '[' => {
                    classes += 1;
                    saida.push('[');
                    if cs.get(k) == Some(&'^') {
                        saida.push('^');
                        k += 1;
                    }
                }
                _ => saida.push(c),
            }
            continue;
        }
        match c {
            // subtração de classe: `[a-z-[aeiou]]`
            '-' if cs.get(k) == Some(&'[') => {
                saida.push_str("--[");
                classes += 1;
                k += 1;
                if cs.get(k) == Some(&'^') {
                    saida.push('^');
                    k += 1;
                }
            }
            ']' => {
                classes -= 1;
                saida.push(']');
            }
            '[' => return Err(format!("pattern '{padrao}' com '[' sem escape")),
            // operadores de classe do crate regex que são literais no XSD
            '&' | '~' => {
                saida.push('\\');
                saida.push(c);
            }
            _ => saida.push(c),
        }
    }
    Ok(saida)
}

fn escape(e: char, em_classe: bool, padrao: &str) -> Result<String, String> {
    Ok(match (e, em_classe) {
        ('s', false) => format!("[{ESPACO_XSD}]"),
        ('s', true) => ESPACO_XSD.to_string(),
        ('S', _) => format!("[^{ESPACO_XSD}]"),
        ('w', _) => format!("[^{NAO_PALAVRA}]"),
        ('W', _) => format!("[{NAO_PALAVRA}]"),
        ('d' | 'D' | 'n' | 'r' | 't', _) => format!("\\{e}"),
        ('i' | 'I' | 'c' | 'C' | 'p' | 'P', _) => {
            return Err(format!(
                "escape '\\{e}' não suportado no pattern '{padrao}'"
            ));
        }
        _ => format!("\\{e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn casa(p: &str, v: &str) -> bool {
        Padrao::novo(p).map(|x| x.casa(v)).unwrap_or(false)
    }

    #[test]
    fn padroes_dos_xsds() {
        assert!(casa(r"\d{3}", "200"));
        assert!(!casa(r"\d{3}", "20"));
        assert!(!casa(r"\d{3}", "2000"));
        let dec = r"0|0\.[0-9]{2}|[1-9]{1}[0-9]{0,12}(\.[0-9]{2})?";
        assert!(casa(dec, "100.00"));
        assert!(!casa(dec, "100.0"));
        assert!(!casa(dec, " 100.00 "));
        let texto = "[!-ÿ]{1}[ -ÿ]{0,}[!-ÿ]{1}|[!-ÿ]{1}";
        assert!(casa(texto, "VN"));
        assert!(!casa(texto, " A"));
        assert!(!casa(texto, ""));
        assert!(casa(r"[\s\S!-ÿ]{1}", "\n"));
        assert!(casa("[^@]+@[^\\.]+\\..+", "a@b.c"));
        assert!(!casa("a.b", "a\nb"));
        assert!(casa("[A-Za-z0-9.:+-/)(]{5}", "a+/()"));
    }
}
