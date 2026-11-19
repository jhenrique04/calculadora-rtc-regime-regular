use std::fmt::Display;

use chrono::NaiveDate;

use crate::decimal::BigDecimal;
use crate::util::arredondamento::{formatar_aliquota, formatar_moeda, formatar_quantidade};

/// falha de um serializer customizado (o Jackson aborta a escrita da resposta)
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct ErroEscrita(pub String);

type R = Result<(), ErroEscrita>;

pub trait Escrever {
    fn escrever(&self, e: &mut Escritor) -> R;
}

/// gerador compacto equivalente ao `JsonGenerator` padrão (sem espaços, UTF-8 cru)
#[derive(Default)]
pub struct Escritor {
    buf: String,
    precisa_virgula: Vec<bool>,
}

const MAX_U_TRIB: usize = 6;
const TAM_CST: usize = 3;
const TAM_CLASS_TRIB: usize = 6;

fn so_digitos(s: &str, tamanho: usize) -> bool {
    s.len() == tamanho && s.bytes().all(|b| b.is_ascii_digit())
}

fn nao_negativo(v: &BigDecimal, tipo: &str) -> R {
    if v.signum() < 0 {
        return Err(ErroEscrita(format!(
            "Números negativos não são permitidos para {tipo}"
        )));
    }
    Ok(())
}

