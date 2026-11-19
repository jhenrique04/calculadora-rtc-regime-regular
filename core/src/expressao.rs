//! avaliador das fórmulas tributárias: subconjunto aritmético do JEXL 3.5
//! (`+ - * /`, parênteses, variáveis e literais) com a mesma aritmética do
//! `AvaliadorExpressaoAritmetica` Java (divisão sempre em BigDecimal
//! DECIMAL128, demais operações na promoção de tipos do `JexlArithmetic`)

use std::collections::HashMap;

use num_bigint::BigInt;
use num_traits::ToPrimitive;
use thiserror::Error;

use crate::decimal::{BigDecimal, ErroDecimal, MathContext};
use crate::util::arredondamento::arredondar_interno;

#[derive(Debug, Clone, PartialEq, Error)]
pub enum ErroExpressao {
    #[error("Division by zero")]
    DivisaoPorZero,
    #[error("expressão inválida: {0}")]
    Sintaxe(String),
    #[error("undefined variable {0}")]
    VariavelIndefinida(String),
    #[error("{0}")]
    Decimal(#[from] ErroDecimal),
}

/// tipos numéricos que o JEXL produz durante a avaliação
#[derive(Debug, Clone, PartialEq)]
enum Numero {
    Int(i32),
    Long(i64),
    Inteiro(BigInt),
    Double(f64),
    Decimal(BigDecimal),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Soma,
    Subtracao,
    Multiplicacao,
    Divisao,
}

/// avalia `expressao` com as variáveis dadas; resultado com escala interna
/// (8), exceto quando a expressão é só o nome de uma variável
pub fn avaliar(
    expressao: &str,
    variaveis: &HashMap<&str, BigDecimal>,
) -> Result<BigDecimal, ErroExpressao> {
    let nome = expressao.trim();
    if let Some(valor) = variaveis.get(nome) {
        return Ok(valor.clone());
    }
    let mut p = Parser {
        tokens: tokenizar(expressao)?,
        pos: 0,
        variaveis,
    };
    let valor = p.expressao()?;
    if p.pos != p.tokens.len() {
        return Err(ErroExpressao::Sintaxe(expressao.to_string()));
    }
    Ok(arredondar_interno(&para_bigdecimal(valor)?))
}

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Numero(String),
    Ident(String),
    Op(char),
    Abre,
    Fecha,
}

fn tokenizar(s: &str) -> Result<Vec<Token>, ErroExpressao> {
    let mut tokens = Vec::new();
    let mut chars = s.chars().peekable();
    while let Some(&c) = chars.peek() {
        match c {
            c if c.is_whitespace() => {
                chars.next();
            }
            '0'..='9' | '.' => {
                let mut lit = String::new();
                while let Some(&d) = chars.peek() {
                    let expoente = (d == '+' || d == '-') && lit.ends_with(['e', 'E']);
                    if !(d.is_ascii_alphanumeric() || d == '.' || expoente) {
                        break;
                    }
                    lit.push(d);
                    chars.next();
                }
                tokens.push(Token::Numero(lit));
            }
            c if c.is_alphabetic() || c == '_' || c == '$' => {
                let mut nome = String::new();
                while let Some(&d) = chars.peek() {
                    if !(d.is_alphanumeric() || d == '_' || d == '$') {
                        break;
                    }
                    nome.push(d);
                    chars.next();
                }
                tokens.push(Token::Ident(nome));
            }
            '+' | '-' | '*' | '/' => {
                tokens.push(Token::Op(c));
                chars.next();
            }
            '(' => {
                tokens.push(Token::Abre);
                chars.next();
            }
            ')' => {
                tokens.push(Token::Fecha);
                chars.next();
            }
            _ => return Err(ErroExpressao::Sintaxe(s.to_string())),
        }
    }
    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    pos: usize,
    variaveis: &'a HashMap<&'a str, BigDecimal>,
}

