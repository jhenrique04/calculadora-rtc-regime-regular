//! consultas dos dados abertos. Queries nativas mantêm o texto dos
//! repositórios (o plano do SQLite decide a ordem das linhas sem `order by`)

use chrono::NaiveDate;
use rusqlite::types::{ToSql, ValueRef};
use rusqlite::{Row, named_params, params};

use super::{Base, data, decimal, entidade_ausente};
use crate::decimal::BigDecimal;
use crate::erro::Resultado;

#[derive(Debug, Clone)]
pub struct Uf {
    pub codigo: i64,
    pub nome: String,
    pub sigla: String,
}

#[derive(Debug, Clone)]
pub struct Municipio {
    pub codigo: i64,
    pub nome: String,
}

#[derive(Debug, Clone)]
pub struct SituacaoResumo {
    pub id: i64,
    pub codigo: String,
    pub descricao: String,
}

#[derive(Debug, Clone)]
pub struct CodigoDescricao {
    pub codigo: Option<String>,
    pub descricao: Option<String>,
}

/// campos exclusivos da consulta de CBS/IBS
#[derive(Debug, Clone, Default)]
pub struct CreditosCbsIbs {
    pub apropriacao_adquirente_cbs: bool,
    pub apropriacao_adquirente_ibs: bool,
    pub presumido_fornecedor: bool,
    pub presumido_adquirente: bool,
    pub operacao_antecedente: Option<String>,
    pub reducao_cbs: Option<BigDecimal>,
    pub reducao_ibs_uf: Option<BigDecimal>,
    pub reducao_ibs_mun: Option<BigDecimal>,
    pub tipo_receita_bruta_sn: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct LinhaClassificacao {
    pub id: i64,
    pub codigo: Option<String>,
    pub descricao: Option<String>,
    pub tipo_aliquota: Option<String>,
    pub nomenclatura: Option<String>,
    pub descricao_tratamento: Option<String>,
    pub incompativel_com_suspensao: bool,
    pub exige_grupo_desoneracao: bool,
    pub possui_percentual_reducao: bool,
    pub data_atualizacao: Option<NaiveDate>,
    pub creditos: Option<CreditosCbsIbs>,
}

#[derive(Debug, Clone)]
pub struct ValidadeDfe {
    pub id_classificacao: i64,
    pub nomenclatura: Option<String>,
    pub exige_grupo_regular: Option<i64>,
    pub permite_diferimento: Option<i64>,
    pub credito_presumido: Option<i64>,
    pub exige_grupo_ibs_cbs: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct CstClassificacao {
    pub id_cst: Option<i64>,
    pub codigo_cst: String,
    pub descricao_cst: Option<String>,
    pub codigo_classificacao: Option<String>,
    pub descricao_classificacao: Option<String>,
}

#[derive(Debug, Clone)]
pub struct LinhaGrupoDfe {
    pub grupo_id: i64,
    pub grupo_titulo: Option<String>,
    pub sigla: String,
    pub titulo: Option<String>,
    pub modelo: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct LinhaGrupoAtor {
    pub grupo_id: i64,
    pub grupo_descricao: Option<String>,
    pub grupo_ordem: i64,
    pub ator_id: i64,
    pub ator_descricao: Option<String>,
    pub ator_ordem: i64,
}

#[derive(Debug, Clone)]
pub struct FundamentacaoLegal {
    pub codigo_classificacao: String,
    pub descricao_classificacao: String,
    pub id_situacao: i64,
    pub codigo_situacao: String,
    pub descricao_situacao: String,
    pub texto: String,
    pub texto_curto: String,
    pub referencia_normativa: String,
}

/// indicador `Integer` do `Object[]`: nulo conta como falso
fn indicador(r: &Row<'_>, i: usize) -> rusqlite::Result<bool> {
    Ok(r.get::<_, Option<i64>>(i)?.is_some_and(|v| v != 0))
}

/// `parseToLocalDate`: qualquer coisa que não seja um ISO válido vira nulo
fn data_lenient(r: &Row<'_>, i: usize) -> rusqlite::Result<Option<NaiveDate>> {
    Ok(match r.get_ref(i)? {
        ValueRef::Text(t) => std::str::from_utf8(t)
            .ok()
            .and_then(|s| NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()),
        _ => None,
    })
}

/// colunas 1..=8 comuns às consultas de CBS/IBS e IS (com o id na coluna 0)
fn classificacao_com_id(r: &Row<'_>, coluna_data: usize) -> rusqlite::Result<LinhaClassificacao> {
    Ok(LinhaClassificacao {
        id: r.get(0)?,
        codigo: r.get(1)?,
        descricao: r.get(2)?,
        tipo_aliquota: r.get(3)?,
        nomenclatura: r.get(4)?,
        descricao_tratamento: r.get(5)?,
        incompativel_com_suspensao: indicador(r, 6)?,
        exige_grupo_desoneracao: indicador(r, 7)?,
        possui_percentual_reducao: indicador(r, 8)?,
        data_atualizacao: data_lenient(r, coluna_data)?,
        creditos: None,
    })
}

/// layout de `PorIdSituacaoTributaria`/`PorCstETributoTipo` (id na coluna 8)
fn classificacao_basica(r: &Row<'_>) -> rusqlite::Result<LinhaClassificacao> {
    Ok(LinhaClassificacao {
        id: r.get(8)?,
        codigo: r.get(0)?,
        descricao: r.get(1)?,
        tipo_aliquota: r.get(2)?,
        nomenclatura: r.get(3)?,
        descricao_tratamento: r.get(4)?,
        incompativel_com_suspensao: indicador(r, 5)?,
        exige_grupo_desoneracao: indicador(r, 6)?,
        possui_percentual_reducao: indicador(r, 7)?,
        data_atualizacao: data_lenient(r, 9)?,
        creditos: None,
    })
}

const COLUNA_DATA_CBS_IBS: usize = 18;
// o serviço lê a data no índice 10 (SITR_IND_GDIF), que nunca é uma data
const COLUNA_DATA_IS: usize = 10;

fn classificacao_cbs_ibs(r: &Row<'_>) -> rusqlite::Result<LinhaClassificacao> {
    let mut l = classificacao_com_id(r, COLUNA_DATA_CBS_IBS)?;
    l.creditos = Some(CreditosCbsIbs {
        apropriacao_adquirente_cbs: indicador(r, 9)?,
        apropriacao_adquirente_ibs: indicador(r, 10)?,
        presumido_fornecedor: indicador(r, 11)?,
        presumido_adquirente: indicador(r, 12)?,
        operacao_antecedente: r.get(13)?,
        reducao_cbs: decimal(r, 14)?,
        reducao_ibs_uf: decimal(r, 15)?,
        reducao_ibs_mun: decimal(r, 16)?,
        tipo_receita_bruta_sn: r.get(19)?,
    });
    Ok(l)
}

fn classificacao_is(r: &Row<'_>) -> rusqlite::Result<LinhaClassificacao> {
    classificacao_com_id(r, COLUNA_DATA_IS)
}

fn codigo_descricao(r: &Row<'_>) -> rusqlite::Result<CodigoDescricao> {
    Ok(CodigoDescricao {
        codigo: r.get(0)?,
        descricao: r.get(1)?,
    })
}

fn cst_classificacao(r: &Row<'_>) -> rusqlite::Result<CstClassificacao> {
    Ok(CstClassificacao {
        id_cst: r.get(0)?,
        codigo_cst: r.get(1)?,
        descricao_cst: r.get(2)?,
        codigo_classificacao: r.get(3)?,
        descricao_classificacao: r.get(4)?,
    })
}

const SQL_POR_SITUACAO: &str = "SELECT ct.CLTR_CD, ct.CLTR_DESCRICAO, ct.CLTR_TIPO_ALIQUOTA, ct.CLTR_NOMENCLATURA, tt.TRTR_DESCRICAO, tt.TRTR_IN_INCOMPATIVEL_COM_SUSPENSAO, tt.TRTR_IN_EXIGE_GRUPO_DESONERACAO, st.SITR_IND_GRED, ct.CLTR_ID, ct.CLTR_DATA_ATUALIZACAO FROM TRATAMENTO_CLASSIFICACAO tc JOIN CLASSIFICACAO_TRIBUTARIA ct ON tc.TRCL_CLTR_ID = ct.CLTR_ID JOIN SITUACAO_TRIBUTARIA st ON st.SITR_ID = ct.CLTR_SITR_ID JOIN TRATAMENTO_TRIBUTARIO tt ON tt.TRTR_ID = tc.TRCL_TRTR_ID WHERE ct.CLTR_SITR_ID = :idSituacaoTributaria AND :data BETWEEN tc.TRCL_INICIO_VIGENCIA AND COALESCE(tc.TRCL_FIM_VIGENCIA, :data) AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND :data BETWEEN tt.TRTR_INICIO_VIGENCIA AND COALESCE(tt.TRTR_FIM_VIGENCIA, :data) AND :data BETWEEN st.SITR_INICIO_VIGENCIA AND COALESCE(st.SITR_FIM_VIGENCIA, :data) ORDER BY ct.CLTR_CD";

const SQL_POR_CST: &str = "SELECT ct.CLTR_CD, ct.CLTR_DESCRICAO, ct.CLTR_TIPO_ALIQUOTA, ct.CLTR_NOMENCLATURA, tt.TRTR_DESCRICAO, tt.TRTR_IN_INCOMPATIVEL_COM_SUSPENSAO, tt.TRTR_IN_EXIGE_GRUPO_DESONERACAO, st.SITR_IND_GRED, ct.CLTR_ID, ct.CLTR_DATA_ATUALIZACAO FROM TRATAMENTO_CLASSIFICACAO tc JOIN CLASSIFICACAO_TRIBUTARIA ct ON tc.TRCL_CLTR_ID = ct.CLTR_ID JOIN SITUACAO_TRIBUTARIA st ON st.SITR_ID = ct.CLTR_SITR_ID JOIN TRATAMENTO_TRIBUTARIO tt ON tt.TRTR_ID = tc.TRCL_TRTR_ID WHERE st.SITR_CD = :cst AND EXISTS ( SELECT 1 FROM TRIBUTO_SITUACAO_TRIBUTARIA tst2 JOIN TRIBUTO tb2 ON tst2.TRST_TBTO_ID = tb2.TBTO_ID WHERE tst2.TRST_SITR_ID = st.SITR_ID AND tb2.TBTO_SIGLA IN ({siglas}) AND :data BETWEEN tst2.TRST_INICIO_VIGENCIA AND COALESCE(tst2.TRST_FIM_VIGENCIA, :data) AND :data BETWEEN tb2.TBTO_INICIO_VIGENCIA AND COALESCE(tb2.TBTO_FIM_VIGENCIA, :data) ) AND :data BETWEEN tc.TRCL_INICIO_VIGENCIA AND COALESCE(tc.TRCL_FIM_VIGENCIA, :data) AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND :data BETWEEN tt.TRTR_INICIO_VIGENCIA AND COALESCE(tt.TRTR_FIM_VIGENCIA, :data) AND :data BETWEEN st.SITR_INICIO_VIGENCIA AND COALESCE(st.SITR_FIM_VIGENCIA, :data) ORDER BY ct.CLTR_CD";

const SQL_CBS_IBS: &str = "SELECT ct.CLTR_ID, ct.CLTR_CD, ct.CLTR_DESCRICAO, ct.CLTR_TIPO_ALIQUOTA, ct.CLTR_NOMENCLATURA, tt.TRTR_DESCRICAO, tt.TRTR_IN_INCOMPATIVEL_COM_SUSPENSAO, tt.TRTR_IN_EXIGE_GRUPO_DESONERACAO, st.SITR_IND_GRED, ct.CLTR_IN_APROPRIACAO_CREDITOS_ADQUIRENTES_CBS, ct.CLTR_IN_APROPRIACAO_CREDITOS_ADQUIRENTES_IBS, ct.CLTR_IN_CREDITO_PRESUMIDO_FORNECEDOR, ct.CLTR_IN_CREDITO_PRESUMIDO_ADQUIRENTE, ct.CLTR_CREDITO_OPERACAO_ANTECEDENTE, pr_cbs.PERE_VALOR AS PERCENTUAL_REDUCAO_CBS, pr_ibsuf.PERE_VALOR AS PERCENTUAL_REDUCAO_IBSUF, pr_ibsmun.PERE_VALOR AS PERCENTUAL_REDUCAO_IBSMUN, st.SITR_IND_GDIF, ct.CLTR_DATA_ATUALIZACAO, ct.CLTR_TPRBSN FROM TRATAMENTO_CLASSIFICACAO tc JOIN CLASSIFICACAO_TRIBUTARIA ct ON tc.TRCL_CLTR_ID = ct.CLTR_ID JOIN SITUACAO_TRIBUTARIA st ON st.SITR_ID = ct.CLTR_SITR_ID JOIN TRATAMENTO_TRIBUTARIO tt ON tt.TRTR_ID = tc.TRCL_TRTR_ID JOIN TRIBUTO_SITUACAO_TRIBUTARIA tst ON tst.TRST_SITR_ID = ct.CLTR_SITR_ID LEFT JOIN PERCENTUAL_REDUCAO pr_cbs ON pr_cbs.PERE_CLTR_ID = ct.CLTR_ID AND pr_cbs.PERE_TBTO_ID = 2 AND :data BETWEEN pr_cbs.PERE_INICIO_VIGENCIA AND COALESCE(pr_cbs.PERE_FIM_VIGENCIA, :data) LEFT JOIN PERCENTUAL_REDUCAO pr_ibsuf ON pr_ibsuf.PERE_CLTR_ID = ct.CLTR_ID AND pr_ibsuf.PERE_TBTO_ID = 3 AND :data BETWEEN pr_ibsuf.PERE_INICIO_VIGENCIA AND COALESCE(pr_ibsuf.PERE_FIM_VIGENCIA, :data) LEFT JOIN PERCENTUAL_REDUCAO pr_ibsmun ON pr_ibsmun.PERE_CLTR_ID = ct.CLTR_ID AND pr_ibsmun.PERE_TBTO_ID = 4 AND :data BETWEEN pr_ibsmun.PERE_INICIO_VIGENCIA AND COALESCE(pr_ibsmun.PERE_FIM_VIGENCIA, :data) WHERE :data BETWEEN tc.TRCL_INICIO_VIGENCIA AND COALESCE(tc.TRCL_FIM_VIGENCIA, :data) AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND :data BETWEEN tt.TRTR_INICIO_VIGENCIA AND COALESCE(tt.TRTR_FIM_VIGENCIA, :data) AND :data BETWEEN tst.TRST_INICIO_VIGENCIA AND COALESCE(tst.TRST_FIM_VIGENCIA, :data) AND tst.TRST_TBTO_ID == 2 AND (:codigo IS NULL OR ct.CLTR_CD = :codigo) ORDER BY ct.CLTR_CD";

const SQL_IS: &str = "SELECT ct.CLTR_ID, ct.CLTR_CD, ct.CLTR_DESCRICAO, ct.CLTR_TIPO_ALIQUOTA, ct.CLTR_NOMENCLATURA, tt.TRTR_DESCRICAO, tt.TRTR_IN_INCOMPATIVEL_COM_SUSPENSAO, tt.TRTR_IN_EXIGE_GRUPO_DESONERACAO, st.SITR_IND_GRED, tt.TRTR_IN_POSSUI_PERCENTUAL_REDUCAO, st.SITR_IND_GDIF, ct.CLTR_DATA_ATUALIZACAO FROM TRATAMENTO_CLASSIFICACAO tc JOIN CLASSIFICACAO_TRIBUTARIA ct ON tc.TRCL_CLTR_ID = ct.CLTR_ID JOIN SITUACAO_TRIBUTARIA st ON st.SITR_ID = ct.CLTR_SITR_ID JOIN TRATAMENTO_TRIBUTARIO tt ON tt.TRTR_ID = tc.TRCL_TRTR_ID JOIN TRIBUTO_SITUACAO_TRIBUTARIA tst ON tst.TRST_SITR_ID = ct.CLTR_SITR_ID WHERE :data BETWEEN tc.TRCL_INICIO_VIGENCIA AND COALESCE(tc.TRCL_FIM_VIGENCIA, :data) AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND :data BETWEEN tt.TRTR_INICIO_VIGENCIA AND COALESCE(tt.TRTR_FIM_VIGENCIA, :data) AND :data BETWEEN tst.TRST_INICIO_VIGENCIA AND COALESCE(tst.TRST_FIM_VIGENCIA, :data) AND :data BETWEEN st.SITR_INICIO_VIGENCIA AND COALESCE(st.SITR_FIM_VIGENCIA, :data) AND tst.TRST_TBTO_ID == 1 AND (:codigo IS NULL OR ct.CLTR_CD = :codigo) ORDER BY ct.CLTR_CD";

const SQL_VALIDADE_DFE: &str = "SELECT ct.CLTR_ID, ct.CLTR_NOMENCLATURA, tt.TRTR_IN_EXIGE_GRUPO_DESONERACAO, st.SITR_IND_GDIF, ct.CLTR_IND_GCREDPRESOPER, st.SITR_IND_GIBSCBS FROM TRATAMENTO_CLASSIFICACAO tc INNER JOIN CLASSIFICACAO_TRIBUTARIA ct ON tc.TRCL_CLTR_ID = ct.CLTR_ID AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) INNER JOIN SITUACAO_TRIBUTARIA st ON st.SITR_ID = ct.CLTR_SITR_ID AND :data BETWEEN st.SITR_INICIO_VIGENCIA AND COALESCE(st.SITR_FIM_VIGENCIA, :data) INNER JOIN TRATAMENTO_TRIBUTARIO tt ON tt.TRTR_ID = tc.TRCL_TRTR_ID AND :data BETWEEN tt.TRTR_INICIO_VIGENCIA AND COALESCE(tt.TRTR_FIM_VIGENCIA, :data) INNER JOIN TRIBUTO_SITUACAO_TRIBUTARIA tst ON tst.TRST_SITR_ID = st.SITR_ID AND tst.TRST_TBTO_ID = 2 AND :data BETWEEN tst.TRST_INICIO_VIGENCIA AND COALESCE(tst.TRST_FIM_VIGENCIA, :data) WHERE ct.CLTR_CD = :codigoClassificacaoTributaria AND :data BETWEEN tc.TRCL_INICIO_VIGENCIA AND COALESCE(tc.TRCL_FIM_VIGENCIA, :data) LIMIT 1";

const SQL_CSTS_CBS_IBS_POR_SIGLA: &str = "SELECT DISTINCT st.SITR_ID as id_cst, st.SITR_CD as codigo_cst, st.SITR_DESCRICAO as descricao_cst, ct.CLTR_CD as codigo_classificacao, ct.CLTR_DESCRICAO as descricao_classificacao FROM TIPO_DFE_CLASSIFICACAO tdcl JOIN TIPO_DFE td ON tdcl.TDCL_TPDF_ID = td.TPDF_ID JOIN CLASSIFICACAO_TRIBUTARIA ct ON tdcl.TDCL_CLTR_ID = ct.CLTR_ID JOIN SITUACAO_TRIBUTARIA st ON ct.CLTR_SITR_ID = st.SITR_ID JOIN TRIBUTO_SITUACAO_TRIBUTARIA tst ON tst.TRST_SITR_ID = st.SITR_ID AND tst.TRST_TBTO_ID != 1 AND :data BETWEEN tst.TRST_INICIO_VIGENCIA AND COALESCE(tst.TRST_FIM_VIGENCIA, :data) WHERE UPPER(REPLACE(REPLACE(REPLACE(td.TPDF_SIGLA, ' ', ''), '-', ''), '_', '')) = :siglaDfe AND :data BETWEEN tdcl.TDCL_INICIO_VIGENCIA AND COALESCE(tdcl.TDCL_FIM_VIGENCIA, :data) ORDER BY st.SITR_CD ASC, ct.CLTR_CD ASC";

const SQL_CSTS_CBS_IBS: &str = "SELECT DISTINCT st.SITR_ID as id_cst, st.SITR_CD as codigo_cst, st.SITR_DESCRICAO as descricao_cst, ct.CLTR_CD as codigo_classificacao, ct.CLTR_DESCRICAO as descricao_classificacao FROM CLASSIFICACAO_TRIBUTARIA ct JOIN SITUACAO_TRIBUTARIA st ON ct.CLTR_SITR_ID = st.SITR_ID JOIN TRIBUTO_SITUACAO_TRIBUTARIA tst ON tst.TRST_SITR_ID = st.SITR_ID AND tst.TRST_TBTO_ID != 1 AND :data BETWEEN tst.TRST_INICIO_VIGENCIA AND COALESCE(tst.TRST_FIM_VIGENCIA, :data) ORDER BY st.SITR_CD ASC, ct.CLTR_CD ASC";

const SQL_CSTS_IS: &str = "SELECT DISTINCT st.SITR_ID as id_cst, st.SITR_CD as codigo_cst, st.SITR_DESCRICAO as descricao_cst, ct.CLTR_CD as codigo_classificacao, ct.CLTR_DESCRICAO as descricao_classificacao FROM CLASSIFICACAO_TRIBUTARIA ct JOIN SITUACAO_TRIBUTARIA st ON ct.CLTR_SITR_ID = st.SITR_ID JOIN TRIBUTO_SITUACAO_TRIBUTARIA tst ON tst.TRST_SITR_ID = st.SITR_ID AND tst.TRST_TBTO_ID = 1 AND :data BETWEEN tst.TRST_INICIO_VIGENCIA AND COALESCE(tst.TRST_FIM_VIGENCIA, :data) ORDER BY st.SITR_CD ASC, ct.CLTR_CD ASC";

const SQL_NBS_APLICAVEIS: &str = "SELECT DISTINCT nbs.NBS_CD, nbs.NBS_DESCRICAO FROM NBS_APLICAVEL n JOIN NBS nbs ON nbs.NBS_CD = n.NBSA_NBS_CD LEFT JOIN EXCECAO_NBS_APLICAVEL e ON e.ENBS_NBSA_ID = n.NBSA_ID AND :data BETWEEN e.ENBS_INICIO_VIGENCIA AND COALESCE(e.ENBS_FIM_VIGENCIA, :data) JOIN CLASSIFICACAO_TRIBUTARIA ct ON ct.CLTR_ID = n.NBSA_CLTR_ID WHERE ct.CLTR_ID = :classificacaoId AND :data BETWEEN n.NBSA_INICIO_VIGENCIA AND COALESCE(n.NBSA_FIM_VIGENCIA, :data) AND :data BETWEEN nbs.NBS_INICIO_VIGENCIA AND COALESCE(nbs.NBS_FIM_VIGENCIA, :data) AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND e.ENBS_ID IS NULL AND LENGTH(nbs.NBS_CD) = 9 ORDER BY nbs.NBS_CD";

const SQL_EXISTE_VINCULO_NBS: &str = "SELECT CASE WHEN COUNT(*) > 0 THEN 1 ELSE 0 END FROM NBS_APLICAVEL n JOIN CLASSIFICACAO_TRIBUTARIA ct ON ct.CLTR_ID = n.NBSA_CLTR_ID WHERE ct.CLTR_ID = :classificacaoId";

const SQL_CLASSIFICACOES_VINCULADAS_NBS: &str = "SELECT ct.CLTR_CD FROM NBS_APLICAVEL n JOIN CLASSIFICACAO_TRIBUTARIA ct ON ct.CLTR_ID = n.NBSA_CLTR_ID LEFT JOIN EXCECAO_NBS_APLICAVEL e ON e.ENBS_NBSA_ID = n.NBSA_ID AND :data BETWEEN e.ENBS_INICIO_VIGENCIA AND COALESCE(e.ENBS_FIM_VIGENCIA, :data) WHERE n.NBSA_NBS_CD = :nbs AND :data BETWEEN n.NBSA_INICIO_VIGENCIA AND COALESCE(n.NBSA_FIM_VIGENCIA, :data) AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND e.ENBS_ID IS NULL";

const SQL_SERVICO_SEM_VINCULO_NBS: &str = "SELECT ct.CLTR_CD FROM CLASSIFICACAO_TRIBUTARIA ct WHERE (ct.CLTR_NOMENCLATURA LIKE '%NBS%') AND :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND ct.CLTR_ID NOT IN ( SELECT n.NBSA_CLTR_ID FROM NBS_APLICAVEL n WHERE :data BETWEEN n.NBSA_INICIO_VIGENCIA AND COALESCE(n.NBSA_FIM_VIGENCIA, :data) )";

const SQL_POR_ATORES: &str = "SELECT ct.CLTR_CD FROM CLASSIFICACAO_TRIBUTARIA ct WHERE :data BETWEEN ct.CLTR_INICIO_VIGENCIA AND COALESCE(ct.CLTR_FIM_VIGENCIA, :data) AND ( :fornecedor IS NULL OR NOT EXISTS (SELECT 1 FROM ATOR_CLASSIFICACAO ac WHERE ac.ATCL_CLTR_ID = ct.CLTR_ID AND ac.ATCL_IN_PAPEL = 'Fornecedor' AND :data BETWEEN ac.ATCL_INICIO_VIGENCIA AND COALESCE(ac.ATCL_FIM_VIGENCIA, :data)) OR EXISTS (SELECT 1 FROM ATOR_CLASSIFICACAO ac WHERE ac.ATCL_CLTR_ID = ct.CLTR_ID AND ac.ATCL_IN_PAPEL = 'Fornecedor' AND ac.ATCL_ATOR_ID = :fornecedor AND :data BETWEEN ac.ATCL_INICIO_VIGENCIA AND COALESCE(ac.ATCL_FIM_VIGENCIA, :data)) ) AND ( :adquirente IS NULL OR NOT EXISTS (SELECT 1 FROM ATOR_CLASSIFICACAO ac WHERE ac.ATCL_CLTR_ID = ct.CLTR_ID AND ac.ATCL_IN_PAPEL = 'Adquirente' AND :data BETWEEN ac.ATCL_INICIO_VIGENCIA AND COALESCE(ac.ATCL_FIM_VIGENCIA, :data)) OR EXISTS (SELECT 1 FROM ATOR_CLASSIFICACAO ac WHERE ac.ATCL_CLTR_ID = ct.CLTR_ID AND ac.ATCL_IN_PAPEL = 'Adquirente' AND ac.ATCL_ATOR_ID = :adquirente AND :data BETWEEN ac.ATCL_INICIO_VIGENCIA AND COALESCE(ac.ATCL_FIM_VIGENCIA, :data)) ) AND ( :siglaDfe IS NULL OR EXISTS (SELECT 1 FROM TIPO_DFE_CLASSIFICACAO tdcl JOIN TIPO_DFE td ON tdcl.TDCL_TPDF_ID = td.TPDF_ID WHERE tdcl.TDCL_CLTR_ID = ct.CLTR_ID AND UPPER(REPLACE(REPLACE(REPLACE(td.TPDF_SIGLA, ' ', ''), '-', ''), '_', '')) = :siglaDfe AND :data BETWEEN tdcl.TDCL_INICIO_VIGENCIA AND COALESCE(tdcl.TDCL_FIM_VIGENCIA, :data)) )";

const SQL_GRUPOS_DFE: &str = "SELECT g.GRDF_ID AS grupo_id, g.GRDF_DESCRICAO AS grupo_titulo, t.TPDF_SIGLA AS tipo_sigla, t.TPDF_DESCRICAO AS tipo_titulo, t.TPDF_TIPO AS tipo_modelo FROM TIPO_DFE t JOIN GRUPO_DFE g ON t.TPDF_GRDF_ID = g.GRDF_ID WHERE :data BETWEEN t.TPDF_INICIO_VIGENCIA AND COALESCE(t.TPDF_FIM_VIGENCIA, :data) AND :data BETWEEN g.GRDF_INICIO_VIGENCIA AND COALESCE(g.GRDF_FIM_VIGENCIA, :data) ORDER BY g.GRDF_ID ASC, t.TPDF_ID ASC";

const SQL_GRUPOS_ATOR: &str = "SELECT g.GRAT_ID AS grupo_id, g.GRAT_DESCRICAO AS grupo_descricao, g.GRAT_ORDEM AS grupo_ordem, a.ATOR_ID AS ator_id, a.ATOR_DESCRICAO AS ator_descricao, a.ATOR_ORDEM AS ator_ordem FROM ATOR a JOIN GRUPO_ATOR g ON a.ATOR_GRAT_ID = g.GRAT_ID WHERE a.ATOR_INICIO_VIGENCIA <= :data AND (a.ATOR_FIM_VIGENCIA IS NULL OR a.ATOR_FIM_VIGENCIA >= :data) AND g.GRAT_INICIO_VIGENCIA <= :data AND (g.GRAT_FIM_VIGENCIA IS NULL OR g.GRAT_FIM_VIGENCIA >= :data) AND ( :papel IS NULL OR EXISTS ( SELECT 1 FROM ATOR_CLASSIFICACAO ac WHERE ac.ATCL_ATOR_ID = a.ATOR_ID AND ac.ATCL_IN_PAPEL = :papel AND ac.ATCL_INICIO_VIGENCIA <= :data AND (ac.ATCL_FIM_VIGENCIA IS NULL OR ac.ATCL_FIM_VIGENCIA >= :data) ) ) ORDER BY g.GRAT_ORDEM ASC, a.ATOR_ORDEM ASC";

const SQL_TRIBUTO_DA_SITUACAO: &str = "SELECT t.TRST_TBTO_ID FROM TRIBUTO_SITUACAO_TRIBUTARIA t JOIN SITUACAO_TRIBUTARIA s ON t.TRST_SITR_ID = s.SITR_ID WHERE t.TRST_SITR_ID = :idSituacaoTributaria AND :data BETWEEN t.TRST_INICIO_VIGENCIA AND COALESCE(t.TRST_FIM_VIGENCIA, :data) LIMIT 1";

impl Base<'_> {
    pub fn ufs(&self) -> Resultado<Vec<Uf>> {
        self.linhas(
            "select u1_0.uf_cd,u1_0.uf_nome,u1_0.uf_sigla from uf u1_0",
            [],
            |r| {
                Ok(Uf {
                    codigo: r.get(0)?,
                    nome: r.get(1)?,
                    sigla: r.get(2)?,
                })
            },
        )
    }

