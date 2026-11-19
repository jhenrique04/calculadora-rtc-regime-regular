use std::cmp::Ordering;

use num_bigint::BigInt;
use num_integer::Integer as _;
use num_traits::{Signed, Zero};

use super::ErroDecimal;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoundingMode {
    Up,
    Down,
    Ceiling,
    Floor,
    HalfUp,
    HalfDown,
    HalfEven,
    Unnecessary,
}

/// divide `num/den` arredondando para inteiro; devolve também se foi exata
pub(super) fn dividir(
    num: &BigInt,
    den: &BigInt,
    modo: RoundingMode,
) -> Result<(BigInt, bool), ErroDecimal> {
    if den.is_zero() {
        return Err(ErroDecimal::DivisaoPorZero);
    }
    // truncamento em direção a zero, como o BigInteger do Java
    let (q, r) = num.div_rem(den);
    if r.is_zero() {
        return Ok((q, true));
    }
    let negativo = num.is_negative() != den.is_negative();
    let metade = (r.abs() * 2u8).cmp(&den.abs());
    let afastar = match modo {
        RoundingMode::Unnecessary => return Err(ErroDecimal::ArredondamentoNecessario),
        RoundingMode::Up => true,
        RoundingMode::Down => false,
        RoundingMode::Ceiling => !negativo,
        RoundingMode::Floor => negativo,
        RoundingMode::HalfUp => metade != Ordering::Less,
        RoundingMode::HalfDown => metade == Ordering::Greater,
        RoundingMode::HalfEven => match metade {
            Ordering::Greater => true,
            Ordering::Less => false,
            Ordering::Equal => q.is_odd(),
        },
    };
    if !afastar {
        return Ok((q, false));
    }
    let q = if negativo { q - 1 } else { q + 1 };
    Ok((q, false))
}
