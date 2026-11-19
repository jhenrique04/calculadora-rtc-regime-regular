//! `IndicadorOperacaoRepository` (queries nativas com o texto original)

use chrono::NaiveDate;
use rusqlite::named_params;

use super::{Base, data};
use crate::erro::Resultado;

#[derive(Debug, Clone)]
pub struct LinhaIndicador {
    pub codigo: Option<String>,
    pub tipo_operacao: Option<String>,
    pub local_fornecimento: Option<i64>,
    pub local_incidencia: Option<i64>,
    pub onerosa: Option<i64>,
    pub adquirente_exterior: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct SituacaoClassificacao {
    pub codigo_situacao: Option<String>,
    pub descricao_situacao: Option<String>,
    pub codigo_classificacao: Option<String>,
    pub descricao_classificacao: Option<String>,
}

/// filtro de `validarCombinacaoExiste`
#[derive(Debug, Clone, Copy)]
pub struct CombinacaoIndicador<'a> {
    pub nbs: Option<&'a str>,
    pub c_class_trib: Option<&'a str>,
    pub c_ind_op: Option<&'a str>,
    pub c_trib_nac: Option<&'a str>,
    pub dia: NaiveDate,
}

const SQL_COMBINACAO: &str = "SELECT EXISTS ( SELECT 1 FROM CLASSIF_NBS_INDOP_LC c INNER JOIN CLASSIFICACAO_TRIBUTARIA ct ON c.CNIL_CLTR_ID = ct.CLTR_ID INNER JOIN INDICADOR_OPERACAO_IBS_CBS io ON c.CNIL_IOIC_ID = io.IOIC_ID WHERE c.CNIL_NBS_CD = :nbsCd AND ct.CLTR_CD = :cClassTrib AND io.IOIC_CD = :cIndOp AND (:cTribNac IS NULL OR c.CNIL_LSLC_CD = :cTribNac) AND c.CNIL_INICIO_VIGENCIA <= :dataOcorrenciaFatoGerador AND (c.CNIL_FIM_VIGENCIA IS NULL OR c.CNIL_FIM_VIGENCIA >= :dataOcorrenciaFatoGerador) )";

const SQL_LOCAL_OPERACAO: &str = "SELECT io.IOIC_CD_LOCAL_FORNECIMENTO_DFE, io.IOIC_CD_LOCAL_INCIDENCIA FROM INDICADOR_OPERACAO_IBS_CBS io WHERE io.IOIC_CD = :cIndOp AND :data BETWEEN io.IOIC_INICIO_VIGENCIA AND COALESCE(io.IOIC_FIM_VIGENCIA, :data)";

const SQL_INDICADORES: &str = "SELECT io.IOIC_CD, io.IOIC_TIPO_OPERACAO, io.IOIC_CD_LOCAL_FORNECIMENTO_DFE, io.IOIC_LOCAL_INCIDENCIA, io.IOIC_CD_LOCAL_INCIDENCIA, NULL, NULL FROM INDICADOR_OPERACAO_IBS_CBS io WHERE io.IOIC_TIPO_OPERACAO IS NOT NULL AND :data BETWEEN io.IOIC_INICIO_VIGENCIA AND COALESCE(io.IOIC_FIM_VIGENCIA, :data) ORDER BY io.IOIC_CD";

const SQL_INDICADORES_NBS: &str = "SELECT DISTINCT io.IOIC_CD, io.IOIC_TIPO_OPERACAO, io.IOIC_CD_LOCAL_FORNECIMENTO_DFE, io.IOIC_LOCAL_INCIDENCIA, io.IOIC_CD_LOCAL_INCIDENCIA, c.CNIL_IN_PS_ONEROSA, c.CNIL_IN_ADQ_EXTERIOR FROM INDICADOR_OPERACAO_IBS_CBS io INNER JOIN CLASSIF_NBS_INDOP_LC c ON io.IOIC_ID = c.CNIL_IOIC_ID WHERE c.CNIL_NBS_CD = :nbs AND :data BETWEEN io.IOIC_INICIO_VIGENCIA AND COALESCE(io.IOIC_FIM_VIGENCIA, :data) AND :data BETWEEN c.CNIL_INICIO_VIGENCIA AND COALESCE(c.CNIL_FIM_VIGENCIA, :data) AND io.IOIC_TIPO_OPERACAO IS NOT NULL ORDER BY io.IOIC_CD";

