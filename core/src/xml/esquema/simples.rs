//! tipos simples: validação léxica dos built-ins e das facetas, na ordem e com
//! as mensagens do `XSSimpleTypeDecl` do Xerces

use std::cmp::Ordering;
use std::collections::HashSet;
use std::sync::Arc;

use super::padrao::Padrao;
use crate::xml::caracteres::eh_ncname;

const BITS_POR_SEXTETO: usize = 6;
const BITS_POR_OCTETO: usize = 8;
const MAX_MES: u32 = 12;
const MAX_HORA: u32 = 23;
const MAX_MINUTO: u32 = 59;
const MAX_SEGUNDO: u32 = 59;
const MAX_HORA_FUSO: u32 = 14;
const HORA_FIM_DO_DIA: u32 = 24;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Espacos {
    Preservar,
    Substituir,
    Colapsar,
}

impl Espacos {
    pub fn normalizar(self, v: &str) -> String {
        match self {
            Self::Preservar => v.to_string(),
            Self::Substituir => v.replace(['\t', '\n', '\r'], " "),
            Self::Colapsar => v
                .split([' ', '\t', '\n', '\r'])
                .filter(|p| !p.is_empty())
                .collect::<Vec<_>>()
                .join(" "),
        }
    }
}

/// validador léxico do built-in primitivo (o `fDVs[fValidationDV]` do Xerces)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Primitivo {
    Texto,
    Uri,
    Decimal,
    Inteiro,
    Booleano,
    Data,
    DataHora,
    Hora,
    AnoMes,
    Ano,
    Base64,
    Hex,
    Duplo,
}

impl Primitivo {
    fn nome(self) -> &'static str {
        match self {
            Self::Texto => "string",
            Self::Uri => "anyURI",
            Self::Decimal => "decimal",
            Self::Inteiro => "integer",
            Self::Booleano => "boolean",
            Self::Data => "date",
            Self::DataHora => "dateTime",
            Self::Hora => "time",
            Self::AnoMes => "gYearMonth",
            Self::Ano => "gYear",
            Self::Base64 => "base64Binary",
            Self::Hex => "hexBinary",
            Self::Duplo => "double",
        }
    }

    fn lexico_valido(self, v: &str) -> bool {
        match self {
            Self::Texto | Self::Uri => true,
            Self::Decimal => decimal_valido(v),
            Self::Inteiro => inteiro_valido(v),
            Self::Booleano => matches!(v, "true" | "false" | "1" | "0"),
            Self::Data => data_valida(v),
            Self::DataHora => data_hora_valida(v),
            Self::Hora => hora_valida(v),
            Self::AnoMes => ano_mes_valido(v),
            Self::Ano => ano_valido(v),
            Self::Base64 => base64_octetos(v).is_some(),
            Self::Hex => v.len().is_multiple_of(2) && v.chars().all(|c| c.is_ascii_hexdigit()),
            Self::Duplo => duplo_valido(v),
        }
    }

    fn numerico(self) -> bool {
        matches!(self, Self::Decimal | Self::Inteiro)
    }
}

#[derive(Debug, Clone)]
pub struct TipoSimples {
    /// nome usado nas mensagens: o do XSD, `#AnonType_...` ou o do built-in
    pub nome: String,
    pub primitivo: Primitivo,
    pub espacos: Espacos,
    /// do tipo mais básico para o mais derivado
    pub padroes: Vec<Arc<Padrao>>,
    pub enumeracao: Option<Vec<String>>,
    pub tamanho: Option<usize>,
    pub tamanho_min: Option<usize>,
    pub tamanho_max: Option<usize>,
    pub max_inclusivo: Option<String>,
    pub max_exclusivo: Option<String>,
    pub min_inclusivo: Option<String>,
    pub min_exclusivo: Option<String>,
    pub total_digitos: Option<usize>,
    pub digitos_fracao: Option<usize>,
    /// padrão especial de NCName herdado (ID, IDREF, NCName)
    pub ncname: bool,
    pub id: bool,
    pub base: Option<usize>,
}

impl TipoSimples {
    fn builtin(nome: &str, primitivo: Primitivo, espacos: Espacos) -> Self {
        Self {
            nome: nome.to_string(),
            primitivo,
            espacos,
            padroes: Vec::new(),
            enumeracao: None,
            tamanho: None,
            tamanho_min: None,
            tamanho_max: None,
            max_inclusivo: None,
            max_exclusivo: None,
            min_inclusivo: None,
            min_exclusivo: None,
            total_digitos: None,
            digitos_fracao: None,
            ncname: false,
            id: false,
            base: None,
        }
    }

