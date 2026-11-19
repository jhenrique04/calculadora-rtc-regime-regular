use super::*;

#[derive(Debug, Default, PartialEq)]
struct Item {
    numero: Option<i32>,
    cst: Option<String>,
    base: Option<BigDecimal>,
}

objeto_json!(Item, "x.Item", {
    "numero" => numero: i32,
    "cst" => cst: String,
    "baseCalculo" => base: BigDecimal,
});

#[derive(Debug, Default, PartialEq)]
struct Raiz {
    municipio: Option<i64>,
    dh: Option<DataHora>,
    itens: Option<Vec<Option<Item>>>,
}

objeto_json!(Raiz, "x.Raiz", {
    "municipio" => municipio: i64,
    "dhFatoGerador" => dh: DataHora,
    "itens" => itens: Vec<Option<Item>>,
});

fn erro(json: &str) -> String {
    match ler_documento::<Raiz>(json.as_bytes()) {
        Err(e) => e.mensagem,
        Ok(v) => panic!("esperava erro, veio {v:?}"),
    }
}

fn ok(json: &str) -> Raiz {
    match ler_documento::<Raiz>(json.as_bytes()) {
        Ok(Some(v)) => v,
        outro => panic!("esperava objeto, veio {outro:?}"),
    }
}

#[test]
fn mensagens_de_sintaxe_iguais_ao_jackson() {
    let loc = "[Source: REDACTED (`StreamReadFeature.INCLUDE_SOURCE_IN_LOCATION` disabled); line: 1, column: 1]";
    let casos = [
        ("{", format!("Unexpected end-of-input: expected close marker for Object (start marker at {loc})")),
        ("{'id':1}", "Unexpected character (''' (code 39)): was expecting double-quote to start field name".into()),
        ("{\"a\":1,}", "Unexpected character ('}' (code 125)): was expecting double-quote to start field name".into()),
        ("{\"municipio\": NaN}", "Non-standard token 'NaN': enable `JsonReadFeature.ALLOW_NON_NUMERIC_NUMBERS` to allow".into()),
        ("{\"municipio\": 01}", "Invalid numeric value: Leading zeroes not allowed".into()),
        ("{\"municipio\": -,", "Unexpected character (',' (code 44)) in numeric value: expected digit (0-9) to follow minus sign, for valid numeric value".into()),
        ("{\"municipio\": 1.,", "Unexpected character (',' (code 44)) in numeric value: Decimal point not followed by a digit".into()),
        ("{\"municipio\": .5}", "Unexpected character ('.' (code 46)): expected a valid value (JSON String, Number, Array, Object or token 'null', 'true' or 'false')".into()),
        ("{\"municipio\": +1}", "Unexpected character ('+' (code 43)) in numeric value: JSON spec does not allow numbers to have plus signs: enable `JsonReadFeature.ALLOW_LEADING_PLUS_SIGN_FOR_NUMBERS` to allow".into()),
        ("{\"municipio\": 1e,", "Unexpected character (',' (code 44)) in numeric value: Exponent indicator not followed by a digit".into()),
        ("{\"municipio\": 1a}", "Unexpected character ('a' (code 97)): was expecting comma to separate Object entries".into()),
        ("{\"municipio\": -Infinity}", "Non-standard token '-Infinity': enable `JsonReadFeature.ALLOW_NON_NUMERIC_NUMBERS` to allow".into()),
        ("{\"municipio\": truex}", "Unrecognized token 'truex': was expecting (JSON String, Number, Array, Object or token 'null', 'true' or 'false')".into()),
        ("{\"municipio\": tru,", "Unrecognized token 'tru': was expecting (JSON String, Number, Array, Object or token 'null', 'true' or 'false')".into()),
        ("{\"municipio\" 1}", "Unexpected character ('1' (code 49)): was expecting a colon to separate field name and value".into()),
        ("{\"itens\":[{},]}", "Unexpected character (']' (code 93)): expected a value".into()),
        ("{\"id\":\"x\"]", format!("Unexpected close marker ']': expected '}}' (for Object starting at {loc})")),
        ("{\"id\": \"abc", "Unexpected end-of-input in VALUE_STRING".into()),
        ("{\"id\": \"a\\qb\"}", "Unrecognized character escape 'q' (code 113)".into()),
        ("{\"id\": \"\\u12\"}", "Unexpected character ('\"' (code 34)): expected a hex-digit for character escape sequence".into()),
        ("   ", "No content to map due to end-of-input".into()),
        ("{\"id\"", "Unexpected end-of-input within/between Object entries".into()),
        ("{\"id\":\"a\tb\"}", "Illegal unquoted character ((CTRL-CHAR, code 9)): has to be escaped using backslash to be included in string value".into()),
        ("{/*c*/}", "Unexpected character ('/' (code 47)): maybe a (non-standard) comment? (not recognized as one since Feature 'ALLOW_COMMENTS' not enabled for parser)".into()),
    ];
    for (entrada, esperado) in casos {
        assert_eq!(erro(entrada), esperado, "entrada {entrada}");
    }
    assert_eq!(
        erro("{\"itens\":["),
        "Unexpected end-of-input: expected close marker for Array (start marker at [Source: REDACTED (`StreamReadFeature.INCLUDE_SOURCE_IN_LOCATION` disabled); line: 1, column: 10])"
    );
    match ler_documento::<Raiz>(b"{\"id\":\"\xff\"}") {
        Err(e) => assert_eq!(e.mensagem, "Invalid UTF-8 start byte 0xff"),
        outro => panic!("{outro:?}"),
    }
}

