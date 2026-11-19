//! `java.math.BigDecimal` com a mesma semântica de escala, arredondamento e
//! formatação do JDK, para que cálculos e JSON saiam idênticos ao Java

mod format;
mod round;

#[cfg(test)]
mod tests;

use std::cmp::Ordering;
use std::fmt;

use num_bigint::{BigInt, Sign};
use num_integer::Integer as _;
use num_traits::{Signed, ToPrimitive, Zero};
use thiserror::Error;

pub use round::RoundingMode;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ErroDecimal {
    #[error("Character array is missing \"exponent\" mark 'e' or 'E'.")]
    Formato,
    #[error("Division by zero")]
    DivisaoPorZero,
    #[error("Rounding necessary")]
    ArredondamentoNecessario,
}

/// precisão e modo de arredondamento (`java.math.MathContext`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MathContext {
    pub precisao: u32,
    pub modo: RoundingMode,
}

impl MathContext {
    pub const DECIMAL128: Self = Self {
        precisao: 34,
        modo: RoundingMode::HalfEven,
    };
    pub const DECIMAL64: Self = Self {
        precisao: 16,
        modo: RoundingMode::HalfEven,
    };
}

/// valor = `unscaled × 10^-scale`. `PartialEq` compara escala também, como
/// `BigDecimal.equals`; use `cmp`/`compare_to` para comparar só o valor
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BigDecimal {
    unscaled: BigInt,
    scale: i64,
}

fn pow10(n: u64) -> BigInt {
    num_traits::pow(BigInt::from(10u8), usize::try_from(n).unwrap_or(usize::MAX))
}

fn digitos(v: &BigInt) -> u64 {
    if v.is_zero() {
        return 1;
    }
    v.magnitude().to_string().len() as u64
}

impl BigDecimal {
    pub fn new(unscaled: impl Into<BigInt>, scale: i64) -> Self {
        Self {
            unscaled: unscaled.into(),
            scale,
        }
    }

    pub fn zero() -> Self {
        Self::new(0, 0)
    }

    pub fn one() -> Self {
        Self::new(1, 0)
    }

    pub fn from_i64(v: i64) -> Self {
        Self::new(v, 0)
    }

    pub fn unscaled(&self) -> &BigInt {
        &self.unscaled
    }

    pub fn scale(&self) -> i64 {
        self.scale
    }

    pub fn signum(&self) -> i32 {
        match self.unscaled.sign() {
            Sign::Minus => -1,
            Sign::NoSign => 0,
            Sign::Plus => 1,
        }
    }

    pub fn is_zero(&self) -> bool {
        self.unscaled.is_zero()
    }

    /// quantidade de dígitos do valor sem escala (zero tem precisão 1)
    pub fn precision(&self) -> u64 {
        digitos(&self.unscaled)
    }

    pub fn negate(&self) -> Self {
        Self::new(-&self.unscaled, self.scale)
    }

    pub fn abs(&self) -> Self {
        Self::new(self.unscaled.abs(), self.scale)
    }

    /// unscaled reescalado para `escala` (só aumenta a escala, sem perda)
    fn alinhado(&self, escala: i64) -> BigInt {
        let diff = escala - self.scale;
        if diff <= 0 {
            return self.unscaled.clone();
        }
        &self.unscaled * pow10(diff.unsigned_abs())
    }

    pub fn add(&self, outro: &Self) -> Self {
        let escala = self.scale.max(outro.scale);
        Self::new(self.alinhado(escala) + outro.alinhado(escala), escala)
    }

    pub fn subtract(&self, outro: &Self) -> Self {
        self.add(&outro.negate())
    }

    pub fn multiply(&self, outro: &Self) -> Self {
        Self::new(&self.unscaled * &outro.unscaled, self.scale + outro.scale)
    }