    pub fn municipios_da_uf(&self, sigla_uf: &str) -> Resultado<Vec<Municipio>> {
        self.linhas(
            "select m1_0.muni_cd,m1_0.muni_nome,m1_0.muni_uf_cd from municipio m1_0 join uf u1_0 on u1_0.uf_cd=m1_0.muni_uf_cd where lower(u1_0.uf_sigla)=lower(?)",
            [sigla_uf],
            |r| {
                Ok(Municipio {
                    codigo: r.get(0)?,
                    nome: r.get(1)?,
                })
            },
        )
    }

    pub fn situacoes_do_tributo(
        &self,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<Vec<SituacaoResumo>> {
        let d = data(dia);
        self.linhas(
            "select st1_0.sitr_id,st1_0.sitr_cd,st1_0.sitr_descricao,st1_0.sitr_fim_vigencia,st1_0.sitr_ind_gajustecompet,st1_0.sitr_ind_gcredpresibszfm,st1_0.sitr_ind_gdif,st1_0.sitr_ind_gibscbs,st1_0.sitr_ind_gibscbsmono,st1_0.sitr_ind_gred,st1_0.sitr_ind_gtransfcred,st1_0.sitr_inicio_vigencia from tributo_situacao_tributaria tst1_0 join situacao_tributaria st1_0 on st1_0.sitr_id=tst1_0.trst_sitr_id where tst1_0.trst_tbto_id=? and tst1_0.trst_inicio_vigencia<=? and (tst1_0.trst_fim_vigencia is null or tst1_0.trst_fim_vigencia>=?)",
            params![id_tributo, d, d],
            |r| {
                Ok(SituacaoResumo {
                    id: r.get(0)?,
                    codigo: r.get(1)?,
                    descricao: r.get(2)?,
                })
            },
        )
    }

    pub fn classificacoes_da_situacao(
        &self,
        id_situacao: i64,
        dia: NaiveDate,
    ) -> Resultado<Vec<LinhaClassificacao>> {
        let d = data(dia);
        self.linhas(
            SQL_POR_SITUACAO,
            named_params! {":idSituacaoTributaria": id_situacao, ":data": d},
            classificacao_basica,
        )
    }

    pub fn classificacoes_do_cst(
        &self,
        cst: &str,
        siglas_tributo: &[&str],
        dia: NaiveDate,
    ) -> Resultado<Vec<LinhaClassificacao>> {
        let d = data(dia);
        let nomes: Vec<String> = (0..siglas_tributo.len())
            .map(|i| format!(":tbtoSiglas{i}"))
            .collect();
        let sql = SQL_POR_CST.replace("{siglas}", &nomes.join(", "));
        let mut parametros: Vec<(&str, &dyn ToSql)> = vec![(":cst", &cst), (":data", &d)];
        for (nome, sigla) in nomes.iter().zip(siglas_tributo) {
            parametros.push((nome.as_str(), sigla));
        }
        self.linhas(&sql, parametros.as_slice(), classificacao_basica)
    }

    pub fn classificacoes_cbs_ibs(
        &self,
        codigo: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<Vec<LinhaClassificacao>> {
        let d = data(dia);
        self.linhas(
            SQL_CBS_IBS,
            named_params! {":data": d, ":codigo": codigo},
            classificacao_cbs_ibs,
        )
    }

    pub fn classificacoes_is(
        &self,
        codigo: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<Vec<LinhaClassificacao>> {
        let d = data(dia);
        self.linhas(
            SQL_IS,
            named_params! {":data": d, ":codigo": codigo},
            classificacao_is,
        )
    }

    pub fn validade_dfe(&self, codigo: &str, dia: NaiveDate) -> Resultado<Option<ValidadeDfe>> {
        let d = data(dia);
        let mut linhas = self.linhas(
            SQL_VALIDADE_DFE,
            named_params! {":data": d, ":codigoClassificacaoTributaria": codigo},
            |r| {
                Ok(ValidadeDfe {
                    id_classificacao: r.get(0)?,
                    nomenclatura: r.get(1)?,
                    exige_grupo_regular: r.get(2)?,
                    permite_diferimento: r.get(3)?,
                    credito_presumido: r.get(4)?,
                    exige_grupo_ibs_cbs: r.get(5)?,
                })
            },
        )?;
        Ok(linhas.pop())
    }

    /// `siglaDfe` já normalizada; `None` lista as de todos os DFes
    pub fn csts_cbs_ibs(
        &self,
        sigla: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<Vec<CstClassificacao>> {
        let d = data(dia);
        match sigla {
            Some(s) => self.linhas(
                SQL_CSTS_CBS_IBS_POR_SIGLA,
                named_params! {":siglaDfe": s, ":data": d},
                cst_classificacao,
            ),
            None => self.linhas(
                SQL_CSTS_CBS_IBS,
                named_params! {":data": d},
                cst_classificacao,
            ),
        }
    }

    pub fn csts_is(&self, dia: NaiveDate) -> Resultado<Vec<CstClassificacao>> {
        let d = data(dia);
        self.linhas(SQL_CSTS_IS, named_params! {":data": d}, cst_classificacao)
    }

    pub fn descricao_nomenclatura(
        &self,
        n: super::Nomenclatura,
        codigo: &str,
        limite: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<String>> {
        let tabela = match n {
            super::Nomenclatura::Ncm => "ncm",
            super::Nomenclatura::Nbs => "nbs",
        };
        let sql = format!(
            "select ltrim(rtrim(n1_0.{t}_descricao,' '),' -') from {t} n1_0 where n1_0.{t}_cd=substr(?,1,?) and ? between n1_0.{t}_inicio_vigencia and coalesce(n1_0.{t}_fim_vigencia,?)",
            t = tabela
        );
        let d = data(dia);
        let v = self.unica(&sql, params![codigo, limite, d, d], |r| {
            r.get::<_, Option<String>>(0)
        })?;
        Ok(v.flatten())
    }

    pub fn ncms_vigentes(&self, dia: NaiveDate) -> Resultado<Vec<CodigoDescricao>> {
        let d = data(dia);
        self.linhas(
            "select distinct n1_0.ncm_cd,n1_0.ncm_descricao from ncm n1_0 where ? between n1_0.ncm_inicio_vigencia and coalesce(n1_0.ncm_fim_vigencia,?) and length(n1_0.ncm_cd)=8 order by n1_0.ncm_cd",
            params![d, d],
            codigo_descricao,
        )
    }

    pub fn nbs_vigentes(&self, dia: NaiveDate) -> Resultado<Vec<CodigoDescricao>> {
        let d = data(dia);
        self.linhas(
            "select n1_0.nbs_cd,n1_0.nbs_descricao,n1_0.nbs_fim_vigencia,n1_0.nbs_inicio_vigencia,n1_0.nbs_lc_116 from nbs n1_0 where ? between n1_0.nbs_inicio_vigencia and coalesce(n1_0.nbs_fim_vigencia,?) and length(n1_0.nbs_cd)=9 order by n1_0.nbs_cd",
            params![d, d],
            codigo_descricao,
        )
    }

    pub fn nbs_aplicaveis(
        &self,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<Vec<CodigoDescricao>> {
        let d = data(dia);
        self.linhas(
            SQL_NBS_APLICAVEIS,
            named_params! {":data": d, ":classificacaoId": id_classificacao},
            codigo_descricao,
        )
    }

    pub fn existe_vinculo_nbs(&self, id_classificacao: i64) -> Resultado<bool> {
        let v = self.unica(
            SQL_EXISTE_VINCULO_NBS,
            named_params! {":classificacaoId": id_classificacao},
            |r| r.get::<_, i64>(0),
        )?;
        Ok(v == Some(1))
    }

    pub fn classificacoes_vinculadas_nbs(
        &self,
        nbs: &str,
        dia: NaiveDate,
    ) -> Resultado<Vec<String>> {
        let d = data(dia);
        self.linhas(
            SQL_CLASSIFICACOES_VINCULADAS_NBS,
            named_params! {":data": d, ":nbs": nbs},
            |r| r.get(0),
        )
    }

    pub fn classificacoes_servico_sem_vinculo_nbs(&self, dia: NaiveDate) -> Resultado<Vec<String>> {
        let d = data(dia);
        self.linhas(
            SQL_SERVICO_SEM_VINCULO_NBS,
            named_params! {":data": d},
            |r| r.get(0),
        )
    }

    pub fn classificacoes_por_atores(
        &self,
        dia: NaiveDate,
        fornecedor: Option<i64>,
        adquirente: Option<i64>,
        sigla: Option<&str>,
    ) -> Resultado<Vec<String>> {
        let d = data(dia);
        self.linhas(
            SQL_POR_ATORES,
            named_params! {
                ":data": d,
                ":fornecedor": fornecedor,
                ":adquirente": adquirente,
                ":siglaDfe": sigla,
            },
            |r| r.get(0),
        )
    }

    pub fn grupos_dfe(&self, dia: NaiveDate) -> Resultado<Vec<LinhaGrupoDfe>> {
        let d = data(dia);
        self.linhas(SQL_GRUPOS_DFE, named_params! {":data": d}, |r| {
            Ok(LinhaGrupoDfe {
                grupo_id: r.get(0)?,
                grupo_titulo: r.get(1)?,
                sigla: r.get(2)?,
                titulo: r.get(3)?,
                modelo: r.get(4)?,
            })
        })
    }

    pub fn grupos_ator(
        &self,
        dia: NaiveDate,
        papel: Option<&str>,
    ) -> Resultado<Vec<LinhaGrupoAtor>> {
        let d = data(dia);
        self.linhas(
            SQL_GRUPOS_ATOR,
            named_params! {":data": d, ":papel": papel},
            |r| {
                Ok(LinhaGrupoAtor {
                    grupo_id: r.get(0)?,
                    grupo_descricao: r.get(1)?,
                    grupo_ordem: r.get(2)?,
                    ator_id: r.get(3)?,
                    ator_descricao: r.get(4)?,
                    ator_ordem: r.get(5)?,
                })
            },
        )
    }

    pub fn tributo_da_situacao(&self, id_situacao: i64, dia: NaiveDate) -> Resultado<Option<i64>> {
        let d = data(dia);
        let mut v = self.linhas(
            SQL_TRIBUTO_DA_SITUACAO,
            named_params! {":idSituacaoTributaria": id_situacao, ":data": d},
            |r| r.get::<_, i64>(0),
        )?;
        Ok(v.pop())
    }

    /// `FundamentacaoClassificacaoRepository.buscarTodas` seguido da carga dos
    /// `@ManyToOne` (classificação + situação e fundamentação legal) por id
    pub fn fundamentacoes_legais(&self, dia: NaiveDate) -> Resultado<Vec<FundamentacaoLegal>> {
        let d = data(dia);
        let vinculos = self.linhas(
            "select fc1_0.fdcl_id,fc1_0.fdcl_cltr_id,fc1_0.fdcl_fim_vigencia,fc1_0.fdcl_fdlg_id,fc1_0.fdcl_inicio_vigencia from fundamentacao_classificacao fc1_0 join classificacao_tributaria ct1_0 on ct1_0.cltr_id=fc1_0.fdcl_cltr_id where ? between fc1_0.fdcl_inicio_vigencia and coalesce(fc1_0.fdcl_fim_vigencia,?) order by ct1_0.cltr_cd",
            params![d, d],
            |r| Ok((r.get::<_, i64>(1)?, r.get::<_, i64>(3)?)),
        )?;
        vinculos
            .into_iter()
            .map(|(id_classificacao, id_fundamentacao)| {
                self.fundamentacao_legal(id_classificacao, id_fundamentacao)
            })
            .collect()
    }

    fn fundamentacao_legal(
        &self,
        id_classificacao: i64,
        id_fundamentacao: i64,
    ) -> Resultado<FundamentacaoLegal> {
        let classificacao = self
            .unica(
                "select ct1_0.cltr_cd,ct1_0.cltr_descricao,st1_0.sitr_id,st1_0.sitr_cd,st1_0.sitr_descricao from classificacao_tributaria ct1_0 join situacao_tributaria st1_0 on st1_0.sitr_id=ct1_0.cltr_sitr_id where ct1_0.cltr_id=?",
                [id_classificacao],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, i64>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, String>(4)?,
                    ))
                },
            )?
            .ok_or_else(|| entidade_ausente("ClassificacaoTributaria", id_classificacao))?;
        let legal = self
            .unica(
                "select fl1_0.fdlg_texto,fl1_0.fdlg_texto_curto,fl1_0.fdlg_referencia_normativa from fundamentacao_legal fl1_0 where fl1_0.fdlg_id=?",
                [id_fundamentacao],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?)),
            )?
            .ok_or_else(|| entidade_ausente("FundamentacaoLegal", id_fundamentacao))?;
        let (
            codigo_classificacao,
            descricao_classificacao,
            id_situacao,
            codigo_situacao,
            descricao_situacao,
        ) = classificacao;
        let (texto, texto_curto, referencia_normativa) = legal;
        Ok(FundamentacaoLegal {
            codigo_classificacao,
            descricao_classificacao,
            id_situacao,
            codigo_situacao,
            descricao_situacao,
            texto,
            texto_curto,
            referencia_normativa,
        })
    }
}
