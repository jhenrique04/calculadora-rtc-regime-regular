//! leitura/escrita JSON com as mesmas regras de coerção e mensagens do Jackson 2.x
//! (configuração padrão do Spring Boot)

mod escritor;
mod leitor;

use chrono::{FixedOffset, NaiveDate, NaiveDateTime};

pub use escritor::{ErroEscrita, Escrever, Escritor};
pub use leitor::{Leitor, Token};

use crate::decimal::BigDecimal;

const MAX_INT: i64 = i32::MAX as i64;
const MIN_INT: i64 = i32::MIN as i64;
const DIGITOS_INT_SEM_OVERFLOW: usize = 9;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{mensagem}")]
pub struct ErroJson {
    pub mensagem: String,
}

impl ErroJson {
    pub fn new(mensagem: impl Into<String>) -> Self {
        Self {
            mensagem: mensagem.into(),
        }
    }
}

pub type R<T> = Result<T, ErroJson>;

/// desserialização a partir do token corrente (já lido com `proximo`); consome o valor inteiro
pub trait LerJson: Sized {
    fn tipo_java() -> String;
    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>>;
}

/// `ObjectMapper.readValue`: `Ok(None)` quando o corpo é `null`
pub fn ler_documento<T: LerJson>(dados: &[u8]) -> R<Option<T>> {
    let mut l = Leitor::novo(dados);
    if l.proximo()?.is_none() {
        return Err(ErroJson::new("No content to map due to end-of-input"));
    }
    T::ler(&mut l)
}

/// `String.trim()` do Java: remove tudo `<= ' '`
pub fn trim_java(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

fn token(l: &Leitor<'_>) -> R<Token> {
    l.atual()
        .cloned()
        .ok_or_else(|| ErroJson::new("Unexpected end-of-input"))
}

fn forma(t: &Token) -> &'static str {
    match t {
        Token::InicioObjeto | Token::FimObjeto | Token::Campo(_) => "Object value",
        Token::InicioLista | Token::FimLista => "Array value",
        Token::Verdadeiro | Token::Falso => "Boolean value",
        Token::Decimal(_) => "Floating-point value",
        Token::Inteiro(_) => "Integer value",
        Token::Texto(_) => "String value",
        Token::Nulo => "Null value",
    }
}

pub fn token_inesperado<T>(tipo: &str, t: &Token) -> R<T> {
    Err(ErroJson::new(format!(
        "Cannot deserialize value of type `{tipo}` from {} (token `JsonToken.{}`)",
        forma(t),
        t.nome_jackson()
    )))
}

fn valor_estranho<T>(tipo: &str, valor: &str, motivo: &str) -> R<T> {
    Err(ErroJson::new(format!(
        "Cannot deserialize value of type `{tipo}` from String \"{valor}\": {motivo}"
    )))
}

fn sem_coercao<T>(tipo: &str, descricao: &str) -> R<T> {
    Err(ErroJson::new(format!(
        "Cannot coerce {descricao} to `{tipo}` value (but could if coercion was enabled using `CoercionConfig`)"
    )))
}

fn eh_branco(s: &str) -> bool {
    s.chars().all(|c| c <= ' ')
}

/// texto de string para escalares numéricos: vazio/branco/"null" viram `null`
fn texto_escalar(s: &str) -> Option<&str> {
    if s.is_empty() || eh_branco(s) {
        return None;
    }
    let t = trim_java(s);
    (t != "null").then_some(t)
}

/// `Integer.parseInt`/`Long.parseLong`: sinal opcional e só dígitos
fn parse_inteiro_java(s: &str) -> Option<i128> {
    let corpo = s.strip_prefix(['+', '-']).unwrap_or(s);
    if corpo.is_empty() || !corpo.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse::<i128>().ok()
}

fn fora_de_faixa<T>(texto: &str, tipo: &str, min: i64, max: i64) -> R<T> {
    Err(ErroJson::new(format!(
        "Numeric value ({texto}) out of range of {tipo} ({min} - {max})"
    )))
}

