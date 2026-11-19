//! mesmas normalizações do `gravar.py`/`comparar.py`: cabeçalhos guardados,
//! data de modificação, boundary e timestamp, e as ordens aleatórias do Java

use std::collections::HashMap;

use serde_json::Value;

use super::http::Obtida;
use super::{Ambiente, Caso};

const GUARDADOS: [&str; 23] = [
    "content-type",
    "vary",
    "x-calc-app-version",
    "x-calc-db-version",
    "x-warning-dados-simulados",
    "content-disposition",
    "content-encoding",
    "location",
    "allow",
    "access-control-allow-origin",
    "access-control-allow-methods",
    "access-control-allow-headers",
    "x-content-type-options",
    "x-xss-protection",
    "cache-control",
    "pragma",
    "expires",
    "x-frame-options",
    "transfer-encoding",
    "content-length",
    "content-range",
    "accept-ranges",
    "last-modified",
];
const MARCA_DATA: &str = "<lm>";
const MARCA_BOUNDARY: &str = "<b>";
const CAMPO_TIMESTAMP: &str = "\"timestamp\":\"";
const PREFIXO_VALIDACAO: &str = "Um ou mais campos são inválidos - ";
const PREFIXO_AVISO: &str = "Campo ";
const TRECHO_DIFERENCA: usize = 120;

fn boundary(cabecalhos: &[(String, String)]) -> Option<String> {
    cabecalhos.iter().find_map(|(_, v)| {
        v.split_once("boundary=")
            .map(|(_, b)| b.split_whitespace().next().unwrap_or_default().to_string())
    })
}

fn cabecalhos(obtida: &Obtida) -> Vec<(String, String)> {
    let b = boundary(&obtida.cabecalhos);
    obtida
        .cabecalhos
        .iter()
        .filter(|(k, _)| GUARDADOS.contains(&k.as_str()))
        .map(|(k, v)| {
            let v = match (&b, k.as_str()) {
                (_, "last-modified") => MARCA_DATA.to_string(),
                (Some(b), _) => v.replace(b.as_str(), MARCA_BOUNDARY),
                _ => v.clone(),
            };
            (k.clone(), v)
        })
        .collect()
}

fn sem_timestamp(texto: &str) -> String {
    let mut saida = String::with_capacity(texto.len());
    let mut resto = texto;
    while let Some(i) = resto.find(CAMPO_TIMESTAMP) {
        let inicio = i + CAMPO_TIMESTAMP.len();
        saida.push_str(&resto[..inicio]);
        saida.push_str("<ts>");
        let fim = resto[inicio..]
            .find('"')
            .map_or(resto.len(), |f| inicio + f);
        resto = &resto[fim..];
    }
    saida.push_str(resto);
    saida
}

fn corpo(obtida: &Obtida) -> String {
    let texto = String::from_utf8_lossy(&obtida.corpo);
    let texto = match boundary(&obtida.cabecalhos) {
        Some(b) => texto.replace(&b, MARCA_BOUNDARY),
        None => texto.into_owned(),
    };
    sem_timestamp(&texto)
}

/// violações do Bean Validation saem na ordem de um `HashSet`
fn validacao_ordenada(detalhe: &str) -> String {
    let Some(itens) = detalhe.strip_prefix(PREFIXO_VALIDACAO) else {
        return detalhe.to_string();
    };
    let miolo = itens
        .trim_end()
        .trim_start_matches('[')
        .trim_end_matches(']');
    let mut lista: Vec<&str> = miolo.split("] [").collect();
    lista.sort_unstable();
    let ordenados: String = lista.iter().map(|i| format!("[{i}] ")).collect();
    format!("{PREFIXO_VALIDACAO}{ordenados}")
}