    /// valor canônico, usado em enumeração, `fixed` e restrições de identidade
    pub fn valor_real(&self, normalizado: &str) -> String {
        match self.primitivo {
            p if p.numerico() => canonico_decimal(normalizado),
            Primitivo::Booleano => match normalizado {
                "1" => "true".into(),
                "0" => "false".into(),
                v => v.to_string(),
            },
            _ => normalizado.to_string(),
        }
    }

    /// `exibido` é o texto que aparece nas mensagens de pattern
    pub fn validar(
        &self,
        exibido: &str,
        normalizado: &str,
        ids: &mut HashSet<String>,
    ) -> Result<(), String> {
        for p in self.padroes.iter().rev() {
            if !p.casa(normalizado) {
                return Err(format!(
                    "cvc-pattern-valid: Value '{exibido}' is not facet-valid with respect to pattern '{}' for type '{}'.",
                    p.texto, self.nome
                ));
            }
        }
        if self.ncname && !eh_ncname(normalizado) {
            return Err(format!(
                "cvc-datatype-valid.1.2.1: '{normalizado}' is not a valid value for 'NCName'."
            ));
        }
        if !self.primitivo.lexico_valido(normalizado) {
            return Err(format!(
                "cvc-datatype-valid.1.2.1: '{normalizado}' is not a valid value for '{}'.",
                self.primitivo.nome()
            ));
        }
        self.verificar_tamanho(normalizado)?;
        self.verificar_enumeracao(normalizado)?;
        self.verificar_digitos(normalizado)?;
        self.verificar_limites(normalizado)?;
        if self.id && !ids.insert(normalizado.to_string()) {
            return Err(format!(
                "cvc-id.2: There are multiple occurrences of ID value '{normalizado}'."
            ));
        }
        Ok(())
    }

    fn comprimento(&self, v: &str) -> usize {
        match self.primitivo {
            Primitivo::Base64 => base64_octetos(v).unwrap_or(0),
            Primitivo::Hex => v.len() / 2,
            _ => v.chars().count(),
        }
    }

    fn verificar_tamanho(&self, v: &str) -> Result<(), String> {
        let n = self.comprimento(v);
        let falha = |faceta: &str, limite: usize| {
            Err(format!(
                "cvc-{faceta}-valid: Value '{v}' with length = '{n}' is not facet-valid with respect to {faceta} '{limite}' for type '{}'.",
                self.nome
            ))
        };
        if let Some(m) = self.tamanho_max.filter(|m| n > *m) {
            return falha("maxLength", m);
        }
        if let Some(m) = self.tamanho_min.filter(|m| n < *m) {
            return falha("minLength", m);
        }
        if let Some(m) = self.tamanho.filter(|m| n != *m) {
            return falha("length", m);
        }
        Ok(())
    }

    fn verificar_enumeracao(&self, v: &str) -> Result<(), String> {
        let Some(valores) = &self.enumeracao else {
            return Ok(());
        };
        let real = self.valor_real(v);
        if valores.iter().any(|e| self.valor_real(e) == real) {
            return Ok(());
        }
        let lista: Vec<String> = valores.iter().map(|e| self.valor_real(e)).collect();
        Err(format!(
            "cvc-enumeration-valid: Value '{v}' is not facet-valid with respect to enumeration '[{}]'. It must be a value from the enumeration.",
            lista.join(", ")
        ))
    }

    fn verificar_digitos(&self, v: &str) -> Result<(), String> {
        if !self.primitivo.numerico() {
            return Ok(());
        }
        let (inteiros, fracao) = digitos(v);
        if let Some(m) = self.digitos_fracao.filter(|m| fracao > *m) {
            return Err(format!(
                "cvc-fractionDigits-valid: Value '{v}' has {fracao} fraction digits, but the number of fraction digits has been limited to {m}."
            ));
        }
        let total = inteiros + fracao;
        if let Some(m) = self.total_digitos.filter(|m| total > *m) {
            return Err(format!(
                "cvc-totalDigits-valid: Value '{v}' has {total} total digits, but the number of total digits has been limited to {m}."
            ));
        }
        Ok(())
    }

