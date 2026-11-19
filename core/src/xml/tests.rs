use std::fs;
use std::path::Path;

use super::*;

#[test]
fn todos_os_esquemas_carregam() {
    for tipo in TipoDocumento::TODOS {
        for subtipo in [TipoXml::Grupo, TipoXml::Nota] {
            if let Err(e) = esquema(tipo, subtipo) {
                panic!("{tipo:?}/{subtipo:?}: {e}");
            }
        }
    }
}

/// amostras válidas da suíte Java (`tests/fixtures/xml/{tipo}__{subtipo}__{nome}`)
#[test]
fn amostras_validas_passam() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/xml");
    let mut quantas = 0;
    for entrada in fs::read_dir(dir).into_iter().flatten().flatten() {
        let nome = entrada.file_name().to_string_lossy().into_owned();
        let partes: Vec<&str> = nome.split("__").collect();
        let (Some(tipo), Some(subtipo)) = (
            partes.first().and_then(|t| TipoDocumento::por_mnemonico(t)),
            partes.get(1).and_then(|s| TipoXml::por_nome(s)),
        ) else {
            continue;
        };
        let xml = fs::read_to_string(entrada.path()).unwrap_or_default();
        assert_eq!(
            validar(&xml, tipo, subtipo).map_err(|e| e.to_string()),
            Ok(()),
            "{nome}"
        );
        quantas += 1;
    }
    assert_eq!(quantas, 17);
}
