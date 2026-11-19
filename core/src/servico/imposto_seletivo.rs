use super::Servicos;
use super::cbs_ibs::{Variaveis, avaliar, var, variaveis};
use super::texto::preenchido;
use crate::db::TratamentoClassificacao;
use crate::decimal::BigDecimal;
use crate::erro::Resultado;
use crate::modelo::entrada::ImpostoSeletivoEntrada;
use crate::modelo::roc::ImpostoSeletivo;
use crate::util::arredondamento::dividir_por_cem;

const CASAS_PERCENTUAL: i64 = 2;

#[derive(Debug, Clone, Default)]
pub struct AliquotaImpostoSeletivo {
    pub ad_valorem: Option<BigDecimal>,
    pub ad_rem: Option<BigDecimal>,
    pub unidade_medida: Option<String>,
}

impl Servicos<'_> {
    pub fn calcular_imposto_seletivo(
        &self,
        is: &ImpostoSeletivoEntrada,
        tratamento: &TratamentoClassificacao,
        aliquota: &AliquotaImpostoSeletivo,
    ) -> Resultado<ImpostoSeletivo> {
        use var::*;
        let t = self.tratamento_tributario(tratamento.id_tratamento)?;
        let ad_valorem = aliquota.ad_valorem.as_ref().map(dividir_por_cem);
        let ad_rem = aliquota.ad_rem.as_ref();
        let informado = is.valor_informado();
        let calculado = BigDecimal::zero();
        let montar = |proprias: &[(&'static str, Option<&BigDecimal>)]| -> Variaveis {
            let mut pares = proprias.to_vec();
            pares.extend_from_slice(&[
                (QUANTIDADE, is.quantidade.as_ref()),
                (ALIQUOTA, None),
                (ALIQUOTA_EFETIVA, None),
                (BASE_CALCULO_INFORMADA, is.base_calculo.as_ref()),
                (IMPOSTO_SELETIVO_INFORMADO, Some(&informado)),
                (IMPOSTO_SELETIVO_CALCULADO, Some(&calculado)),
                (PERCENTUAL_REDUCAO, None),
                (ALIQUOTA_PADRAO_OU_REFERENCIA, None),
                (ALIQUOTA_REFERENCIA, None),
                (ALIQUOTA_AD_VALOREM, ad_valorem.as_ref()),
                (ALIQUOTA_AD_REM, ad_rem),
                (AJUSTE, None),
                (REDUTOR, None),
            ]);
            variaveis(&pares)
        };
        let base_calculo = avaliar(&t.expressao_base_calculo, &montar(&[]))?;
        let com_base = [(BASE_CALCULO, Some(&base_calculo))];
        let tributo_calculado = avaliar(&t.expressao_tributo_calculado, &montar(&com_base))?;
        let tributo_devido = match t
            .expressao_tributo_devido
            .as_deref()
            .filter(|e| preenchido(Some(e)))
        {
            Some(e) => {
                let v = montar(&[
                    (BASE_CALCULO, Some(&base_calculo)),
                    (TRIBUTO_CALCULADO, Some(&tributo_calculado)),
                ]);
                Some(avaliar(e, &v)?)
            }
            None => None,
        };
        Ok(ImpostoSeletivo {
            cst_is: is.cst.clone(),
            c_class_trib_is: is.c_class_trib.clone(),
            v_bc_is: Some(base_calculo),
            p_is: ad_valorem.map(|a| a.move_point_right(CASAS_PERCENTUAL)),
            p_is_espec: ad_rem.cloned(),
            ad_rem_is: ad_rem.cloned(),
            u_trib: is.unidade.clone(),
            q_trib: is.quantidade.clone(),
            v_is: tributo_devido,
            memoria_calculo: None,
        })
    }
}