fn normalizar(v: Value) -> Value {
    match v {
        Value::Object(m) => {
            let mut m: serde_json::Map<String, Value> =
                m.into_iter().map(|(k, v)| (k, normalizar(v))).collect();
            // elementoBaseCalculo.todos() é um Set.of
            if let Some(Value::Array(avisos)) = m.get_mut("avisos")
                && avisos
                    .iter()
                    .all(|a| a.as_str().is_some_and(|s| s.starts_with(PREFIXO_AVISO)))
            {
                avisos.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            }
            if let Some(Value::String(d)) = m.get_mut("detail") {
                *d = validacao_ordenada(d);
            }
            Value::Object(m)
        }
        Value::Array(l) => {
            let mut l: Vec<Value> = l.into_iter().map(normalizar).collect();
            if !l.is_empty() && l.iter().all(|v| v.get("pointer").is_some()) {
                l.sort_by_key(|v| v.to_string());
            }
            Value::Array(l)
        }
        outro => outro,
    }
}

/// só quando os bytes diferem: mesmo tamanho e iguais depois de ordenar o que
/// o Java devolve em ordem aleatória
fn equivalentes(esperado: &str, obtido: &str) -> bool {
    if esperado.len() != obtido.len() {
        return false;
    }
    match (
        serde_json::from_str::<Value>(esperado),
        serde_json::from_str::<Value>(obtido),
    ) {
        (Ok(a), Ok(b)) => normalizar(a) == normalizar(b),
        _ => false,
    }
}

fn contagem(lista: &[(String, String)]) -> HashMap<&(String, String), usize> {
    let mut m = HashMap::new();
    for par in lista {
        *m.entry(par).or_default() += 1;
    }
    m
}

fn diferenca(esperado: &str, obtido: &str) -> String {
    let i = esperado
        .chars()
        .zip(obtido.chars())
        .take_while(|(a, b)| a == b)
        .count();
    let trecho = |s: &str| {
        s.chars()
            .skip(i.saturating_sub(40))
            .take(TRECHO_DIFERENCA)
            .collect::<String>()
    };
    format!(
        "corpo difere no caractere {i}\n    java: {:?}\n    rust: {:?}",
        trecho(esperado),
        trecho(obtido)
    )
}

fn comparar_corpo(ambiente: &Ambiente, caso: &Caso, obtido: &str) -> Result<(), String> {
    let e = &caso.esperado;
    if let Some(esperado) = &e.corpo {
        let esperado = ambiente.local(esperado);
        return if esperado == obtido || equivalentes(&esperado, obtido) {
            Ok(())
        } else {
            Err(diferenca(&esperado, obtido))
        };
    }
    let obtido = ambiente.gravado(obtido);
    let crc = crc32fast::hash(obtido.as_bytes());
    if Some(crc) == e.corpo_crc32 && Some(obtido.len()) == e.corpo_tamanho {
        Ok(())
    } else {
        Err(format!(
            "corpo grande difere: crc32 {crc} tamanho {} (esperado {:?} {:?})",
            obtido.len(),
            e.corpo_crc32,
            e.corpo_tamanho
        ))
    }
}

pub fn comparar(
    ambiente: &Ambiente,
    caso: &Caso,
    obtida: &Obtida,
    ignorar_corpo: bool,
) -> Result<(), String> {
    let e = &caso.esperado;
    if e.status != obtida.status {
        return Err(format!("status {} != {}", e.status, obtida.status));
    }
    let esperados: Vec<(String, String)> = e
        .cabecalhos
        .iter()
        .map(|(k, v)| (k.clone(), ambiente.local(v)))
        .collect();
    let mut obtidos = cabecalhos(obtida);
    let mut esperados = esperados;
    // corpo de tamanho diferente por desvio aceito
    if ignorar_corpo {
        esperados.retain(|(k, _)| k != "content-length");
        obtidos.retain(|(k, _)| k != "content-length");
    }
    if contagem(&esperados) != contagem(&obtidos) {
        return Err(format!(
            "cabeçalhos\n    java: {esperados:?}\n    rust: {obtidos:?}"
        ));
    }
    if ignorar_corpo {
        return Ok(());
    }
    comparar_corpo(ambiente, caso, &corpo(obtida))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordena_violacoes() {
        assert_eq!(
            validacao_ordenada(&format!("{PREFIXO_VALIDACAO}[b: x] [a[0].c: y] ")),
            format!("{PREFIXO_VALIDACAO}[a[0].c: y] [b: x] ")
        );
    }
}