    fn verificar_limites(&self, v: &str) -> Result<(), String> {
        let limites = [
            (
                "maxInclusive",
                &self.max_inclusivo,
                &[Ordering::Less, Ordering::Equal][..],
            ),
            ("maxExclusive", &self.max_exclusivo, &[Ordering::Less][..]),
            (
                "minInclusive",
                &self.min_inclusivo,
                &[Ordering::Greater, Ordering::Equal][..],
            ),
            (
                "minExclusive",
                &self.min_exclusivo,
                &[Ordering::Greater][..],
            ),
        ];
        for (faceta, limite, aceitos) in limites {
            let Some(l) = limite else { continue };
            if !aceitos.contains(&self.comparar(v, l)) {
                return Err(format!(
                    "cvc-{faceta}-valid: Value '{v}' is not facet-valid with respect to {faceta} '{l}' for type '{}'.",
                    self.nome
                ));
            }
        }
        Ok(())
    }

    fn comparar(&self, a: &str, b: &str) -> Ordering {
        if self.primitivo.numerico() {
            comparar_decimais(a, b)
        } else {
            a.cmp(b)
        }
    }
}

/// built-ins do namespace XML Schema que os XSDs (e um `xsi:type`) podem usar
pub fn builtins() -> Vec<TipoSimples> {
    use Espacos::{Colapsar, Preservar, Substituir};
    use Primitivo::*;
    let simples = [
        ("anySimpleType", Texto, Preservar),
        ("string", Texto, Preservar),
        ("normalizedString", Texto, Substituir),
        ("token", Texto, Colapsar),
        ("language", Texto, Colapsar),
        ("Name", Texto, Colapsar),
        ("NMTOKEN", Texto, Colapsar),
        ("QName", Texto, Colapsar),
        ("anyURI", Uri, Colapsar),
        ("base64Binary", Base64, Colapsar),
        ("hexBinary", Hex, Colapsar),
        ("boolean", Booleano, Colapsar),
        ("decimal", Decimal, Colapsar),
        ("integer", Inteiro, Colapsar),
        ("double", Duplo, Colapsar),
        ("float", Duplo, Colapsar),
        ("date", Data, Colapsar),
        ("dateTime", DataHora, Colapsar),
        ("time", Hora, Colapsar),
        ("gYearMonth", AnoMes, Colapsar),
        ("gYear", Ano, Colapsar),
    ];
    let mut tipos: Vec<TipoSimples> = simples
        .into_iter()
        .map(|(n, p, e)| TipoSimples::builtin(n, p, e))
        .collect();
    for (nome, id) in [
        ("NCName", false),
        ("ID", true),
        ("IDREF", false),
        ("ENTITY", false),
    ] {
        let mut t = TipoSimples::builtin(nome, Texto, Colapsar);
        t.ncname = true;
        t.id = id;
        tipos.push(t);
    }
    let inteiros: [(&str, Option<&str>, Option<&str>); 12] = [
        ("nonNegativeInteger", Some("0"), None),
        ("positiveInteger", Some("1"), None),
        ("nonPositiveInteger", None, Some("0")),
        ("negativeInteger", None, Some("-1")),
        (
            "long",
            Some("-9223372036854775808"),
            Some("9223372036854775807"),
        ),
        ("int", Some("-2147483648"), Some("2147483647")),
        ("short", Some("-32768"), Some("32767")),
        ("byte", Some("-128"), Some("127")),
        ("unsignedLong", Some("0"), Some("18446744073709551615")),
        ("unsignedInt", Some("0"), Some("4294967295")),
        ("unsignedShort", Some("0"), Some("65535")),
        ("unsignedByte", Some("0"), Some("255")),
    ];
    for (nome, min, max) in inteiros {
        let mut t = TipoSimples::builtin(nome, Inteiro, Colapsar);
        t.min_inclusivo = min.map(str::to_string);
        t.max_inclusivo = max.map(str::to_string);
        tipos.push(t);
    }
    tipos
}

fn sem_sinal(v: &str) -> (bool, &str) {
    match v.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, v.strip_prefix('+').unwrap_or(v)),
    }
}

