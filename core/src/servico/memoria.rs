//! `MemoriaCalculoService` + `TokenizerService`: texto da memória de cálculo

use chrono::NaiveDate;

use super::Servicos;
use super::cbs_ibs::{CbsIbsSaida, npe};
use super::texto::branco;
use crate::db::TratamentoClassificacao;
use crate::decimal::BigDecimal;
use crate::erro::Resultado;
use crate::modelo::roc::ImpostoSeletivo;

const SEM_TEXTO: &str = "Texto de memória de cálculo não encontrado";
const ZERO_TEXTO: &str = "0";

/// troca `[chave]` pelo valor (vazio se a chave não existir); `[` sem `]` fica literal
pub fn substituir_placeholders(modelo: &str, valores: &[(&str, String)]) -> String {
    if !modelo.contains('[') {
        return modelo.to_string();
    }
    let mut saida = String::with_capacity(modelo.len());
    let mut resto = modelo;
    while !resto.is_empty() {
        let Some(abre) = resto.find('[') else {
            saida.push_str(resto);
            break;
        };
        let Some(fecha) = resto[abre + 1..].find(']').map(|i| abre + 1 + i) else {
            saida.push_str(resto);
            break;
        };
        saida.push_str(&resto[..abre]);
        let chave = &resto[abre + 1..fecha];
        if let Some((_, v)) = valores.iter().find(|(k, _)| *k == chave) {
            saida.push_str(v);
        }
        resto = &resto[fecha + 1..];
    }
    saida
}

fn ou_zero(v: Option<&BigDecimal>) -> String {
    v.map_or_else(|| ZERO_TEXTO.to_string(), BigDecimal::to_string)
}

fn ou_vazio(v: Option<&BigDecimal>) -> String {
    v.map(BigDecimal::to_string).unwrap_or_default()
}

impl Servicos<'_> {
    fn texto_memoria(
        &self,
        tratamento: &TratamentoClassificacao,
        dia: NaiveDate,
        valores: impl FnOnce(String, String) -> Vec<(&'static str, String)>,
    ) -> Resultado<String> {
        let fundamentacao = self.fundamentacao(tratamento.id_classificacao, dia)?;
        let tt = self.tratamento_tributario(tratamento.id_tratamento)?;
        let norma = fundamentacao.texto_curto.unwrap_or_default();
        let valores = valores(norma, tt.descricao);
        Ok(match fundamentacao.memoria_calculo {
            Some(texto) if !branco(Some(&texto)) => substituir_placeholders(&texto, &valores),
            _ => SEM_TEXTO.to_string(),
        })
    }

    pub fn memoria_cbs_ibs(
        &self,
        tratamento: &TratamentoClassificacao,
        saida: &mut CbsIbsSaida,
        quantidade: &BigDecimal,
        unidade: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<()> {
        let ad_rem = saida
            .grupo_monofasia
            .as_ref()
            .and_then(|g| g.monofasico.as_ref())
            .and_then(|m| m.aliquota_ad_rem.as_ref());
        let reducao = saida
            .grupo_reducao
            .as_ref()
            .and_then(|g| g.p_red_aliq.as_ref());
        let (pdif, vdif) = match &saida.grupo_diferimento {
            Some(d) => match (&d.p_dif, &d.v_dif) {
                (Some(p), Some(v)) => (p.to_string(), v.to_string()),
                _ => (ZERO_TEXTO.to_string(), ZERO_TEXTO.to_string()),
            },
            None => (ZERO_TEXTO.to_string(), ZERO_TEXTO.to_string()),
        };
        let base = ou_zero(saida.base_calculo.as_ref());
        let aliquota = ou_zero(saida.aliquota.as_ref());
        let ad_rem = ou_zero(ad_rem);
        let reducao = ou_zero(reducao);
        let unidade = unidade.unwrap_or_default().to_string();
        let texto = self.texto_memoria(tratamento, dia, |norma, tratamento| {
            vec![
                ("norma", norma),
                ("tratamento", tratamento),
                ("base_calculo", base),
                ("aliquota_ad_valorem", aliquota),
                ("aliquota_ad_rem", ad_rem),
                ("quantidade", quantidade.to_string()),
                ("unidade", unidade),
                ("percentual_reducao", reducao),
                ("percentual_diferimento", pdif),
                ("valor_diferimento", vdif),
                ("aliquota_desoneracao", ZERO_TEXTO.to_string()),
                ("montante_desoneracao", ZERO_TEXTO.to_string()),
            ]
        })?;
        saida.memoria_calculo = Some(texto);
        Ok(())
    }

    pub fn memoria_imposto_seletivo(
        &self,
        tratamento: &TratamentoClassificacao,
        is: &mut ImpostoSeletivo,
        dia: NaiveDate,
    ) -> Resultado<()> {
        let base = is
            .v_bc_is
            .as_ref()
            .ok_or_else(|| npe("Cannot invoke \"java.math.BigDecimal.toString()\" because the return value of \"br.gov.serpro.rtc.api.model.roc.ImpostoSeletivoDomain.getVBCIS()\" is null"))?
            .to_string();
        let ad_valorem = ou_vazio(is.p_is.as_ref());
        let ad_rem = ou_vazio(is.p_is_espec.as_ref());
        let quantidade = ou_vazio(is.q_trib.as_ref());
        let unidade = is.u_trib.clone().unwrap_or_default();
        let texto = self.texto_memoria(tratamento, dia, |norma, tratamento| {
            vec![
                ("norma", norma),
                ("tratamento", tratamento),
                ("base_calculo", base),
                ("aliquota_ad_valorem", ad_valorem),
                ("aliquota_ad_rem", ad_rem),
                ("quantidade", quantidade),
                ("unidade", unidade),
            ]
        })?;
        is.memoria_calculo = Some(texto);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholders_como_tokenizer_java() {
        let v = [("a", "1".to_string()), ("b", "2".to_string())];
        assert_eq!(substituir_placeholders("x[a]y[b]z", &v), "x1y2z");
        assert_eq!(substituir_placeholders("[c] e [a", &v), " e [a");
        assert_eq!(substituir_placeholders("sem", &v), "sem");
    }
}