    /// `add(BigDecimal, MathContext)`, incluindo a regra de operando zero
    pub fn add_mc(&self, outro: &Self, mc: MathContext) -> Self {
        if mc.precisao == 0 {
            return self.add(outro);
        }
        let preferida = self.scale.max(outro.scale);
        match (self.is_zero(), outro.is_zero()) {
            (true, true) => return Self::new(0, preferida),
            (false, false) => return self.add(outro).round_mc(mc),
            _ => {}
        }
        let base = if self.is_zero() { outro } else { self };
        let resultado = base.round_mc(mc);
        match resultado.scale.cmp(&preferida) {
            Ordering::Equal => resultado,
            Ordering::Greater => resultado.strip_zeros_to_scale(preferida),
            Ordering::Less => {
                let folga = i64::from(mc.precisao) - resultado.precision() as i64;
                let falta = preferida - resultado.scale;
                let nova = if folga >= falta {
                    preferida
                } else {
                    resultado.scale + folga
                };
                resultado.with_scale_exact(nova)
            }
        }
    }

    pub fn subtract_mc(&self, outro: &Self, mc: MathContext) -> Self {
        self.add_mc(&outro.negate(), mc)
    }

    pub fn multiply_mc(&self, outro: &Self, mc: MathContext) -> Self {
        self.multiply(outro).round_mc(mc)
    }

    /// `divide(BigDecimal, MathContext)`: quociente com `precisao` dígitos;
    /// se exato, escala mais próxima de `this.scale - divisor.scale`
    pub fn divide_mc(&self, divisor: &Self, mc: MathContext) -> Result<Self, ErroDecimal> {
        if divisor.is_zero() {
            return Err(ErroDecimal::DivisaoPorZero);
        }
        let preferida = self.scale - divisor.scale;
        if self.is_zero() {
            return Ok(Self::new(0, preferida));
        }
        let px = self.precision() as i64;
        let mut py = divisor.precision() as i64;
        // mantissa do dividendo maior que a do divisor: quociente ganha um dígito
        if comparar_mantissas(self, px, divisor, py) == Ordering::Greater {
            py -= 1;
        }
        let escala = preferida + py - px + i64::from(mc.precisao);
        let (q, exato) = self.dividir_na_escala(divisor, escala, mc.modo)?;
        let q = if exato {
            q.strip_zeros_to_scale(preferida)
        } else {
            q
        };
        Ok(q.round_mc(mc))
    }

    pub fn divide_scale(
        &self,
        divisor: &Self,
        escala: i64,
        modo: RoundingMode,
    ) -> Result<Self, ErroDecimal> {
        if divisor.is_zero() {
            return Err(ErroDecimal::DivisaoPorZero);
        }
        self.dividir_na_escala(divisor, escala, modo)
            .map(|(q, _)| q)
    }

    fn dividir_na_escala(
        &self,
        divisor: &Self,
        escala: i64,
        modo: RoundingMode,
    ) -> Result<(Self, bool), ErroDecimal> {
        // q = self/divisor × 10^escala = ux × 10^(escala - sx + sy) / uy
        let expoente = escala - self.scale + divisor.scale;
        let (num, den) = if expoente >= 0 {
            (
                &self.unscaled * pow10(expoente.unsigned_abs()),
                divisor.unscaled.clone(),
            )
        } else {
            (
                self.unscaled.clone(),
                &divisor.unscaled * pow10(expoente.unsigned_abs()),
            )
        };
        let (q, exato) = round::dividir(&num, &den, modo)?;
        Ok((Self::new(q, escala), exato))
    }

    /// `round(MathContext)` / `doRound`: reduz a `precisao` dígitos
    pub fn round_mc(&self, mc: MathContext) -> Self {
        let mut atual = self.clone();
        let limite = u64::from(mc.precisao);
        if limite == 0 {
            return atual;
        }
        while atual.precision() > limite {
            let cortar = atual.precision() - limite;
            let divisor = pow10(cortar);
            let (q, _) = match round::dividir(&atual.unscaled, &divisor, mc.modo) {
                Ok(r) => r,
                // modo UNNECESSARY com perda: mantém valor como o JDK lançaria; não usado
                Err(_) => return atual,
            };
            atual = Self::new(q, atual.scale - cortar as i64);
        }
        atual
    }