fn so_digitos(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn inteiro_valido(v: &str) -> bool {
    so_digitos(sem_sinal(v).1)
}

fn decimal_valido(v: &str) -> bool {
    let (_, r) = sem_sinal(v);
    match r.split_once('.') {
        Some((i, f)) => {
            (i.is_empty() || so_digitos(i))
                && (f.is_empty() || so_digitos(f))
                && !(i.is_empty() && f.is_empty())
        }
        None => so_digitos(r),
    }
}

fn duplo_valido(v: &str) -> bool {
    matches!(v, "INF" | "-INF" | "NaN")
        || v.parse::<f64>().is_ok() && !v.contains(['i', 'I', 'n', 'N'])
}

/// dígitos significativos antes e depois da vírgula (para `totalDigits`)
fn digitos(v: &str) -> (usize, usize) {
    let (_, r) = sem_sinal(v);
    let (i, f) = r.split_once('.').unwrap_or((r, ""));
    let i = i.trim_start_matches('0');
    let f = f.trim_end_matches('0');
    (i.len(), f.len())
}

fn canonico_decimal(v: &str) -> String {
    let (negativo, r) = sem_sinal(v);
    let (i, f) = r.split_once('.').unwrap_or((r, ""));
    let i = i.trim_start_matches('0');
    let f = f.trim_end_matches('0');
    let i = if i.is_empty() { "0" } else { i };
    let corpo = if f.is_empty() {
        i.to_string()
    } else {
        format!("{i}.{f}")
    };
    if negativo && corpo != "0" {
        format!("-{corpo}")
    } else {
        corpo
    }
}

fn comparar_decimais(a: &str, b: &str) -> Ordering {
    let (ca, cb) = (canonico_decimal(a), canonico_decimal(b));
    let ((na, ra), (nb, rb)) = (sem_sinal(&ca), sem_sinal(&cb));
    match (na, nb) {
        (false, true) => Ordering::Greater,
        (true, false) => Ordering::Less,
        (false, false) => comparar_modulos(ra, rb),
        (true, true) => comparar_modulos(rb, ra),
    }
}

fn comparar_modulos(a: &str, b: &str) -> Ordering {
    let (ia, fa) = a.split_once('.').unwrap_or((a, ""));
    let (ib, fb) = b.split_once('.').unwrap_or((b, ""));
    ia.len()
        .cmp(&ib.len())
        .then_with(|| ia.cmp(ib))
        .then_with(|| fa.cmp(fb))
}

fn numero(s: &str) -> Option<u32> {
    if so_digitos(s) { s.parse().ok() } else { None }
}

/// `-?YYYY` com pelo menos 4 dígitos e sem ano zero; devolve o ano e o resto
fn ano(v: &str) -> Option<(i64, &str)> {
    let (negativo, r) = match v.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, v),
    };
    let fim = r.find(|c: char| !c.is_ascii_digit()).unwrap_or(r.len());
    let digitos = &r[..fim];
    const MIN_DIGITOS_ANO: usize = 4;
    if digitos.len() < MIN_DIGITOS_ANO
        || (digitos.len() > MIN_DIGITOS_ANO && digitos.starts_with('0'))
    {
        return None;
    }
    let a: i64 = digitos.parse().ok()?;
    if a == 0 {
        return None;
    }
    Some((if negativo { -a } else { a }, &r[fim..]))
}

fn dois_digitos(s: &str) -> Option<(u32, &str)> {
    let cabeca = s.get(..2)?;
    Some((numero(cabeca)?, &s[2..]))
}

fn fuso_valido(s: &str) -> bool {
    if s.is_empty() || s == "Z" {
        return true;
    }
    let Some(r) = s.strip_prefix(['+', '-']) else {
        return false;
    };
    let Some((h, r)) = dois_digitos(r) else {
        return false;
    };
    let Some(r) = r.strip_prefix(':') else {
        return false;
    };
    let Some((m, r)) = dois_digitos(r) else {
        return false;
    };
    r.is_empty() && m <= MAX_MINUTO && (h < MAX_HORA_FUSO || (h == MAX_HORA_FUSO && m == 0))
}

fn bissexto(a: i64) -> bool {
    (a % 4 == 0 && a % 100 != 0) || a % 400 == 0
}

