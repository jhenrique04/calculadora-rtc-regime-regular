//! embute os XSDs de `recursos/xml` (o validador não lê disco em runtime)

use std::env;
use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const DIRETORIO: &str = "recursos/xml";

fn coletar(dir: &Path, saida: &mut Vec<PathBuf>) -> io::Result<()> {
    for entrada in fs::read_dir(dir)? {
        let caminho = entrada?.path();
        if caminho.is_dir() {
            coletar(&caminho, saida)?;
        } else if caminho.extension().is_some_and(|e| e == "xsd") {
            saida.push(caminho);
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let manifesto = PathBuf::from(env::var("CARGO_MANIFEST_DIR").map_err(io::Error::other)?);
    let raiz = manifesto.join(DIRETORIO);
    println!("cargo:rerun-if-changed={DIRETORIO}");
    let mut arquivos = Vec::new();
    coletar(&raiz, &mut arquivos)?;
    arquivos.sort();
    let mut codigo = String::from("pub static ARQUIVOS: &[(&str, &[u8])] = &[\n");
    for caminho in &arquivos {
        let relativo = caminho.strip_prefix(&raiz).map_err(io::Error::other)?;
        let nome = relativo.to_string_lossy().replace('\\', "/");
        writeln!(
            codigo,
            "    ({nome:?}, include_bytes!({:?})),",
            caminho.display()
        )
        .map_err(io::Error::other)?;
    }
    codigo.push_str("];\n");
    let destino =
        PathBuf::from(env::var("OUT_DIR").map_err(io::Error::other)?).join("xsd_embutidos.rs");
    fs::write(destino, codigo)
}