impl Parser<'_> {
    fn proximo_op(&self, ops: &[char]) -> Option<char> {
        match self.tokens.get(self.pos) {
            Some(Token::Op(c)) if ops.contains(c) => Some(*c),
            _ => None,
        }
    }

    fn expressao(&mut self) -> Result<Numero, ErroExpressao> {
        let mut acc = self.termo()?;
        while let Some(c) = self.proximo_op(&['+', '-']) {
            self.pos += 1;
            let dir = self.termo()?;
            let op = if c == '+' { Op::Soma } else { Op::Subtracao };
            acc = operar(op, acc, dir)?;
        }
        Ok(acc)
    }

    fn termo(&mut self) -> Result<Numero, ErroExpressao> {
        let mut acc = self.unario()?;
        while let Some(c) = self.proximo_op(&['*', '/']) {
            self.pos += 1;
            let dir = self.unario()?;
            let op = if c == '*' {
                Op::Multiplicacao
            } else {
                Op::Divisao
            };
            acc = operar(op, acc, dir)?;
        }
        Ok(acc)
    }

    fn unario(&mut self) -> Result<Numero, ErroExpressao> {
        if self.proximo_op(&['-']).is_some() {
            self.pos += 1;
            return Ok(negar(self.unario()?));
        }
        self.primario()
    }

    fn primario(&mut self) -> Result<Numero, ErroExpressao> {
        let token = self.tokens.get(self.pos).cloned();
        self.pos += 1;
        match token {
            Some(Token::Numero(lit)) => literal(&lit),
            Some(Token::Ident(nome)) => match self.variaveis.get(nome.as_str()) {
                Some(v) => Ok(Numero::Decimal(v.clone())),
                None => Err(ErroExpressao::VariavelIndefinida(nome)),
            },
            Some(Token::Abre) => {
                let v = self.expressao()?;
                match self.tokens.get(self.pos) {
                    Some(Token::Fecha) => {
                        self.pos += 1;
                        Ok(v)
                    }
                    _ => Err(ErroExpressao::Sintaxe("parêntese não fechado".into())),
                }
            }
            _ => Err(ErroExpressao::Sintaxe("operando esperado".into())),
        }
    }
}

/// `NumberParser`: inteiro vira Integer/Long/BigInteger; com ponto, Double
fn literal(lit: &str) -> Result<Numero, ErroExpressao> {
    let erro = || ErroExpressao::Sintaxe(lit.to_string());
    if lit.bytes().all(|b| b.is_ascii_digit()) {
        let v: BigInt = lit.parse().map_err(|_| erro())?;
        return Ok(match (v.to_i32(), v.to_i64()) {
            (Some(i), _) => Numero::Int(i),
            (None, Some(l)) => Numero::Long(l),
            _ => Numero::Inteiro(v),
        });
    }
    lit.parse::<f64>().map(Numero::Double).map_err(|_| erro())
}

fn negar(n: Numero) -> Numero {
    match n {
        Numero::Int(i) => Numero::Int(i.wrapping_neg()),
        Numero::Long(l) => Numero::Long(l.wrapping_neg()),
        Numero::Inteiro(b) => Numero::Inteiro(-b),
        Numero::Double(d) => Numero::Double(-d),
        Numero::Decimal(d) => Numero::Decimal(d.negate()),
    }
}

fn como_long(n: &Numero) -> Option<i64> {
    match n {
        Numero::Int(i) => Some(i64::from(*i)),
        Numero::Long(l) => Some(*l),
        _ => None,
    }
}

/// `toBigDecimal` sobrescrito no Java: `new BigDecimal(n.toString())`
fn para_bigdecimal(n: Numero) -> Result<BigDecimal, ErroExpressao> {
    let texto = match n {
        Numero::Decimal(d) => return Ok(d),
        Numero::Int(i) => i.to_string(),
        Numero::Long(l) => l.to_string(),
        Numero::Inteiro(b) => b.to_string(),
        Numero::Double(d) => java_double_to_string(d),
    };
    Ok(texto.parse()?)
}

fn para_double(n: &Numero) -> f64 {
    match n {
        Numero::Int(i) => f64::from(*i),
        Numero::Long(l) => *l as f64,
        Numero::Inteiro(b) => b.to_f64().unwrap_or(f64::NAN),
        Numero::Double(d) => *d,
        Numero::Decimal(d) => d.to_f64(),
    }
}

fn para_bigint(n: &Numero) -> BigInt {
    match n {
        Numero::Int(i) => BigInt::from(*i),
        Numero::Long(l) => BigInt::from(*l),
        Numero::Inteiro(b) => b.clone(),
        Numero::Double(d) => BigInt::from(*d as i64),
        Numero::Decimal(d) => d.to_bigint(),
    }
}

fn eh_long(n: &Numero) -> bool {
    matches!(n, Numero::Long(_))
}

/// `narrowLong`: Integer quando nenhum operando é Long e o valor cabe
fn estreitar_long(esq: &Numero, dir: &Numero, v: i64) -> Numero {
    match i32::try_from(v) {
        Ok(i) if !eh_long(esq) && !eh_long(dir) => Numero::Int(i),
        _ => Numero::Long(v),
    }
}