impl Escritor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn finalizar(self) -> String {
        self.buf
    }

    fn separar(&mut self) {
        if let Some(v) = self.precisa_virgula.last_mut() {
            if *v {
                self.buf.push(',');
            }
            *v = true;
        }
    }

    pub fn inicio_objeto(&mut self) {
        self.separar();
        self.buf.push('{');
        self.precisa_virgula.push(false);
    }

    pub fn fim_objeto(&mut self) {
        self.precisa_virgula.pop();
        self.buf.push('}');
    }

    pub fn inicio_lista(&mut self) {
        self.separar();
        self.buf.push('[');
        self.precisa_virgula.push(false);
    }

    pub fn fim_lista(&mut self) {
        self.precisa_virgula.pop();
        self.buf.push(']');
    }

    /// nome de campo: o valor seguinte não leva vírgula própria
    pub fn nome(&mut self, nome: &str) {
        self.separar();
        self.string_crua(nome);
        self.buf.push(':');
        if let Some(v) = self.precisa_virgula.last_mut() {
            *v = false;
        }
    }

    fn depois_do_valor(&mut self) {
        if let Some(v) = self.precisa_virgula.last_mut() {
            *v = true;
        }
    }

    fn string_crua(&mut self, s: &str) {
        self.buf.push('"');
        for c in s.chars() {
            match c {
                '"' => self.buf.push_str("\\\""),
                '\\' => self.buf.push_str("\\\\"),
                '\n' => self.buf.push_str("\\n"),
                '\r' => self.buf.push_str("\\r"),
                '\t' => self.buf.push_str("\\t"),
                '\u{8}' => self.buf.push_str("\\b"),
                '\u{c}' => self.buf.push_str("\\f"),
                c if u32::from(c) < 0x20 => {
                    self.buf.push_str(&format!("\\u{:04X}", u32::from(c)));
                }
                c => self.buf.push(c),
            }
        }
        self.buf.push('"');
    }

    pub fn valor_string(&mut self, s: &str) {
        self.separar();
        self.string_crua(s);
        self.depois_do_valor();
    }

    pub fn valor_bruto(&mut self, s: impl Display) {
        self.separar();
        self.buf.push_str(&s.to_string());
        self.depois_do_valor();
    }

    pub fn texto(&mut self, nome: &str, v: &Option<String>) -> R {
        if let Some(v) = v {
            self.nome(nome);
            self.valor_string(v);
        }
        Ok(())
    }

    pub fn inteiro<I: Display>(&mut self, nome: &str, v: &Option<I>) -> R {
        if let Some(v) = v {
            self.nome(nome);
            self.valor_bruto(v);
        }
        Ok(())
    }

    pub fn booleano(&mut self, nome: &str, v: &bool) -> R {
        self.nome(nome);
        self.valor_bruto(v);
        Ok(())
    }

    /// `Boolean` (caixa): nulo é omitido
    pub fn logico(&mut self, nome: &str, v: &Option<bool>) -> R {
        match v {
            Some(b) => self.booleano(nome, b),
            None => Ok(()),
        }
    }

    /// `BigDecimal` sem serializer customizado: `toString()` como número JSON
    pub fn decimal(&mut self, nome: &str, v: &Option<BigDecimal>) -> R {
        if let Some(v) = v {
            self.nome(nome);
            self.valor_bruto(v);
        }
        Ok(())
    }

    /// `BigDecimal` com `@JsonInclude(ALWAYS)`: nulo sai como `null`
    pub fn decimal_sempre(&mut self, nome: &str, v: &Option<BigDecimal>) -> R {
        self.nome(nome);
        match v {
            Some(v) => self.valor_bruto(v),
            None => self.valor_bruto("null"),
        }
        Ok(())
    }

    pub fn moeda(&mut self, nome: &str, v: &Option<BigDecimal>) -> R {
        self.formatado(nome, v, "TDec1302", formatar_moeda)
    }

    pub fn aliquota(&mut self, nome: &str, v: &Option<BigDecimal>) -> R {
        self.formatado(nome, v, "TDec_0302_04", formatar_aliquota)
    }

    pub fn quantidade(&mut self, nome: &str, v: &Option<BigDecimal>) -> R {
        self.formatado(nome, v, "TDec_1104OpRTC", formatar_quantidade)
    }

    fn formatado(
        &mut self,
        nome: &str,
        v: &Option<BigDecimal>,
        tipo: &str,
        formatar: fn(&BigDecimal) -> String,
    ) -> R {
        if let Some(v) = v {
            nao_negativo(v, tipo)?;
            self.nome(nome);
            self.valor_string(&formatar(v));
        }
        Ok(())
    }

    pub fn cst(&mut self, nome: &str, v: &Option<String>) -> R {
        self.validado(
            nome,
            v,
            |s| so_digitos(s, TAM_CST),
            "TCST deve ser uma string de exatamente 3 dígitos numéricos",
        )
    }

    pub fn class_trib(&mut self, nome: &str, v: &Option<String>) -> R {
        self.validado(
            nome,
            v,
            |s| so_digitos(s, TAM_CLASS_TRIB),
            "TcClassTrib deve ser uma string de exatamente 6 dígitos numéricos",
        )
    }

    pub fn u_trib(&mut self, nome: &str, v: &Option<String>) -> R {
        let valido = |s: &str| {
            let n = s.encode_utf16().count();
            n > 0 && n <= MAX_U_TRIB
        };
        self.validado(nome, v, valido, "uTrib deve ter entre 1 e 6 caracteres")
    }

    fn validado(
        &mut self,
        nome: &str,
        v: &Option<String>,
        ok: impl Fn(&str) -> bool,
        msg: &str,
    ) -> R {
        if let Some(v) = v {
            if !ok(v) {
                return Err(ErroEscrita(msg.into()));
            }
            self.nome(nome);
            self.valor_string(v);
        }
        Ok(())
    }

    /// `LocalDate` com `WRITE_DATES_AS_TIMESTAMPS` desligado: ISO `yyyy-MM-dd`
    pub fn data(&mut self, nome: &str, v: &Option<NaiveDate>) -> R {
        if let Some(v) = v {
            self.nome(nome);
            self.valor_string(&v.format("%Y-%m-%d").to_string());
        }
        Ok(())
    }

    /// `OffsetDateTime` com `ISO_OFFSET_DATE_TIME` (offset zero vira "Z")
    pub fn data_hora(&mut self, nome: &str, v: &Option<super::DataHora>) -> R {
        let Some(d) = v else { return Ok(()) };
        let segundos = d.offset.local_minus_utc();
        let offset = if segundos == 0 {
            "Z".to_string()
        } else {
            let sinal = if segundos < 0 { '-' } else { '+' };
            let abs = segundos.unsigned_abs();
            format!("{sinal}{:02}:{:02}", abs / 3600, abs / 60 % 60)
        };
        let texto = format!("{}{offset}", d.local.format("%Y-%m-%dT%H:%M:%S"));
        self.texto(nome, &Some(texto))
    }

    pub fn objeto<T: Escrever>(&mut self, nome: &str, v: &Option<T>) -> R {
        if let Some(v) = v {
            self.nome(nome);
            v.escrever(self)?;
            self.depois_do_valor();
        }
        Ok(())
    }

    pub fn lista<T: Escrever>(&mut self, nome: &str, v: &Option<Vec<T>>) -> R {
        self.objeto(nome, v)
    }
}

impl Escrever for String {
    fn escrever(&self, e: &mut Escritor) -> R {
        e.valor_string(self);
        Ok(())
    }
}

impl<T: Escrever> Escrever for Vec<T> {
    fn escrever(&self, e: &mut Escritor) -> R {
        e.inicio_lista();
        for item in self {
            item.escrever(e)?;
        }
        e.fim_lista();
        Ok(())
    }
}

/// gera `Escrever` para um DTO de saída (`@JsonInclude(NON_NULL)` + `@JsonPropertyOrder`)
#[macro_export]
macro_rules! saida_json {
    ($tipo:ty { $($nome:literal => $campo:ident : $fmt:ident),* $(,)? }) => {
        impl $crate::json::Escrever for $tipo {
            fn escrever(&self, e: &mut $crate::json::Escritor) -> Result<(), $crate::json::ErroEscrita> {
                e.inicio_objeto();
                $( e.$fmt($nome, &self.$campo)?; )*
                e.fim_objeto();
                Ok(())
            }
        }
    };
}