    pub fn set_scale(&self, escala: i64, modo: RoundingMode) -> Result<Self, ErroDecimal> {
        if escala >= self.scale {
            return Ok(self.with_scale_exact(escala));
        }
        let divisor = pow10((self.scale - escala).unsigned_abs());
        let (q, _) = round::dividir(&self.unscaled, &divisor, modo)?;
        Ok(Self::new(q, escala))
    }

    /// aumenta a escala sem perda (diminuir exige `set_scale` com modo)
    fn with_scale_exact(&self, escala: i64) -> Self {
        if escala >= self.scale {
            return Self::new(self.alinhado(escala), escala);
        }
        let divisor = pow10((self.scale - escala).unsigned_abs());
        Self::new(&self.unscaled / divisor, escala)
    }

    /// remove zeros à direita sem descer abaixo de `preferida`
    fn strip_zeros_to_scale(&self, preferida: i64) -> Self {
        let dez = BigInt::from(10u8);
        let mut u = self.unscaled.clone();
        let mut escala = self.scale;
        while escala > preferida && !u.is_zero() {
            let (q, r) = u.div_rem(&dez);
            if !r.is_zero() {
                break;
            }
            u = q;
            escala -= 1;
        }
        Self::new(u, escala)
    }

    /// `stripTrailingZeros`: zero vira `0` com escala 0 (JDK 8+)
    pub fn strip_trailing_zeros(&self) -> Self {
        if self.is_zero() {
            return Self::zero();
        }
        self.strip_zeros_to_scale(i64::MIN)
    }

    /// escala resultante é `max(scale - n, 0)`
    pub fn move_point_right(&self, n: i64) -> Self {
        let r = Self::new(self.unscaled.clone(), self.scale - n);
        if r.scale < 0 {
            r.with_scale_exact(0)
        } else {
            r
        }
    }

    pub fn move_point_left(&self, n: i64) -> Self {
        self.move_point_right(-n)
    }

    pub fn compare_to(&self, outro: &Self) -> Ordering {
        let escala = self.scale.max(outro.scale);
        self.alinhado(escala).cmp(&outro.alinhado(escala))
    }

    /// parte inteira truncada (`toBigInteger`)
    pub fn to_bigint(&self) -> BigInt {
        if self.scale <= 0 {
            return self.alinhado(0);
        }
        &self.unscaled / pow10(self.scale.unsigned_abs())
    }

    pub fn long_value_exact(&self) -> Option<i64> {
        let s = self.strip_trailing_zeros();
        if s.scale > 0 {
            return None;
        }
        s.to_bigint().to_i64()
    }

    pub fn to_f64(&self) -> f64 {
        self.to_string().parse().unwrap_or(f64::NAN)
    }

    pub fn to_plain_string(&self) -> String {
        format::plain(&self.unscaled, self.scale)
    }

    pub fn max(&self, outro: &Self) -> Self {
        if self.compare_to(outro) == Ordering::Less {
            outro.clone()
        } else {
            self.clone()
        }
    }
}

fn comparar_mantissas(x: &BigDecimal, px: i64, y: &BigDecimal, py: i64) -> Ordering {
    // compara x/10^px com y/10^py ignorando sinal
    let (ax, ay) = (x.unscaled.abs(), y.unscaled.abs());
    match px.cmp(&py) {
        Ordering::Equal => ax.cmp(&ay),
        Ordering::Less => (ax * pow10((py - px).unsigned_abs())).cmp(&ay),
        Ordering::Greater => ax.cmp(&(ay * pow10((px - py).unsigned_abs()))),
    }
}

impl PartialOrd for BigDecimal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for BigDecimal {
    fn cmp(&self, other: &Self) -> Ordering {
        self.compare_to(other)
    }
}

impl std::str::FromStr for BigDecimal {
    type Err = ErroDecimal;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        format::parse(s)
    }
}

/// `toString()`: notação científica quando escala < 0 ou expoente < -6
impl fmt::Display for BigDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&format::canonico(&self.unscaled, self.scale))
    }
}

impl fmt::Debug for BigDecimal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self}")
    }
}

impl Default for BigDecimal {
    fn default() -> Self {
        Self::zero()
    }
}

impl From<i64> for BigDecimal {
    fn from(v: i64) -> Self {
        Self::from_i64(v)
    }
}
