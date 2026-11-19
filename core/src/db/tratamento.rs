use chrono::NaiveDate;
use rusqlite::params;

use super::{Base, data};
use crate::erro::Resultado;

#[derive(Debug, Clone)]
pub struct TratamentoClassificacao {
    pub id_tratamento: i64,
    pub id_classificacao: i64,
    pub incompativel_com_suspensao: bool,
    pub exige_grupo_desoneracao: bool,
}

#[derive(Debug, Clone)]
pub struct TratamentoTributario {
    pub id: i64,
    pub descricao: String,
    pub expressao_aliquota: Option<String>,
    pub expressao_aliquota_efetiva: Option<String>,
    pub expressao_base_calculo: String,
    pub expressao_percentual_diferimento: Option<String>,
    pub expressao_tributo_calculado: String,
    pub expressao_tributo_devido: Option<String>,
    pub expressao_valor_diferimento: Option<String>,
    pub possui_ajuste: bool,
    pub possui_monofasia: bool,
    pub possui_percentual_reducao: bool,
}

impl Base<'_> {
    pub fn tratamento_classificacao(
        &self,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<TratamentoClassificacao>> {
        let d = data(dia);
        self.unica(
            "select tc1_0.trcl_trtr_id,tc1_0.trcl_cltr_id,tt1_0.trtr_in_incompativel_com_suspensao,tt1_0.trtr_in_exige_grupo_desoneracao from tratamento_classificacao tc1_0 join tratamento_tributario tt1_0 on tt1_0.trtr_id=tc1_0.trcl_trtr_id where tc1_0.trcl_cltr_id=? and (tc1_0.trcl_inicio_vigencia<=? and (tc1_0.trcl_fim_vigencia is null or tc1_0.trcl_fim_vigencia>=?))",
            params![id_classificacao, d, d],
            |r| {
                Ok(TratamentoClassificacao {
                    id_tratamento: r.get(0)?,
                    id_classificacao: r.get(1)?,
                    incompativel_com_suspensao: r.get(2)?,
                    exige_grupo_desoneracao: r.get(3)?,
                })
            },
        )
    }

    pub fn tratamento_tributario(&self, id: i64) -> Resultado<Option<TratamentoTributario>> {
        self.unica(
            "select tt1_0.trtr_id,tt1_0.trtr_descricao,tt1_0.trtr_expressao_aliquota,tt1_0.trtr_expressao_aliquota_efetiva,tt1_0.trtr_expressao_base_calculo,tt1_0.trtr_expressao_percentual_diferimento,tt1_0.trtr_expressao_tributo_calculado,tt1_0.trtr_expressao_tributo_devido,tt1_0.trtr_expressao_valor_diferimento,tt1_0.trtr_in_possui_ajuste,tt1_0.trtr_in_possui_monofasia,tt1_0.trtr_in_possui_percentual_reducao from tratamento_tributario tt1_0 where tt1_0.trtr_id=?",
            [id],
            |r| {
                Ok(TratamentoTributario {
                    id: r.get(0)?,
                    descricao: r.get(1)?,
                    expressao_aliquota: r.get(2)?,
                    expressao_aliquota_efetiva: r.get(3)?,
                    expressao_base_calculo: r.get(4)?,
                    expressao_percentual_diferimento: r.get(5)?,
                    expressao_tributo_calculado: r.get(6)?,
                    expressao_tributo_devido: r.get(7)?,
                    expressao_valor_diferimento: r.get(8)?,
                    possui_ajuste: r.get(9)?,
                    possui_monofasia: r.get(10)?,
                    possui_percentual_reducao: r.get(11)?,
                })
            },
        )
    }
}
