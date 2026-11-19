//! `DownloadService`: resolve a URL de download por plataforma

use crate::erro::{Classe, Falha, Resultado};
use crate::saida_json;

/// plataforma encontrada (nome como configurado) e sua URL
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolucaoDownload<'a> {
    pub plataforma: &'a str,
    pub url: &'a str,
}

#[derive(Debug, Clone)]
pub struct DownloadUrlSaida {
    pub download_url: Option<String>,
}

saida_json!(DownloadUrlSaida {
    "downloadUrl" => download_url: texto,
});

fn nao_encontrada(plataforma: Option<&str>, disponiveis: &[(String, String)]) -> Falha {
    let nomes: Vec<&str> = disponiveis.iter().map(|(n, _)| n.as_str()).collect();
    Falha::new(
        Classe::PlataformaDownloadNaoEncontrada,
        format!(
            "Plataforma de download '{}' não encontrada. Plataformas disponíveis: [{}]",
            plataforma.unwrap_or("null"),
            nomes.join(", ")
        ),
    )
}

/// comparação sem caixa contra o parâmetro aparado; a mensagem usa o original
pub fn resolver<'a>(
    plataforma: Option<&str>,
    disponiveis: &'a [(String, String)],
) -> Resultado<ResolucaoDownload<'a>> {
    let Some(p) = plataforma.filter(|p| !p.trim().is_empty()) else {
        return Err(nao_encontrada(plataforma, disponiveis));
    };
    let alvo = p.trim_matches(|c: char| c <= ' ').to_lowercase();
    disponiveis
        .iter()
        .find(|(nome, _)| nome.to_lowercase() == alvo)
        .map(|(plataforma, url)| ResolucaoDownload { plataforma, url })
        .ok_or_else(|| nao_encontrada(plataforma, disponiveis))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plataformas() -> Vec<(String, String)> {
        vec![
            ("linux".into(), "https://x/linux".into()),
            ("windows".into(), String::new()),
        ]
    }

    #[test]
    fn resolve_sem_caixa_e_aparado() {
        let p = plataformas();
        let r = resolver(Some("  LINUX "), &p).ok();
        assert_eq!(r.map(|r| r.url), Some("https://x/linux"));
    }

    #[test]
    fn ausente_lista_disponiveis() {
        let p = plataformas();
        let msg = resolver(Some("mac"), &p).err().and_then(|f| f.mensagem);
        assert_eq!(
            msg.as_deref(),
            Some(
                "Plataforma de download 'mac' não encontrada. Plataformas disponíveis: [linux, windows]"
            )
        );
        let msg = resolver(Some(" "), &[]).err().and_then(|f| f.mensagem);
        assert_eq!(
            msg.as_deref(),
            Some("Plataforma de download ' ' não encontrada. Plataformas disponíveis: []")
        );
    }
}