const SQL_SITUACOES_NBS: &str = "SELECT DISTINCT st.SITR_CD AS codigoSituacaoTributaria, st.SITR_DESCRICAO AS descricaoSituacaoTributaria, ct.CLTR_CD AS codigoClassificacaoTributaria, ct.CLTR_DESCRICAO AS descricaoClassificacaoTributaria FROM CLASSIF_NBS_INDOP_LC c INNER JOIN CLASSIFICACAO_TRIBUTARIA ct ON c.CNIL_CLTR_ID = ct.CLTR_ID INNER JOIN SITUACAO_TRIBUTARIA st ON ct.CLTR_SITR_ID = st.SITR_ID WHERE c.CNIL_NBS_CD = :nbs AND :data BETWEEN c.CNIL_INICIO_VIGENCIA AND COALESCE(c.CNIL_FIM_VIGENCIA, :data) AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND :data BETWEEN st.SITR_INICIO_VIGENCIA AND COALESCE(st.SITR_FIM_VIGENCIA, :data) ORDER BY ct.CLTR_CD, st.SITR_CD";

const SQL_SITUACAO_CLASSIFICACAO: &str = "SELECT st.SITR_CD AS codigoSituacaoTributaria, st.SITR_DESCRICAO AS descricaoSituacaoTributaria, ct.CLTR_CD AS codigoClassificacaoTributaria, ct.CLTR_DESCRICAO AS descricaoClassificacaoTributaria FROM CLASSIFICACAO_TRIBUTARIA ct INNER JOIN SITUACAO_TRIBUTARIA st ON ct.CLTR_SITR_ID = st.SITR_ID WHERE ct.CLTR_CD = :codigoClassificacaoTributaria AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND :data BETWEEN st.SITR_INICIO_VIGENCIA AND COALESCE(st.SITR_FIM_VIGENCIA, :data) LIMIT 1";

fn linha_indicador(r: &rusqlite::Row<'_>) -> rusqlite::Result<LinhaIndicador> {
    Ok(LinhaIndicador {
        codigo: r.get(0)?,
        tipo_operacao: r.get(1)?,
        local_fornecimento: r.get(2)?,
        local_incidencia: r.get(4)?,
        onerosa: r.get(5)?,
        adquirente_exterior: r.get(6)?,
    })
}

fn situacao_classificacao(r: &rusqlite::Row<'_>) -> rusqlite::Result<SituacaoClassificacao> {
    Ok(SituacaoClassificacao {
        codigo_situacao: r.get(0)?,
        descricao_situacao: r.get(1)?,
        codigo_classificacao: r.get(2)?,
        descricao_classificacao: r.get(3)?,
    })
}

impl Base<'_> {
    pub fn combinacao_indicador_existe(&self, c: CombinacaoIndicador<'_>) -> Resultado<bool> {
        let d = data(c.dia);
        let v = self.unica(
            SQL_COMBINACAO,
            named_params! {
                ":nbsCd": c.nbs,
                ":cClassTrib": c.c_class_trib,
                ":cIndOp": c.c_ind_op,
                ":cTribNac": c.c_trib_nac,
                ":dataOcorrenciaFatoGerador": d,
            },
            |r| r.get::<_, i64>(0),
        )?;
        Ok(v == Some(1))
    }

    /// `Object[]` do Spring Data: lista de linhas, o serviço usa a primeira
    pub fn local_operacao(
        &self,
        c_ind_op: &str,
        dia: NaiveDate,
    ) -> Resultado<Option<(Option<i64>, Option<i64>)>> {
        let d = data(dia);
        let linhas = self.linhas(
            SQL_LOCAL_OPERACAO,
            named_params! {":cIndOp": c_ind_op, ":data": d},
            |r| Ok((r.get(0)?, r.get(1)?)),
        )?;
        Ok(linhas.into_iter().next())
    }

    pub fn indicadores_operacao(&self, dia: NaiveDate) -> Resultado<Vec<LinhaIndicador>> {
        let d = data(dia);
        self.linhas(SQL_INDICADORES, named_params! {":data": d}, linha_indicador)
    }

    pub fn indicadores_operacao_nbs(
        &self,
        dia: NaiveDate,
        nbs: &str,
    ) -> Resultado<Vec<LinhaIndicador>> {
        let d = data(dia);
        self.linhas(
            SQL_INDICADORES_NBS,
            named_params! {":data": d, ":nbs": nbs},
            linha_indicador,
        )
    }

    pub fn situacoes_classificacoes_nbs(
        &self,
        nbs: &str,
        dia: NaiveDate,
    ) -> Resultado<Vec<SituacaoClassificacao>> {
        let d = data(dia);
        self.linhas(
            SQL_SITUACOES_NBS,
            named_params! {":nbs": nbs, ":data": d},
            situacao_classificacao,
        )
    }

    pub fn situacao_da_classificacao(
        &self,
        codigo: &str,
        dia: NaiveDate,
    ) -> Resultado<Option<SituacaoClassificacao>> {
        let d = data(dia);
        self.unica(
            SQL_SITUACAO_CLASSIFICACAO,
            named_params! {":codigoClassificacaoTributaria": codigo, ":data": d},
            situacao_classificacao,
        )
    }
}