#[test]
fn mensagens_de_vinculo_iguais_ao_jackson() {
    let casos = [
        (
            "[]",
            "Cannot deserialize value of type `x.Raiz` from Array value (token `JsonToken.START_ARRAY`)",
        ),
        (
            "1",
            "Cannot construct instance of `x.Raiz` (although at least one Creator exists): no int/Int-argument constructor/factory method to deserialize from Number value (1)",
        ),
        (
            "\"x\"",
            "Cannot construct instance of `x.Raiz` (although at least one Creator exists): no String-argument constructor/factory method to deserialize from String value ('x')",
        ),
        (
            "{\"itens\":[{\"numero\":\"abc\"}]}",
            "Cannot deserialize value of type `java.lang.Integer` from String \"abc\": not a valid `java.lang.Integer` value",
        ),
        (
            "{\"itens\":[{\"numero\":true}]}",
            "Cannot deserialize value of type `java.lang.Integer` from Boolean value (token `JsonToken.VALUE_TRUE`)",
        ),
        (
            "{\"itens\":[{\"numero\":9999999999}]}",
            "Numeric value (9999999999) out of range of int (-2147483648 - 2147483647)",
        ),
        (
            "{\"itens\":[{\"numero\":1e20}]}",
            "Numeric value (1e20) out of range of int (-2147483648 - 2147483647)",
        ),
        (
            "{\"itens\":[{\"numero\":\"9999999999\"}]}",
            "Cannot deserialize value of type `java.lang.Integer` from String \"9999999999\": Overflow: numeric value (9999999999) out of range of `java.lang.Integer` (-2147483648 -2147483647)",
        ),
        (
            "{\"municipio\": 99999999999999999999}",
            "Numeric value (99999999999999999999) out of range of long (-9223372036854775808 - 9223372036854775807)",
        ),
        (
            "{\"municipio\": \"4314902.7\"}",
            "Cannot deserialize value of type `java.lang.Long` from String \"4314902.7\": not a valid `java.lang.Long` value",
        ),
        (
            "{\"itens\":[{\"baseCalculo\":\"NaN\"}]}",
            "Cannot deserialize value of type `java.math.BigDecimal` from String \"NaN\": not a valid representation",
        ),
        (
            "{\"itens\":[{\"cst\":{}}]}",
            "Cannot deserialize value of type `java.lang.String` from Object value (token `JsonToken.START_OBJECT`)",
        ),
        (
            "{\"itens\":{}}",
            "Cannot deserialize value of type `java.util.ArrayList<x.Item>` from Object value (token `JsonToken.START_OBJECT`)",
        ),
        (
            "{\"itens\":[1]}",
            "Cannot construct instance of `x.Item` (although at least one Creator exists): no int/Int-argument constructor/factory method to deserialize from Number value (1)",
        ),
        (
            "{\"dhFatoGerador\":{}}",
            "Campo 'dhFatoGerador' [{] inválido. Formato esperado: 'yyyy-MM-dd'T'HH:mm:ssXXX'. Exemplo de valor: '2026-01-05T13:45:00-03:00 ou 2026-05-22T10:57:23Z'",
        ),
        (
            "{\"dhFatoGerador\":\"2026-02-30T09:50:05-03:00\"}",
            "Campo 'dhFatoGerador' [2026-02-30T09:50:05-03:00] inválido. Formato esperado: 'yyyy-MM-dd'T'HH:mm:ssXXX'. Exemplo de valor: '2026-01-05T13:45:00-03:00 ou 2026-05-22T10:57:23Z'",
        ),
        (
            "{\"dhFatoGerador\":\"2026-01-01T09:50:05+19:00\"}",
            "Campo 'dhFatoGerador' [2026-01-01T09:50:05+19:00] inválido. Formato esperado: 'yyyy-MM-dd'T'HH:mm:ssXXX'. Exemplo de valor: '2026-01-05T13:45:00-03:00 ou 2026-05-22T10:57:23Z'",
        ),
    ];
    for (entrada, esperado) in casos {
        assert_eq!(erro(entrada), esperado, "entrada {entrada}");
    }
}

