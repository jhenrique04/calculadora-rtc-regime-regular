//! entradas do `NfseController`

use chrono::NaiveDate;

use crate::decimal::BigDecimal;
use crate::objeto_json;

/// `NfseBaseCalculoInput`: os opcionais começam em zero, mas `null` explícito
/// sobrescreve o padrão
#[derive(Debug, Clone)]
pub struct NfseBaseCalculoEntrada {
    pub ano_fato_gerador: Option<i32>,
    pub valor_servico: Option<BigDecimal>,
    pub desconto_incondicional: Option<BigDecimal>,
    pub v_calc_ree_rep_res: Option<BigDecimal>,
    pub v_calc_ded_red_ibs_cbs: Option<BigDecimal>,
    pub iss: Option<BigDecimal>,
    pub pis: Option<BigDecimal>,
    pub cofins: Option<BigDecimal>,
}

impl Default for NfseBaseCalculoEntrada {
    fn default() -> Self {
        Self {
            ano_fato_gerador: None,
            valor_servico: None,
            desconto_incondicional: Some(BigDecimal::zero()),
            v_calc_ree_rep_res: Some(BigDecimal::zero()),
            v_calc_ded_red_ibs_cbs: Some(BigDecimal::zero()),
            iss: Some(BigDecimal::zero()),
            pis: Some(BigDecimal::zero()),
            cofins: Some(BigDecimal::zero()),
        }
    }
}

objeto_json!(NfseBaseCalculoEntrada, "br.gov.serpro.rtc.api.model.input.nfse.NfseBaseCalculoInput", {
    "anoFatoGerador" => ano_fato_gerador: i32,
    "valorServico" => valor_servico: BigDecimal,
    "descontoIncondicional" => desconto_incondicional: BigDecimal,
    "vCalcReeRepRes" => v_calc_ree_rep_res: BigDecimal,
    "vCalcDedRedIBSCBS" => v_calc_ded_red_ibs_cbs: BigDecimal,
    "iss" => iss: BigDecimal,
    "pis" => pis: BigDecimal,
    "cofins" => cofins: BigDecimal,
});

#[derive(Debug, Clone, Default)]
pub struct NfseIndicadorOperacaoEntrada {
    pub c_trib_nac: Option<String>,
    pub c_ind_op: Option<String>,
    pub c_class_trib: Option<String>,
    pub nbs: Option<String>,
    pub data_fato_gerador: Option<NaiveDate>,
}

objeto_json!(NfseIndicadorOperacaoEntrada, "br.gov.serpro.rtc.api.model.input.nfse.NfseValidacaoIndicadorOperacaoInput", {
    "cTribNac" => c_trib_nac: String,
    "cIndOp" => c_ind_op: String,
    "cClassTrib" => c_class_trib: String,
    "nbs" => nbs: String,
    "dataOcorrenciaFatoGerador" => data_fato_gerador: NaiveDate,
});
