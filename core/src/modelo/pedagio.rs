//! entrada do `PedagioController`

use crate::decimal::BigDecimal;
use crate::json::DataHora;
use crate::objeto_json;

#[derive(Debug, Clone, Default)]
pub struct TrechoEntrada {
    pub numero: Option<i32>,
    pub municipio: Option<i64>,
    pub uf: Option<String>,
    pub extensao: Option<BigDecimal>,
}

objeto_json!(TrechoEntrada, "br.gov.serpro.rtc.api.model.input.pedagio.TrechoPedagioInput", {
    "numero" => numero: i32,
    "municipio" => municipio: i64,
    "uf" => uf: String,
    "extensao" => extensao: BigDecimal,
});

#[derive(Debug, Clone, Default)]
pub struct PedagioEntrada {
    pub data_hora_emissao: Option<DataHora>,
    pub municipio_origem: Option<i64>,
    pub uf_origem: Option<String>,
    pub cst: Option<String>,
    pub c_class_trib: Option<String>,
    pub base_calculo: Option<BigDecimal>,
    pub trechos: Option<Vec<Option<TrechoEntrada>>>,
}

objeto_json!(PedagioEntrada, "br.gov.serpro.rtc.api.model.input.pedagio.PedagioInput", {
    "dataHoraEmissao" => data_hora_emissao: DataHora,
    "codigoMunicipioOrigem" => municipio_origem: i64,
    "ufMunicipioOrigem" => uf_origem: String,
    "cst" => cst: String,
    "cClassTrib" => c_class_trib: String,
    "baseCalculo" => base_calculo: BigDecimal,
    "trechos" => trechos: Vec<Option<TrechoEntrada>>,
});

impl PedagioEntrada {
    /// `getExtensaoTotal`: ignora trechos e extensões nulos
    pub fn extensao_total(&self) -> BigDecimal {
        self.trechos
            .iter()
            .flatten()
            .flatten()
            .filter_map(|t| t.extensao.as_ref())
            .fold(BigDecimal::zero(), |acc, e| acc.add(e))
    }
}