#[test]
fn coercoes_aceitas_pelo_jackson() {
    let r = ok(
        "\u{feff}{\"municipio\": \" 4314902 \", \"x\": [1, {\"a\": []}], \"itens\": [{\"numero\": 1.9, \"cst\": 0, \"baseCalculo\": \" 2e2 \"}, {\"numero\": \"null\", \"cst\": true, \"baseCalculo\": \"\"}, null]} lixo",
    );
    assert_eq!(r.municipio, Some(4314902));
    let itens = r.itens.unwrap_or_default();
    assert_eq!(itens.len(), 3);
    let primeiro = itens[0].as_ref().map(|i| {
        (
            i.numero,
            i.cst.clone(),
            i.base.as_ref().map(ToString::to_string),
        )
    });
    assert_eq!(
        primeiro,
        Some((Some(1), Some("0".into()), Some("2E+2".into())))
    );
    let segundo = itens[1]
        .as_ref()
        .map(|i| (i.numero, i.cst.clone(), i.base.is_none()));
    assert_eq!(segundo, Some((None, Some("true".into()), true)));
    assert!(itens[2].is_none());
    assert_eq!(ok("{\"municipio\":1,\"municipio\":2}").municipio, Some(2));
    assert!(matches!(ler_documento::<Raiz>(b"null"), Ok(None)));
    let dh = ok("{\"dhFatoGerador\":\"2026-12-31T23:00:00-03:00\"}")
        .dh
        .map(|d| d.data().to_string());
    assert_eq!(dh.as_deref(), Some("2026-12-31"));
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Ente {
    Uniao,
    Estado,
}

#[test]
fn enum_por_codigo() {
    let valores = [("1", Ente::Uniao), ("2", Ente::Estado)];
    let ler = |json: &str| {
        let mut l = Leitor::novo(json.as_bytes());
        l.proximo()
            .and_then(|_| ler_enum_codigo(&mut l, "x.Ente", &valores))
    };
    assert_eq!(ler("2"), Ok(Some(Ente::Estado)));
    assert_eq!(ler("\" 1\""), Ok(Some(Ente::Uniao)));
    assert_eq!(
        ler("0").map_err(|e| e.mensagem),
        Err("Cannot deserialize value of type `x.Ente` from String \"0\": not one of the values accepted for Enum class: [1, 2]".into())
    );
    assert_eq!(
        ler("\"  \"").map_err(|e| e.mensagem),
        Err("Cannot coerce empty String (\"\") to `x.Ente` value (but could if coercion was enabled using `CoercionConfig`)".into())
    );
    assert_eq!(
        ler("1.0").map_err(|e| e.mensagem),
        Err("Cannot deserialize value of type `x.Ente` from Floating-point value (token `JsonToken.VALUE_NUMBER_FLOAT`)".into())
    );
}

#[test]
fn escrita_compacta_com_escapes() {
    let mut e = Escritor::new();
    e.inicio_objeto();
    let r = e
        .texto("a", &Some("x\"\u{1}ç".into()))
        .and_then(|_| {
            e.moeda(
                "v",
                &Some("1.005".parse().unwrap_or_else(|_| BigDecimal::zero())),
            )
        })
        .and_then(|_| e.inteiro::<i32>("n", &None))
        .and_then(|_| e.lista("l", &Some(vec![String::from("p")])));
    assert!(r.is_ok());
    e.fim_objeto();
    assert_eq!(
        e.finalizar(),
        "{\"a\":\"x\\\"\\u0001ç\",\"v\":\"1.00\",\"l\":[\"p\"]}"
    );
}