fn dias_no_mes(a: i64, m: u32) -> u32 {
    match m {
        2 if bissexto(a) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// `YYYY-MM-DD`; devolve o resto (hora/fuso)
fn data(v: &str) -> Option<&str> {
    let (a, r) = ano(v)?;
    let (m, r) = dois_digitos(r.strip_prefix('-')?)?;
    let (d, r) = dois_digitos(r.strip_prefix('-')?)?;
    ((1..=MAX_MES).contains(&m) && d >= 1 && d <= dias_no_mes(a, m)).then_some(r)
}

/// `hh:mm:ss(.s+)?` seguido de fuso válido
fn hora_valida(v: &str) -> bool {
    hora(v).unwrap_or(false)
}

fn hora(v: &str) -> Option<bool> {
    let (h, r) = dois_digitos(v)?;
    let (m, r) = dois_digitos(r.strip_prefix(':')?)?;
    let (s, mut r) = dois_digitos(r.strip_prefix(':')?)?;
    let mut fracao_zero = true;
    if let Some(f) = r.strip_prefix('.') {
        let fim = f.find(|c: char| !c.is_ascii_digit()).unwrap_or(f.len());
        if fim == 0 {
            return None;
        }
        fracao_zero = f[..fim].bytes().all(|b| b == b'0');
        r = &f[fim..];
    }
    let fim_do_dia = h == HORA_FIM_DO_DIA && m == 0 && s == 0 && fracao_zero;
    let valida = (h <= MAX_HORA || fim_do_dia) && m <= MAX_MINUTO && s <= MAX_SEGUNDO;
    Some(valida && fuso_valido(r))
}

fn data_valida(v: &str) -> bool {
    data(v).is_some_and(fuso_valido)
}

fn data_hora_valida(v: &str) -> bool {
    data(v)
        .and_then(|r| r.strip_prefix('T'))
        .is_some_and(hora_valida)
}

fn ano_mes_valido(v: &str) -> bool {
    let Some((_, r)) = ano(v) else { return false };
    let Some((m, r)) = r.strip_prefix('-').and_then(dois_digitos) else {
        return false;
    };
    (1..=MAX_MES).contains(&m) && fuso_valido(r)
}

fn ano_valido(v: &str) -> bool {
    ano(v).is_some_and(|(_, r)| fuso_valido(r))
}

/// quantidade de octetos de um base64 válido
fn base64_octetos(v: &str) -> Option<usize> {
    let limpo: Vec<u8> = v.bytes().filter(|b| !b" \t\n\r".contains(b)).collect();
    const TAMANHO_QUARTETO: usize = 4;
    if !limpo.len().is_multiple_of(TAMANHO_QUARTETO) {
        return None;
    }
    let preenchimento = limpo.iter().rev().take_while(|b| **b == b'=').count();
    if preenchimento > 2 {
        return None;
    }
    let dados = &limpo[..limpo.len() - preenchimento];
    if !dados
        .iter()
        .all(|b| b.is_ascii_alphanumeric() || *b == b'+' || *b == b'/')
    {
        return None;
    }
    Some(dados.len() * BITS_POR_SEXTETO / BITS_POR_OCTETO)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tipo(nome: &str) -> TipoSimples {
        builtins()
            .into_iter()
            .find(|t| t.nome == nome)
            .unwrap_or_else(|| TipoSimples::builtin(nome, Primitivo::Texto, Espacos::Preservar))
    }

    #[test]
    fn lexicos() {
        assert!(data_valida("2024-02-29"));
        assert!(!data_valida("2023-02-29"));
        assert!(data_hora_valida("2024-01-01T10:00:00-03:00"));
        assert!(!data_hora_valida("2024-01-01T25:00:00"));
        assert!(ano_mes_valido("2024-12"));
        assert!(!ano_mes_valido("2024-13"));
        assert!(decimal_valido("-1.5") && decimal_valido(".5") && !decimal_valido("."));
        assert_eq!(base64_octetos("QUJD"), Some(3));
        assert_eq!(base64_octetos("QUI="), Some(2));
        assert_eq!(base64_octetos("QU=I"), None);
    }

    #[test]
    fn facetas_e_mensagens() {
        let mut ids = HashSet::new();
        let mut t = tipo("string");
        t.nome = "T".into();
        t.tamanho_max = Some(2);
        assert_eq!(
            t.validar("abc", "abc", &mut ids),
            Err("cvc-maxLength-valid: Value 'abc' with length = '3' is not facet-valid with respect to maxLength '2' for type 'T'.".into())
        );
        let mut e = tipo("integer");
        e.enumeracao = Some(vec!["1".into(), "02".into()]);
        assert_eq!(e.validar("2", "2", &mut ids), Ok(()));
        assert_eq!(
            e.validar("3", "3", &mut ids),
            Err("cvc-enumeration-valid: Value '3' is not facet-valid with respect to enumeration '[1, 2]'. It must be a value from the enumeration.".into())
        );
        let id = tipo("ID");
        assert_eq!(id.validar("a", "a", &mut ids), Ok(()));
        assert_eq!(
            id.validar("a", "a", &mut ids),
            Err("cvc-id.2: There are multiple occurrences of ID value 'a'.".into())
        );
        assert_eq!(
            id.validar("1a", "1a", &mut ids),
            Err("cvc-datatype-valid.1.2.1: '1a' is not a valid value for 'NCName'.".into())
        );
        assert_eq!(
            tipo("positiveInteger").validar("0", "0", &mut ids),
            Err("cvc-minInclusive-valid: Value '0' is not facet-valid with respect to minInclusive '1' for type 'positiveInteger'.".into())
        );
        assert_eq!(comparar_decimais("-2", "1"), Ordering::Less);
        assert_eq!(comparar_decimais("10.5", "9.75"), Ordering::Greater);
    }
}