/// `narrowBigInteger`: operandos inteiros pequenos estreitam para int/long
fn estreitar_bigint(esq: &Numero, dir: &Numero, v: BigInt) -> Numero {
    let numeravel = |n: &Numero| matches!(n, Numero::Int(_) | Numero::Long(_));
    if (numeravel(esq) || numeravel(dir))
        && let Some(l) = v.to_i64()
    {
        return match i32::try_from(l) {
            Ok(i) => Numero::Int(i),
            Err(_) => Numero::Long(l),
        };
    }
    Numero::Inteiro(v)
}

fn operar(op: Op, esq: Numero, dir: Numero) -> Result<Numero, ErroExpressao> {
    if op == Op::Divisao {
        let (l, r) = (para_bigdecimal(esq)?, para_bigdecimal(dir)?);
        if r.is_zero() {
            return Err(ErroExpressao::DivisaoPorZero);
        }
        return Ok(Numero::Decimal(l.divide_mc(&r, MathContext::DECIMAL128)?));
    }
    if let (Some(x), Some(y)) = (como_long(&esq), como_long(&dir)) {
        let exato = match op {
            Op::Soma => x.checked_add(y),
            Op::Subtracao => x.checked_sub(y),
            _ => x.checked_mul(y),
        };
        return Ok(match exato {
            Some(v) => estreitar_long(&esq, &dir, v),
            None => Numero::Inteiro(aplicar_bigint(op, BigInt::from(x), BigInt::from(y))),
        });
    }
    if matches!(esq, Numero::Decimal(_)) || matches!(dir, Numero::Decimal(_)) {
        let (l, r) = (para_bigdecimal(esq)?, para_bigdecimal(dir)?);
        let mc = MathContext::DECIMAL128;
        return Ok(Numero::Decimal(match op {
            Op::Soma => l.add_mc(&r, mc),
            Op::Subtracao => l.subtract_mc(&r, mc),
            _ => l.multiply_mc(&r, mc),
        }));
    }
    if matches!(esq, Numero::Double(_)) || matches!(dir, Numero::Double(_)) {
        let (l, r) = (para_double(&esq), para_double(&dir));
        return Ok(Numero::Double(match op {
            Op::Soma => l + r,
            Op::Subtracao => l - r,
            _ => l * r,
        }));
    }
    let v = aplicar_bigint(op, para_bigint(&esq), para_bigint(&dir));
    Ok(estreitar_bigint(&esq, &dir, v))
}

fn aplicar_bigint(op: Op, l: BigInt, r: BigInt) -> BigInt {
    match op {
        Op::Soma => l + r,
        Op::Subtracao => l - r,
        _ => l * r,
    }
}

