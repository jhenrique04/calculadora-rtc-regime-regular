//! `VersaoOfflineService`: compara a versão local com a publicada online

use super::Servicos;
use crate::erro::{Falha, Resultado};
use crate::{objeto_json, saida_json};

/// `VersaoOutput` devolvido pelo endpoint remoto (só os campos comparados)
#[derive(Debug, Clone, Default)]
pub struct VersaoRemota {
    pub versao_app: Option<String>,
    pub versao_db: Option<String>,
    pub data_versao_db: Option<String>,
}

objeto_json!(VersaoRemota, "br.gov.serpro.rtc.api.model.output.VersaoOutput", {
    "versaoApp" => versao_app: String,
    "versaoDb" => versao_db: String,
    "dataVersaoDb" => data_versao_db: String,
});

#[derive(Debug, Clone)]
pub struct VersaoOfflineSaida {
    pub aplicacao_atualizada: bool,
    pub db_atualizada: bool,
    pub data_db_atualizada: bool,
    pub versao_aplicacao_local: Option<String>,
    pub versao_aplicacao_remota: Option<String>,
    pub versao_db_local: Option<String>,
    pub versao_db_remota: Option<String>,
    pub data_db_local: Option<String>,
    pub data_db_remota: Option<String>,
}

saida_json!(VersaoOfflineSaida {
    "aplicacaoAtualizada" => aplicacao_atualizada: booleano,
    "dbAtualizada" => db_atualizada: booleano,
    "dataDbAtualizada" => data_db_atualizada: booleano,
    "versaoAplicacaoLocal" => versao_aplicacao_local: texto,
    "versaoAplicacaoRemota" => versao_aplicacao_remota: texto,
    "versaoDbLocal" => versao_db_local: texto,
    "versaoDbRemota" => versao_db_remota: texto,
    "dataDbLocal" => data_db_local: texto,
    "dataDbRemota" => data_db_remota: texto,
});

/// prefixo `MAJOR.MINOR.PATCH` sem zeros à esquerda (regex `SEMVER_PATTERN`)
fn extrair_semver(versao: &str) -> Option<&str> {
    let mut fim = 0;
    for parte in 0..3 {
        if parte > 0 {
            if versao.as_bytes().get(fim) != Some(&b'.') {
                return None;
            }
            fim += 1;
        }
        let digitos = versao[fim..].bytes().take_while(u8::is_ascii_digit).count();
        if digitos == 0 {
            return None;
        }
        // "0" casa sozinho; "01" para no zero e o ponto seguinte falha
        let usados = if versao.as_bytes().get(fim) == Some(&b'0') {
            1
        } else {
            digitos
        };
        fim += usados;
    }
    versao.get(..fim)
}

fn iguais(local: Option<&String>, remota: Option<&String>) -> bool {
    local.is_some() && local == remota
}

impl VersaoOfflineSaida {
    pub fn comparar(local: VersaoLocal, remota: Option<VersaoRemota>) -> Self {
        let remota = remota.unwrap_or_default();
        let semver_local = local.versao_app.as_deref().and_then(extrair_semver);
        let semver_remota = remota.versao_app.as_deref().and_then(extrair_semver);
        Self {
            aplicacao_atualizada: semver_local.is_some() && semver_local == semver_remota,
            db_atualizada: iguais(local.versao_db.as_ref(), remota.versao_db.as_ref()),
            data_db_atualizada: iguais(
                local.data_versao_db.as_ref(),
                remota.data_versao_db.as_ref(),
            ),
            versao_aplicacao_local: local.versao_app,
            versao_aplicacao_remota: remota.versao_app,
            versao_db_local: local.versao_db,
            versao_db_remota: remota.versao_db,
            data_db_local: local.data_versao_db,
            data_db_remota: remota.data_versao_db,
        }
    }

    /// linhas de `exibirStatusVersao`: (atualizadas, desatualizadas)
    pub fn linhas_status(&self) -> (Vec<String>, Vec<String>) {
        let mut atualizados = Vec::new();
        let mut desatualizados = Vec::new();
        let grupos = [
            (
                "Aplicação",
                self.aplicacao_atualizada,
                &self.versao_aplicacao_local,
                &self.versao_aplicacao_remota,
            ),
            (
                "Banco de Dados",
                self.db_atualizada,
                &self.versao_db_local,
                &self.versao_db_remota,
            ),
            (
                "Data do Banco",
                self.data_db_atualizada,
                &self.data_db_local,
                &self.data_db_remota,
            ),
        ];
        for (nome, ok, local, remota) in grupos {
            let destino = if ok {
                &mut atualizados
            } else {
                &mut desatualizados
            };
            destino.push(format!("{nome} (LOCAL):  {}", texto_java(local)));
            destino.push(format!("{nome} (ONLINE): {}", texto_java(remota)));
        }
        (atualizados, desatualizados)
    }
}

/// concatenação Java: `null` vira "null"
fn texto_java(v: &Option<String>) -> &str {
    v.as_deref().unwrap_or("null")
}

/// lado local da comparação
#[derive(Debug, Clone)]
pub struct VersaoLocal {
    pub versao_app: Option<String>,
    pub versao_db: Option<String>,
    pub data_versao_db: Option<String>,
}

impl Servicos<'_> {
    pub fn versao_local(&self, versao_app: &str) -> Resultado<VersaoLocal> {
        let v = self.versao(versao_app)?;
        let data = v.data_versao_db.ok_or_else(|| {
            Falha::sistema("Cannot invoke \"java.time.LocalDate.toString()\" because the return value of \"br.gov.serpro.rtc.domain.model.entity.VersaoBaseDados.getData()\" is null")
        })?;
        Ok(VersaoLocal {
            versao_app: Some(versao_app.to_string()),
            versao_db: v.versao_db,
            data_versao_db: Some(data),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local() -> VersaoLocal {
        VersaoLocal {
            versao_app: Some("1.6.1".into()),
            versao_db: Some("V0059".into()),
            data_versao_db: Some("2026-09-30".into()),
        }
    }

    #[test]
    fn semver_ancorado_no_inicio() {
        assert_eq!(extrair_semver("1.7.0-753903df"), Some("1.7.0"));
        assert_eq!(extrair_semver("10.0.12"), Some("10.0.12"));
        assert_eq!(extrair_semver("01.2.3"), None);
        assert_eq!(extrair_semver("v1.2.3"), None);
        assert_eq!(extrair_semver("1.2"), None);
    }

    #[test]
    fn compara_com_remota() {
        let remota = VersaoRemota {
            versao_app: Some("1.6.1-abc".into()),
            versao_db: Some("V0073".into()),
            data_versao_db: Some("2026-09-30".into()),
        };
        let s = VersaoOfflineSaida::comparar(local(), Some(remota));
        assert!(s.aplicacao_atualizada);
        assert!(!s.db_atualizada);
        assert!(s.data_db_atualizada);
        let (ok, nok) = s.linhas_status();
        assert_eq!(ok.len(), 4);
        assert_eq!(
            nok,
            vec![
                "Banco de Dados (LOCAL):  V0059",
                "Banco de Dados (ONLINE): V0073"
            ]
        );
    }

    #[test]
    fn sem_remota_tudo_desatualizado() {
        let s = VersaoOfflineSaida::comparar(local(), None);
        assert!(!s.aplicacao_atualizada && !s.db_atualizada && !s.data_db_atualizada);
        assert_eq!(
            s.linhas_status().1.get(1).map(String::as_str),
            Some("Aplicação (ONLINE): null")
        );
    }
}