fn inteiro_de_numero(t: &Token, tipo: &str, min: i64, max: i64) -> R<i64> {
    match t {
        Token::Inteiro(texto) => match texto.parse::<i64>() {
            Ok(v) if (min..=max).contains(&v) => Ok(v),
            _ => fora_de_faixa(texto, tipo, min, max),
        },
        Token::Decimal(texto) => {
            let d: f64 = texto.parse().unwrap_or(f64::NAN);
            if d.is_nan() || d < min as f64 || d > max as f64 {
                return fora_de_faixa(texto, tipo, min, max);
            }
            Ok(d as i64)
        }
        _ => Err(ErroJson::new("número esperado")),
    }
}

impl LerJson for i32 {
    fn tipo_java() -> String {
        "java.lang.Integer".into()
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        let t = token(l)?;
        let tipo = Self::tipo_java();
        match &t {
            Token::Nulo => Ok(None),
            Token::Inteiro(_) | Token::Decimal(_) => {
                let v = inteiro_de_numero(&t, "int", MIN_INT, MAX_INT)?;
                Ok(i32::try_from(v).ok())
            }
            Token::Texto(s) => {
                let Some(texto) = texto_escalar(s) else {
                    return Ok(None);
                };
                let Some(v) = parse_inteiro_java(texto) else {
                    return valor_estranho(&tipo, texto, "not a valid `java.lang.Integer` value");
                };
                if texto.len() > DIGITOS_INT_SEM_OVERFLOW && i64::try_from(v).is_err() {
                    return valor_estranho(&tipo, texto, "not a valid `java.lang.Integer` value");
                }
                match i32::try_from(v) {
                    Ok(v) => Ok(Some(v)),
                    Err(_) => valor_estranho(
                        &tipo,
                        texto,
                        &format!(
                            "Overflow: numeric value ({texto}) out of range of `java.lang.Integer` ({MIN_INT} -{MAX_INT})"
                        ),
                    ),
                }
            }
            _ => token_inesperado(&tipo, &t),
        }
    }
}

impl LerJson for i64 {
    fn tipo_java() -> String {
        "java.lang.Long".into()
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        let t = token(l)?;
        let tipo = Self::tipo_java();
        match &t {
            Token::Nulo => Ok(None),
            Token::Inteiro(_) | Token::Decimal(_) => {
                inteiro_de_numero(&t, "long", i64::MIN, i64::MAX).map(Some)
            }
            Token::Texto(s) => {
                let Some(texto) = texto_escalar(s) else {
                    return Ok(None);
                };
                match parse_inteiro_java(texto).and_then(|v| i64::try_from(v).ok()) {
                    Some(v) => Ok(Some(v)),
                    None => valor_estranho(&tipo, texto, "not a valid `java.lang.Long` value"),
                }
            }
            _ => token_inesperado(&tipo, &t),
        }
    }
}

impl LerJson for BigDecimal {
    fn tipo_java() -> String {
        "java.math.BigDecimal".into()
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        let t = token(l)?;
        let tipo = Self::tipo_java();
        match &t {
            Token::Nulo => Ok(None),
            Token::Inteiro(texto) | Token::Decimal(texto) => texto
                .parse()
                .map(Some)
                .map_err(|_| ErroJson::new(format!("Invalid numeric value: {texto}"))),
            Token::Texto(s) => {
                let Some(texto) = texto_escalar(s) else {
                    return Ok(None);
                };
                match texto.parse() {
                    Ok(v) => Ok(Some(v)),
                    Err(_) => valor_estranho(&tipo, texto, "not a valid representation"),
                }
            }
            _ => token_inesperado(&tipo, &t),
        }
    }
}

impl LerJson for String {
    fn tipo_java() -> String {
        "java.lang.String".into()
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        let t = token(l)?;
        match &t {
            Token::Nulo => Ok(None),
            Token::InicioLista | Token::InicioObjeto => token_inesperado(&Self::tipo_java(), &t),
            _ => Ok(Some(t.texto())),
        }
    }
}