/// `Double.toString` do Java: notação plana entre 1e-3 e 1e7, senão
/// científica `d.dddE±n`, sempre com ao menos um dígito decimal
pub fn java_double_to_string(d: f64) -> String {
    if d.is_nan() {
        return "NaN".into();
    }
    if d.is_infinite() {
        return if d > 0.0 { "Infinity" } else { "-Infinity" }.into();
    }
    if d == 0.0 {
        return if d.is_sign_negative() { "-0.0" } else { "0.0" }.into();
    }
    // `{:e}` dá os dígitos mínimos de round-trip, como o algoritmo do JDK 19+
    let cientifico = format!("{:e}", d.abs());
    let (mantissa, expoente) = cientifico.split_once('e').unwrap_or((&cientifico, "0"));
    let expoente: i32 = expoente.parse().unwrap_or(0);
    let digitos: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let sinal = if d < 0.0 { "-" } else { "" };
    let abs = d.abs();
    if (1e-3..1e7).contains(&abs) {
        let pos_ponto = expoente + 1;
        let texto = if pos_ponto <= 0 {
            format!(
                "0.{}{digitos}",
                "0".repeat(pos_ponto.unsigned_abs() as usize)
            )
        } else {
            let p = pos_ponto as usize;
            let mut inteiro = digitos.clone();
            while inteiro.len() < p {
                inteiro.push('0');
            }
            let (i, f) = inteiro.split_at(p);
            format!("{i}.{}", if f.is_empty() { "0" } else { f })
        };
        return format!("{sinal}{texto}");
    }
    let (primeiro, resto) = digitos.split_at(1);
    let resto = if resto.is_empty() { "0" } else { resto };
    format!("{sinal}{primeiro}.{resto}E{expoente}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bd(s: &str) -> BigDecimal {
        s.parse().unwrap_or_else(|_| BigDecimal::zero())
    }

    fn vars(pares: &[(&'static str, &str)]) -> HashMap<&'static str, BigDecimal> {
        pares.iter().map(|(k, v)| (*k, bd(v))).collect()
    }

    fn av(expr: &str, pares: &[(&'static str, &str)]) -> String {
        match avaliar(expr, &vars(pares)) {
            Ok(v) => v.to_string(),
            Err(e) => format!("ERR {e}"),
        }
    }

    #[test]
    fn formulas_do_banco() {
        let v = [
            ("aliquota", "0.00900000"),
            ("percentualReducao", "0.60000000"),
            ("pRedutorCompraGov", "0"),
            ("baseCalculoInformada", "200"),
            ("impostoSeletivoInformado", "0"),
            ("impostoSeletivoCalculado", "0"),
            ("pBio", "0.30000000"),
            ("quantidade", "3"),
            ("aliquotaAdRemSecundaria", "0.1922"),
            ("aliquotaAdRemPrincipal", "0.7925"),
            ("variacaoPontoPercentual", "5"),
            ("tributoCalculado", "1.80000000"),
        ];
        assert_eq!(av("baseCalculoInformada", &v), "200");
        assert_eq!(av("0", &v), "0E-8");
        assert_eq!(av("1.00", &v), "1.00000000");
        assert_eq!(av("tributoCalculado*1.00", &v), "1.80000000");
        assert_eq!(
            av(
                "aliquota*(1-percentualReducao)*(1-pRedutorCompraGov/100)",
                &v
            ),
            "0.00360000"
        );
        assert_eq!(
            av(
                "(baseCalculoInformada-impostoSeletivoInformado+impostoSeletivoCalculado)*(1-100/100)",
                &v
            ),
            "0E-8"
        );
        assert_eq!(
            av("(pBio/(1-pBio))*quantidade*aliquotaAdRemSecundaria", &v),
            "0.24711429"
        );
        assert_eq!(
            av(
                "(variacaoPontoPercentual/100)*quantidade*(pBio/(1-pBio))*aliquotaAdRemSecundaria",
                &v
            ),
            "0.01235571"
        );
        assert_eq!(av("baseCalculoInformada*0.5", &v), "100.00000000");
        assert_eq!(av("2.08 / 100", &v), "0.02080000");
    }

    /// saídas do `AvaliadorExpressaoAritmetica` original (JEXL 3.5.0) para as
    /// fórmulas do banco e casos extras com variáveis aleatórias
    #[test]
    fn identico_ao_jexl() {
        let mut falhas = Vec::new();
        for linha in include_str!("../tests/fixtures/jexl_3_5.txt").lines() {
            let Some((entrada, esperado)) = linha.split_once('#') else {
                continue;
            };
            let mut partes = entrada.split('|');
            let expr = partes.next().unwrap_or_default();
            let pares: Vec<(&str, BigDecimal)> = partes
                .filter_map(|kv| kv.split_once('='))
                .map(|(k, v)| (k, bd(v)))
                .collect();
            let mapa: HashMap<&str, BigDecimal> = pares.into_iter().collect();
            let obtido = match avaliar(expr, &mapa) {
                Ok(v) => v.to_string(),
                Err(_) => "ERR JexlException".into(),
            };
            if obtido != esperado {
                falhas.push(format!("{expr} => java={esperado} rust={obtido}"));
            }
        }
        assert!(
            falhas.is_empty(),
            "{} falhas:\n{}",
            falhas.len(),
            falhas[..falhas.len().min(20)].join("\n")
        );
    }

    #[test]
    fn erros() {
        assert_eq!(av("1/0", &[]), "ERR Division by zero");
        assert_eq!(av("x*2", &[]), "ERR undefined variable x");
    }

    #[test]
    fn double_como_java() {
        for (d, s) in [
            (1.0, "1.0"),
            (2.08, "2.08"),
            (100.0, "100.0"),
            (0.001, "0.001"),
            (0.0001, "1.0E-4"),
            (1e7, "1.0E7"),
            (1234567.0, "1234567.0"),
            (-0.5, "-0.5"),
            (1.5e-10, "1.5E-10"),
            (123456789.0, "1.23456789E8"),
        ] {
            assert_eq!(java_double_to_string(d), s);
        }
    }
}
