//! equivalentes das checagens de texto do Java (`StringUtils`, `String.trim`)

use chrono::NaiveDate;

/// `Character.isWhitespace`: espaços Unicode exceto os não separáveis
fn espaco_java(c: char) -> bool {
    match c {
        '\u{00A0}' | '\u{2007}' | '\u{202F}' => false,
        '\t' | '\n' | '\u{000B}' | '\u{000C}' | '\r' | '\u{001C}'..='\u{001F}' => true,
        c => c.is_whitespace() && c != '\u{0085}',
    }
}

pub fn branco(s: Option<&str>) -> bool {
    s.is_none_or(|s| s.chars().all(espaco_java))
}

pub fn preenchido(s: Option<&str>) -> bool {
    !branco(s)
}

/// `str == null || str.trim().isEmpty()` (o trim remove tudo `<= U+0020`)
pub fn vazio_apos_trim(s: Option<&str>) -> bool {
    s.is_none_or(|s| s.chars().all(|c| c <= ' '))
}

/// `StringUtils.length`: unidades UTF-16, zero para nulo
pub fn tamanho(s: Option<&str>) -> usize {
    s.map_or(0, |s| s.encode_utf16().count())
}

/// `String.valueOf` de uma referência possivelmente nula
pub fn exibir(s: Option<&str>) -> &str {
    s.unwrap_or("null")
}

pub fn data_java(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checagens_como_java() {
        assert!(branco(None));
        assert!(branco(Some(" \t")));
        assert!(!branco(Some("\u{00A0}")));
        assert!(vazio_apos_trim(Some("\u{0001} ")));
        assert!(!vazio_apos_trim(Some("\u{00A0}")));
        assert_eq!(tamanho(Some("😀")), 2);
        assert_eq!(tamanho(None), 0);
    }
}
