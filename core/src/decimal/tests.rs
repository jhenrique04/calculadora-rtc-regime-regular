use super::*;

fn bd(s: &str) -> BigDecimal {
    match s.parse() {
        Ok(v) => v,
        Err(e) => panic!("{s}: {e}"),
    }
}

fn modo(s: &str) -> RoundingMode {
    match s {
        "HALF_EVEN" => RoundingMode::HalfEven,
        "HALF_UP" => RoundingMode::HalfUp,
        "HALF_DOWN" => RoundingMode::HalfDown,
        "UP" => RoundingMode::Up,
        "DOWN" => RoundingMode::Down,
        "CEILING" => RoundingMode::Ceiling,
        "FLOOR" => RoundingMode::Floor,
        _ => panic!("modo {s}"),
    }
}

fn texto(r: Result<BigDecimal, ErroDecimal>) -> String {
    match r {
        Ok(v) => v.to_string(),
        Err(_) => "ERR".into(),
    }
}

fn executar(linha: &str) -> String {
    let p: Vec<&str> = linha.split(' ').collect();
    let x = bd(p[1]);
    let y = || bd(p[2]);
    let int = |i: usize| p[i].parse::<i64>().unwrap_or_default();
    match p[0] {
        "add" => x.add(&y()).to_string(),
        "sub" => x.subtract(&y()).to_string(),
        "mul" => x.multiply(&y()).to_string(),
        "addmc" => x.add_mc(&y(), MathContext::DECIMAL128).to_string(),
        "submc" => x.subtract_mc(&y(), MathContext::DECIMAL128).to_string(),
        "mulmc" => x.multiply_mc(&y(), MathContext::DECIMAL128).to_string(),
        "divmc" => texto(x.divide_mc(&y(), MathContext::DECIMAL128)),
        "divmc64" => texto(x.divide_mc(&y(), MathContext::DECIMAL64)),
        "div8" => texto(x.divide_scale(&y(), 8, modo(p[3]))),
        "scale" => texto(x.set_scale(int(2), modo(p[3]))),
        "plain" => x.to_plain_string(),
        "str" => x.to_string(),
        "strip" => x.strip_trailing_zeros().to_string(),
        "mpr" => x.move_point_right(int(2)).to_string(),
        "cmp" => (x.compare_to(&y()) as i8).to_string(),
        "prec" => x.precision().to_string(),
        "bi" => x.to_bigint().to_string(),
        op => panic!("op {op}"),
    }
}

/// casos aleatórios e dirigidos executados no JDK 21 (`java.math.BigDecimal`)
#[test]
fn identico_ao_jdk() {
    let mut falhas = Vec::new();
    for linha in include_str!("../../tests/fixtures/bigdecimal_jdk21.txt").lines() {
        let Some((caso, esperado)) = linha.split_once('|') else {
            continue;
        };
        let esperado = if esperado.starts_with("ERR") {
            "ERR"
        } else {
            esperado
        };
        let obtido = executar(caso);
        if obtido != esperado {
            falhas.push(format!("{caso}: java={esperado} rust={obtido}"));
        }
    }
    assert!(
        falhas.is_empty(),
        "{} falhas:\n{}",
        falhas.len(),
        falhas[..falhas.len().min(30)].join("\n")
    );
}

#[test]
fn igualdade_considera_escala() {
    assert_ne!(bd("1.0"), bd("1.00"));
    assert_eq!(bd("1.0").cmp(&bd("1.00")), Ordering::Equal);
    assert_eq!(bd("12.30").long_value_exact(), None);
    assert_eq!(bd("12.00").long_value_exact(), Some(12));
}
