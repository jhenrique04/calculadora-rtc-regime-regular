//! paridade com a versão Java: reenvia os pedidos gravados contra o container
//! de referência (ferramentas/paridade) ao `rtc-server` real, com um banco
//! gerado pelas mesmas migrações, e compara status, cabeçalhos e corpo

mod paridade_apoio;

use std::path::PathBuf;

use paridade_apoio::{Ambiente, Suite, comparar, enviar};

const DIRETORIO_FIXTURES: &str = "tests/fixtures/paridade";

/// diferenças conhecidas e aceitas, com o motivo: corpo e tamanho não são comparados
const DESVIOS: &[(&str, &str)] = &[
    (
        "xg2/109/texto-controle",
        "surrogate solto no JSON: JAXB grava '?', Rust lê como U+FFFD",
    ),
    (
        "sw/gz/api/api-docs",
        "gzip contém a porta do servidor de gravação",
    ),
    (
        "sw/gz/api/api-docs/swagger-config",
        "gzip contém a porta do servidor de gravação",
    ),
];

fn suites() -> Vec<(String, Suite)> {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DIRETORIO_FIXTURES);
    let mut arquivos: Vec<PathBuf> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.to_string_lossy().ends_with(".json.gz"))
        .collect();
    arquivos.sort();
    arquivos
        .into_iter()
        .map(|p| {
            let nome = p
                .file_name()
                .map(|n| n.to_string_lossy().trim_end_matches(".json.gz").to_string())
                .unwrap_or_default();
            (nome, Suite::carregar(&p))
        })
        .collect()
}

#[test]
fn respostas_iguais_as_da_versao_java() {
    let ambiente = Ambiente::subir();
    let mut falhas = Vec::new();
    let mut total = 0;
    for (nome, suite) in suites() {
        for caso in &suite.casos {
            total += 1;
            let desvio = DESVIOS.iter().find(|(id, _)| *id == caso.id).map(|d| d.1);
            let obtida = enviar(&ambiente, caso, suite.seguir_redirecionamento);
            if let Err(e) = comparar(&ambiente, caso, &obtida, desvio.is_some()) {
                falhas.push(format!("[{nome}] {}: {e}", caso.id));
            }
        }
    }
    for f in &falhas {
        eprintln!("{f}");
    }
    assert!(
        falhas.is_empty(),
        "{} de {total} casos diferentes da versão Java",
        falhas.len()
    );
}
