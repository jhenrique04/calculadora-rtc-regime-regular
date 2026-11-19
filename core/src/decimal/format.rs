use num_bigint::BigInt;
use num_traits::Signed;

use super::{BigDecimal, ErroDecimal};

/// `new BigDecimal(String)`: `[+-]dígitos[.dígitos][(e|E)[+-]dígitos]`
pub(super) fn parse(s: &str) -> Result<BigDecimal, ErroDecimal> {
    let (mantissa, expoente) = match s.find(['e', 'E']) {
        Some(i) => (&s[..i], parse_expoente(&s[i + 1..])?),
        None => (s, 0),
    };
    let (negativo, corpo) = match mantissa.as_bytes().first() {
        Some(b'-') => (true, &mantissa[1..]),
        Some(b'+') => (false, &mantissa[1..]),
        _ => (false, mantissa),
    };
    let (inteiro, fracao) = corpo.split_once('.').unwrap_or((corpo, ""));
    let digitos = format!("{inteiro}{fracao}");
    if digitos.is_empty() || !digitos.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ErroDecimal::Formato);
    }
    let valor: BigInt = digitos.parse().map_err(|_| ErroDecimal::Formato)?;
    let valor = if negativo { -valor } else { valor };
    let escala = fracao.len() as i64 - expoente;
    Ok(BigDecimal::new(valor, escala))
}

fn parse_expoente(s: &str) -> Result<i64, ErroDecimal> {
    let corpo = s.strip_prefix(['+', '-']).unwrap_or(s);
    if corpo.is_empty() || !corpo.bytes().all(|b| b.is_ascii_digit()) {
        return Err(ErroDecimal::Formato);
    }
    s.parse().map_err(|_| ErroDecimal::Formato)
}

fn sinal(v: &BigInt) -> &'static str {
    if v.is_negative() { "-" } else { "" }
}

pub(super) fn canonico(unscaled: &BigInt, escala: i64) -> String {
    let coef = unscaled.abs().to_string();
    let ajustado = -escala + (coef.len() as i64 - 1);
    if escala >= 0 && ajustado >= -6 {
        return format!("{}{}", sinal(unscaled), com_ponto(&coef, escala));
    }
    let mut out = String::from(sinal(unscaled));
    out.push_str(&coef[..1]);
    if coef.len() > 1 {
        out.push('.');
        out.push_str(&coef[1..]);
    }
    out.push('E');
    if ajustado >= 0 {
        out.push('+');
    }
    out.push_str(&ajustado.to_string());
    out
}

pub(super) fn plain(unscaled: &BigInt, escala: i64) -> String {
    let coef = unscaled.abs().to_string();
    if escala >= 0 {
        return format!("{}{}", sinal(unscaled), com_ponto(&coef, escala));
    }
    if unscaled.sign() == num_bigint::Sign::NoSign {
        return "0".into();
    }
    let zeros = "0".repeat(usize::try_from(-escala).unwrap_or(0));
    format!("{}{coef}{zeros}", sinal(unscaled))
}

fn com_ponto(coef: &str, escala: i64) -> String {
    let escala = usize::try_from(escala).unwrap_or(0);
    if escala == 0 {
        return coef.to_string();
    }
    if coef.len() > escala {
        let (i, f) = coef.split_at(coef.len() - escala);
        return format!("{i}.{f}");
    }
    format!("0.{}{coef}", "0".repeat(escala - coef.len()))
}