impl LerJson for bool {
    fn tipo_java() -> String {
        "java.lang.Boolean".into()
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        let t = token(l)?;
        let tipo = Self::tipo_java();
        match &t {
            Token::Nulo => Ok(None),
            Token::Verdadeiro => Ok(Some(true)),
            Token::Falso => Ok(Some(false)),
            Token::Inteiro(texto) => Ok(Some(texto != "0")),
            Token::Texto(s) => match texto_escalar(s) {
                None => Ok(None),
                Some(v) if v.eq_ignore_ascii_case("true") => Ok(Some(true)),
                Some(v) if v.eq_ignore_ascii_case("false") => Ok(Some(false)),
                Some(v) => valor_estranho(&tipo, v, "only \"true\" or \"false\" recognized"),
            },
            _ => token_inesperado(&tipo, &t),
        }
    }
}

/// lista Java (`ArrayList`); elementos `null` são preservados
impl<T: LerJson> LerJson for Vec<Option<T>> {
    fn tipo_java() -> String {
        format!("java.util.ArrayList<{}>", T::tipo_java())
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        ler_lista(l, &Self::tipo_java(), T::ler)
    }
}

/// `CollectionDeserializer` com o leitor de cada elemento
pub fn ler_lista<T>(
    l: &mut Leitor<'_>,
    tipo: &str,
    mut item: impl FnMut(&mut Leitor<'_>) -> R<Option<T>>,
) -> R<Option<Vec<Option<T>>>> {
    let t = token(l)?;
    match t {
        Token::Nulo => Ok(None),
        Token::InicioLista => {
            let mut itens = Vec::new();
            loop {
                match l.proximo()? {
                    Some(Token::FimLista) => return Ok(Some(itens)),
                    Some(_) => itens.push(item(l)?),
                    None => return Err(ErroJson::new("Unexpected end-of-input")),
                }
            }
        }
        _ => token_inesperado(tipo, &t),
    }
}

/// `boolean` primitivo: aceita só as três grafias de cada valor
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct BooleanoPrimitivo(pub bool);

impl LerJson for BooleanoPrimitivo {
    fn tipo_java() -> String {
        "boolean".into()
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        let t = token(l)?;
        let tipo = Self::tipo_java();
        match &t {
            Token::Nulo => Ok(None),
            Token::Verdadeiro => Ok(Some(Self(true))),
            Token::Falso => Ok(Some(Self(false))),
            Token::Inteiro(texto) => Ok(Some(Self(texto != "0"))),
            Token::Texto(s) => match texto_escalar(s) {
                None => Ok(Some(Self(false))),
                Some("true" | "True" | "TRUE") => Ok(Some(Self(true))),
                Some("false" | "False" | "FALSE") => Ok(Some(Self(false))),
                Some(v) => valor_estranho(
                    &tipo,
                    v,
                    "only \"true\"/\"True\"/\"TRUE\" or \"false\"/\"False\"/\"FALSE\" recognized",
                ),
            },
            _ => token_inesperado(&tipo, &t),
        }
    }
}

fn sem_construtor<T>(tipo: &str, motivo: String) -> R<T> {
    Err(ErroJson::new(format!(
        "Cannot construct instance of `{tipo}` (although at least one Creator exists): {motivo}"
    )))
}

/// como texto em branco chega a um bean: os DTOs do ROC tratam como vazio
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextoBranco {
    Distinto,
    ComoVazio,
}

/// `BeanDeserializer`: `campo` devolve `false` para propriedades desconhecidas (ignoradas)
pub fn ler_objeto<T: Default>(
    l: &mut Leitor<'_>,
    tipo: &str,
    campo: impl FnMut(&mut T, &str, &mut Leitor<'_>) -> R<bool>,
) -> R<Option<T>> {
    ler_objeto_com(l, tipo, TextoBranco::Distinto, campo)
}

pub fn ler_objeto_com<T: Default>(
    l: &mut Leitor<'_>,
    tipo: &str,
    branco: TextoBranco,
    mut campo: impl FnMut(&mut T, &str, &mut Leitor<'_>) -> R<bool>,
) -> R<Option<T>> {
    let t = token(l)?;
    match &t {
        Token::Nulo => return Ok(None),
        Token::InicioObjeto => {}
        Token::Texto(s) if s.is_empty() || (branco == TextoBranco::ComoVazio && eh_branco(s)) => {
            return sem_coercao(tipo, "empty String (\"\")");
        }
        Token::Texto(s) if eh_branco(s) => {
            return sem_coercao(tipo, "blank String (all whitespace)");
        }
        Token::Texto(s) => {
            return sem_construtor(
                tipo,
                format!(
                    "no String-argument constructor/factory method to deserialize from String value ('{s}')"
                ),
            );
        }
        Token::Inteiro(texto) => {
            let tipo_num = match texto.parse::<i64>() {
                Ok(v) if (MIN_INT..=MAX_INT).contains(&v) => "int/Int",
                Ok(_) => "long/Long",
                Err(_) => "BigInteger",
            };
            return sem_construtor(
                tipo,
                format!(
                    "no {tipo_num}-argument constructor/factory method to deserialize from Number value ({texto})"
                ),
            );
        }
        Token::Decimal(texto) => {
            return sem_construtor(
                tipo,
                format!(
                    "no double/Double-argument constructor/factory method to deserialize from Number value ({texto})"
                ),
            );
        }
        Token::Verdadeiro | Token::Falso => {
            return sem_construtor(
                tipo,
                format!(
                    "no boolean/Boolean-argument constructor/factory method to deserialize from boolean value ({})",
                    t.texto()
                ),
            );
        }
        _ => return token_inesperado(tipo, &t),
    }
    let mut obj = T::default();
    loop {
        match l.proximo()? {
            Some(Token::FimObjeto) => return Ok(Some(obj)),
            Some(Token::Campo(nome)) => {
                l.proximo()?;
                if !campo(&mut obj, &nome, l)? {
                    l.pular_filhos()?;
                }
            }
            _ => return Err(ErroJson::new("Unexpected end-of-input")),
        }
    }
}

/// enum Java com `@JsonValue` inteiro: casa pelo texto do código
pub fn ler_enum_codigo<E: Copy>(
    l: &mut Leitor<'_>,
    tipo: &str,
    valores: &[(&str, E)],
) -> R<Option<E>> {
    let t = token(l)?;
    let texto = match &t {
        Token::Nulo => return Ok(None),
        Token::Texto(s) | Token::Inteiro(s) => s.clone(),
        _ => return token_inesperado(tipo, &t),
    };
    let achar = |s: &str| valores.iter().find(|(k, _)| *k == s).map(|(_, v)| *v);
    if let Some(v) = achar(&texto) {
        return Ok(Some(v));
    }
    let aparado = trim_java(&texto);
    if let Some(v) = achar(aparado) {
        return Ok(Some(v));
    }
    if aparado.is_empty() {
        return sem_coercao(tipo, "empty String (\"\")");
    }
    let chaves: Vec<&str> = valores.iter().map(|(k, _)| *k).collect();
    valor_estranho(
        tipo,
        aparado,
        &format!(
            "not one of the values accepted for Enum class: [{}]",
            chaves.join(", ")
        ),
    )
}

/// `int` primitivo do Jackson: texto vazio vira 0 e decimal é truncado
pub fn ler_int_primitivo(l: &mut Leitor<'_>) -> R<Option<i32>> {
    const TIPO: &str = "int";
    let t = token(l)?;
    match &t {
        Token::Nulo => Ok(None),
        Token::Inteiro(_) | Token::Decimal(_) => {
            let v = inteiro_de_numero(&t, TIPO, MIN_INT, MAX_INT)?;
            Ok(i32::try_from(v).ok())
        }
        Token::Texto(s) => {
            let Some(texto) = texto_escalar(s) else {
                return Ok(Some(0));
            };
            match parse_inteiro_java(texto).and_then(|v| i32::try_from(v).ok()) {
                Some(v) => Ok(Some(v)),
                None => valor_estranho(TIPO, texto, "not a valid `int` value"),
            }
        }
        _ => token_inesperado(TIPO, &t),
    }
}

/// `@JsonCreator fromCodigo(int)` que lança `IllegalArgumentException`
pub fn ler_enum_criador_int<E: Copy>(
    l: &mut Leitor<'_>,
    tipo: &str,
    valores: &[(i32, E)],
) -> R<Option<E>> {
    let Some(codigo) = ler_int_primitivo(l)? else {
        return Ok(None);
    };
    match valores.iter().find(|(c, _)| *c == codigo) {
        Some((_, v)) => Ok(Some(*v)),
        None => {
            let simples = tipo.rsplit('.').next().unwrap_or(tipo);
            Err(ErroJson::new(format!(
                "Cannot construct instance of `{tipo}`, problem: Código inválido para {simples}: {codigo}"
            )))
        }
    }
}

/// enum por nome (`@JsonProperty`), aceitando o índice como número ou texto
pub fn ler_enum_nome<E: Copy>(
    l: &mut Leitor<'_>,
    tipo: &str,
    valores: &[(&str, E)],
) -> R<Option<E>> {
    let t = token(l)?;
    let por_indice = |i: usize| valores.get(i).map(|(_, v)| *v);
    let ultimo = valores.len().saturating_sub(1);
    let texto = match &t {
        Token::Nulo => return Ok(None),
        Token::Inteiro(n) => {
            return match n.parse::<usize>().ok().and_then(por_indice) {
                Some(v) => Ok(Some(v)),
                None => Err(ErroJson::new(format!(
                    "Cannot deserialize value of type `{tipo}` from number {n}: index value outside legal index range [0..{ultimo}]"
                ))),
            };
        }
        Token::Texto(s) => s.clone(),
        _ => return token_inesperado(tipo, &t),
    };
    let achar = |s: &str| valores.iter().find(|(k, _)| *k == s).map(|(_, v)| *v);
    let aparado = trim_java(&texto);
    if let Some(v) = achar(&texto).or_else(|| achar(aparado)) {
        return Ok(Some(v));
    }
    if aparado.is_empty() {
        return sem_coercao(tipo, "empty String (\"\")");
    }
    if aparado.bytes().all(|b| b.is_ascii_digit())
        && let Some(v) = aparado.parse::<usize>().ok().and_then(por_indice)
    {
        return Ok(Some(v));
    }
    let mut chaves: Vec<&str> = valores.iter().map(|(k, _)| *k).collect();
    chaves.sort_unstable();
    valor_estranho(
        tipo,
        aparado,
        &format!(
            "not one of the values accepted for Enum class: [{}]",
            chaves.join(", ")
        ),
    )
}

/// `OffsetDateTime` já validado pelo deserializer customizado
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataHora {
    pub local: NaiveDateTime,
    pub offset: FixedOffset,
}

impl DataHora {
    pub fn data(&self) -> NaiveDate {
        self.local.date()
    }
}

struct FormatoData {
    padrao: fn(&str) -> bool,
    formato: &'static str,
    exemplo: &'static str,
}

fn formato_invalido<T>(l: &Leitor<'_>, texto: &str, f: &FormatoData) -> R<T> {
    let nome = l.nome_atual().unwrap_or("null");
    Err(ErroJson::new(format!(
        "Campo '{nome}' [{texto}] inválido. Formato esperado: '{}'. Exemplo de valor: '{}'",
        f.formato, f.exemplo
    )))
}

fn digitos_ascii(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

fn casa_data(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && digitos_ascii(&s[..4])
        && digitos_ascii(&s[5..7])
        && digitos_ascii(&s[8..])
}

fn casa_data_hora(s: &str) -> bool {
    let b = s.as_bytes();
    if b.len() < 20 || !casa_data(&s[..10]) || b[10] != b'T' {
        return false;
    }
    let hora = &s[11..19];
    let hb = hora.as_bytes();
    let hora_ok = hb[2] == b':'
        && hb[5] == b':'
        && digitos_ascii(&hora[..2])
        && digitos_ascii(&hora[3..5])
        && digitos_ascii(&hora[6..]);
    let off = &s[19..];
    let ob = off.as_bytes();
    let off_ok = off == "Z"
        || (ob.len() == 6
            && (ob[0] == b'+' || ob[0] == b'-')
            && ob[3] == b':'
            && digitos_ascii(&off[1..3])
            && digitos_ascii(&off[4..]));
    hora_ok && off_ok
}

const FORMATO_DATA: FormatoData = FormatoData {
    padrao: casa_data,
    formato: "yyyy-MM-dd",
    exemplo: "2026-01-05",
};

const FORMATO_DATA_HORA: FormatoData = FormatoData {
    padrao: casa_data_hora,
    formato: "yyyy-MM-dd'T'HH:mm:ssXXX",
    exemplo: "2026-01-05T13:45:00-03:00 ou 2026-05-22T10:57:23Z",
};

fn parse_data(s: &str) -> Option<NaiveDate> {
    let ano = s[..4].parse().ok()?;
    let mes = s[5..7].parse().ok()?;
    let dia = s[8..10].parse().ok()?;
    NaiveDate::from_ymd_opt(ano, mes, dia)
}

const SEGUNDOS_HORA: i32 = 3600;
const MAX_OFFSET_SEGUNDOS: i32 = 18 * SEGUNDOS_HORA;

fn parse_offset(s: &str) -> Option<FixedOffset> {
    if s == "Z" {
        return FixedOffset::east_opt(0);
    }
    let horas: i32 = s[1..3].parse().ok()?;
    let minutos: i32 = s[4..6].parse().ok()?;
    if minutos > 59 {
        return None;
    }
    let total = horas * SEGUNDOS_HORA + minutos * 60;
    if total > MAX_OFFSET_SEGUNDOS {
        return None;
    }
    FixedOffset::east_opt(if s.starts_with('-') { -total } else { total })
}

fn parse_data_hora(s: &str) -> Option<DataHora> {
    let data = parse_data(&s[..10])?;
    let h = s[11..13].parse().ok()?;
    let m = s[14..16].parse().ok()?;
    let seg = s[17..19].parse().ok()?;
    let local = data.and_hms_opt(h, m, seg)?;
    Some(DataHora {
        local,
        offset: parse_offset(&s[19..])?,
    })
}

fn ler_formatado<T>(
    l: &mut Leitor<'_>,
    f: &FormatoData,
    parse: fn(&str) -> Option<T>,
) -> R<Option<T>> {
    let t = token(l)?;
    if t == Token::Nulo {
        return Ok(None);
    }
    let texto = t.texto();
    if !(f.padrao)(&texto) {
        return formato_invalido(l, &texto, f);
    }
    match parse(&texto) {
        Some(v) => Ok(Some(v)),
        None => formato_invalido(l, &texto, f),
    }
}

impl LerJson for DataHora {
    fn tipo_java() -> String {
        "java.time.OffsetDateTime".into()
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        ler_formatado(l, &FORMATO_DATA_HORA, parse_data_hora)
    }
}

impl LerJson for NaiveDate {
    fn tipo_java() -> String {
        "java.time.LocalDate".into()
    }

    fn ler(l: &mut Leitor<'_>) -> R<Option<Self>> {
        ler_formatado(l, &FORMATO_DATA, parse_data)
    }
}

/// gera `LerJson` para um DTO Java: cada campo é `Option<T>` e o nome JSON é o do campo Java
#[macro_export]
macro_rules! objeto_json {
    ($tipo:ty, $java:expr, { $($nome:literal => $campo:ident : $t:ty),* $(,)? }) => {
        impl $crate::json::LerJson for $tipo {
            fn tipo_java() -> String {
                String::from($java)
            }

            fn ler(l: &mut $crate::json::Leitor<'_>) -> $crate::json::R<Option<Self>> {
                $crate::json::ler_objeto(l, $java, |obj: &mut Self, nome, l| {
                    match nome {
                        $($nome => {
                            obj.$campo = <$t as $crate::json::LerJson>::ler(l)?;
                            Ok(true)
                        })*
                        _ => Ok(false),
                    }
                })
            }
        }
    };
}

#[cfg(test)]
mod tests;
