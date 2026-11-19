//! varredura de `flyway.locations`: nomes `V<versão>__<descrição>.sql`,
//! `B<versão>__<descrição>.sql` e os callbacks pelo nome do evento

use std::fs;
use std::path::{Path, PathBuf};

use super::{ErroBanco, R};

const SEPARADOR: &str = "__";
const SUFIXO: &str = ".sql";
const ANTES: &str = "beforeMigrate.sql";
const DEPOIS: &str = "afterMigrate.sql";
const BOM: char = '\u{feff}';

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tipo {
    Baseline,
    Versionada,
}

impl Tipo {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Baseline => "SQL_BASELINE",
            Self::Versionada => "SQL",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Migracao {
    /// como escrita no nome do arquivo (`0002`), que é o que vai para o histórico
    pub versao: String,
    /// partes numéricas, para ordenar como `MigrationVersion`
    pub numero: Vec<u64>,
    pub descricao: String,
    pub tipo: Tipo,
    /// caminho relativo à location, com `/`
    pub script: String,
    pub conteudo: String,
    pub checksum: i32,
}

#[derive(Debug, Clone)]
pub struct Callback {
    pub script: String,
    pub conteudo: String,
}

pub struct Scripts {
    pub migracoes: Vec<Migracao>,
    pub antes: Option<Callback>,
    pub depois: Option<Callback>,
}

/// `ChecksumCalculator`: CRC32 de cada linha sem a quebra, BOM descartado
pub fn checksum(conteudo: &str) -> i32 {
    let mut crc = crc32fast::Hasher::new();
    let conteudo = conteudo.strip_prefix(BOM).unwrap_or(conteudo);
    for linha in linhas(conteudo) {
        crc.update(linha.as_bytes());
    }
    // o Java guarda o valor sem sinal truncado para int
    crc.finalize() as i32
}

/// linhas como o `BufferedReader.readLine`: `\n`, `\r` ou `\r\n`
fn linhas(texto: &str) -> impl Iterator<Item = &str> {
    let mut resto = texto;
    std::iter::from_fn(move || {
        if resto.is_empty() {
            return None;
        }
        let fim = resto.find(['\r', '\n']).unwrap_or(resto.len());
        let linha = &resto[..fim];
        let quebra = if resto[fim..].starts_with("\r\n") {
            2
        } else {
            usize::from(fim < resto.len())
        };
        resto = &resto[fim + quebra..];
        Some(linha)
    })
}

fn versao(texto: &str, arquivo: &str) -> R<Vec<u64>> {
    texto
        .split(['.', '_'])
        .map(|p| p.parse().map_err(|_| ErroBanco::Nome(arquivo.to_string())))
        .collect()
}

fn migracao(nome: &str, script: String, conteudo: String) -> R<Option<Migracao>> {
    let tipo = match nome.chars().next() {
        Some('V') => Tipo::Versionada,
        Some('B') => Tipo::Baseline,
        _ => return Ok(None),
    };
    let Some(sem_sufixo) = nome.strip_suffix(SUFIXO) else {
        return Ok(None);
    };
    let Some((v, descricao)) = sem_sufixo[1..].split_once(SEPARADOR) else {
        return Err(ErroBanco::Nome(nome.to_string()));
    };
    Ok(Some(Migracao {
        versao: v.replace('_', "."),
        numero: versao(v, nome)?,
        descricao: descricao.replace('_', " "),
        tipo,
        script,
        checksum: checksum(&conteudo),
        conteudo,
    }))
}

fn arquivos(dir: &Path, saida: &mut Vec<PathBuf>) -> R<()> {
    let ler = |fonte| ErroBanco::Leitura {
        caminho: dir.to_path_buf(),
        fonte,
    };
    for entrada in fs::read_dir(dir).map_err(ler)? {
        let caminho = entrada.map_err(ler)?.path();
        if caminho.is_dir() {
            arquivos(&caminho, saida)?;
        } else if caminho.extension().is_some_and(|e| e == "sql") {
            saida.push(caminho);
        }
    }
    Ok(())
}

fn relativo(raiz: &Path, caminho: &Path) -> String {
    let rel = caminho.strip_prefix(raiz).unwrap_or(caminho);
    rel.components()
        .map(|c| c.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

pub fn descobrir(raiz: &Path) -> R<Scripts> {
    let mut caminhos = Vec::new();
    arquivos(raiz, &mut caminhos)?;
    let mut scripts = Scripts {
        migracoes: Vec::new(),
        antes: None,
        depois: None,
    };
    for caminho in caminhos {
        let conteudo = fs::read_to_string(&caminho).map_err(|fonte| ErroBanco::Leitura {
            caminho: caminho.clone(),
            fonte,
        })?;
        let script = relativo(raiz, &caminho);
        let nome = caminho
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        match nome.as_str() {
            ANTES => scripts.antes = Some(Callback { script, conteudo }),
            DEPOIS => scripts.depois = Some(Callback { script, conteudo }),
            _ => scripts.migracoes.extend(migracao(&nome, script, conteudo)?),
        }
    }
    scripts.migracoes.sort_by(|a, b| a.numero.cmp(&b.numero));
    let repetida = scripts
        .migracoes
        .windows(2)
        .find(|p| p[0].numero == p[1].numero && p[0].tipo == p[1].tipo);
    if let Some(p) = repetida {
        return Err(ErroBanco::VersaoDuplicada(p[0].versao.clone()));
    }
    Ok(scripts)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linhas_como_buffered_reader() {
        let l: Vec<_> = linhas("a\r\nb\rc\n\nd").collect();
        assert_eq!(l, ["a", "b", "c", "", "d"]);
        assert_eq!(linhas("a\n").count(), 1);
    }

    #[test]
    fn checksum_ignora_quebras_e_bom() {
        assert_eq!(checksum("x\ny\n"), checksum("\u{feff}x\r\ny"));
        assert_ne!(checksum("x\ny"), checksum("x\nz"));
    }

    #[test]
    fn nomes_de_migracao() -> R<()> {
        let m = migracao(
            "V0002__ajuste_campo.sql",
            "m/V0002__ajuste_campo.sql".into(),
            String::new(),
        )?;
        let m = m.ok_or_else(|| ErroBanco::Nome("V0002".into()))?;
        assert_eq!((m.versao.as_str(), m.numero.as_slice()), ("0002", &[2][..]));
        assert_eq!(m.descricao, "ajuste campo");
        assert_eq!(m.tipo.nome(), "SQL");
        assert!(migracao("README.sql", String::new(), String::new())?.is_none());
        assert!(migracao("V1_semseparador.sql", String::new(), String::new()).is_err());
        Ok(())
    }
}
