//! `ArredondamentoUtils`: escala interna 8 e formatação de moeda/alíquota

use crate::decimal::{BigDecimal, RoundingMode};

pub const PRECISAO_INTERNA: i64 = 8;
pub const MODO: RoundingMode = RoundingMode::HalfEven;
const ESCALA_MOEDA: i64 = 2;
const ESCALA_ALIQUOTA_MAX: i64 = 4;
const ESCALA_QUANTIDADE: i64 = 4;

pub fn cem() -> BigDecimal {
    BigDecimal::from_i64(100)
}

fn com_escala(valor: &BigDecimal, escala: i64) -> BigDecimal {
    // HALF_EVEN nunca falha; o fallback só existe para não usar unwrap
    valor
        .set_scale(escala, MODO)
        .unwrap_or_else(|_| valor.clone())
}

pub fn arredondar_interno(valor: &BigDecimal) -> BigDecimal {
    com_escala(valor, PRECISAO_INTERNA)
}

pub fn dividir(dividendo: &BigDecimal, divisor: &BigDecimal) -> Option<BigDecimal> {
    dividendo.divide_scale(divisor, PRECISAO_INTERNA, MODO).ok()
}

pub fn dividir_por_cem(valor: &BigDecimal) -> BigDecimal {
    dividir(valor, &cem()).unwrap_or_else(|| valor.clone())
}

pub fn multiplicar(v1: &BigDecimal, v2: &BigDecimal) -> BigDecimal {
    arredondar_interno(&v1.multiply(v2))
}

pub fn subtrair(v1: &BigDecimal, v2: &BigDecimal) -> BigDecimal {
    arredondar_interno(&v1.subtract(v2))
}

pub fn multiplicar_por_cem(valor: &BigDecimal) -> BigDecimal {
    valor.multiply(&cem())
}

pub fn totalizar<'a>(valores: impl Iterator<Item = &'a BigDecimal>) -> BigDecimal {
    valores.fold(BigDecimal::zero(), |acc, v| acc.add(v))
}

pub fn formatar_moeda(valor: &BigDecimal) -> String {
    com_escala(valor, ESCALA_MOEDA).to_plain_string()
}

pub fn formatar_aliquota(valor: &BigDecimal) -> String {
    let mut s = com_escala(valor, ESCALA_ALIQUOTA_MAX).strip_trailing_zeros();
    if s.scale() < ESCALA_MOEDA {
        s = com_escala(&s, ESCALA_MOEDA);
    }
    s.to_plain_string()
}

pub fn formatar_quantidade(valor: &BigDecimal) -> String {
    if valor.strip_trailing_zeros().scale() <= 0 {
        return valor.to_bigint().to_string();
    }
    com_escala(valor, ESCALA_QUANTIDADE).to_plain_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bd(s: &str) -> BigDecimal {
        s.parse().unwrap_or_else(|_| BigDecimal::zero())
    }

    #[test]
    fn formatacoes() {
        assert_eq!(formatar_moeda(&bd("1.805")), "1.80");
        assert_eq!(formatar_moeda(&bd("1.815")), "1.82");
        assert_eq!(formatar_aliquota(&bd("0.9")), "0.90");
        assert_eq!(formatar_aliquota(&bd("0.12345")), "0.1234");
        assert_eq!(formatar_aliquota(&bd("12")), "12.00");
        assert_eq!(formatar_quantidade(&bd("3.000")), "3");
        assert_eq!(formatar_quantidade(&bd("3.5")), "3.5000");
        assert_eq!(dividir_por_cem(&bd("0.9")).to_string(), "0.00900000");
    }
}
