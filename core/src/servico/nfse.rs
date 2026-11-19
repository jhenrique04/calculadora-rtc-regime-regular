use chrono::NaiveDate;

use super::Servicos;
use super::base_calculo::validar_extintos;
use super::nomenclatura::nao_encontrada;
use super::texto::data_java;
use crate::db::{CombinacaoIndicador, LinhaIndicador, Nomenclatura, SituacaoClassificacao};
use crate::decimal::BigDecimal;
use crate::erro::{Classe, Falha, Resultado};
use crate::modelo::nfse::{NfseBaseCalculoEntrada, NfseIndicadorOperacaoEntrada};
use crate::saida_json;

const ANO_INICIO_CBS_IBS: i32 = 2026;
const ANO_EXTINCAO_PIS_COFINS: i32 = 2027;
const ANO_EXTINCAO_ISS: i32 = 2033;
const CCLASSTRIB_PADRAO: &str = "000001";
const TAM_C_TRIB_NAC_CURTO: usize = 4;
const TAM_C_TRIB_NAC_LONGO: usize = 6;

/// `LocalFornecimento` (código, descrição)
const LOCAIS_FORNECIMENTO: [(i32, &str); 28] = [
    (
        1,
        "Endereço diverso do fornecedor, adquirente ou destinatário",
    ),
    (
        2,
        "Local da prestação do serviço diverso do estabelecimento do fornecedor",
    ),
    (3, "Endereço do destinatário"),
    (4, "Endereço fornecido para entrega do bem"),
    (5, "Estabelecimento do fornecedor como local da prestação"),
    (6, "Local do porto como local da prestação"),
    (
        7,
        "Local da prestação correspondente à extensão da via explorada, proporcionalmente ao território dos entes tributantes",
    ),
    (8, "Local da retirada do bem"),
    (9, "Local de início do transporte"),
    (
        10,
        "Local do domicílio principal do destinatário residente ou domiciliado no País, caso o adquirente não seja residente ou domiciliado no País",
    ),
    (
        11,
        "Local do domicílio principal do adquirente residente ou domiciliado no País. Nas aquisições indicadas no art. 11, §4º, II, considera-se domicílio principal do adquirente o estabelecimento matriz",
    ),
    (
        12,
        "local do domicílio principal do destinatário residente ou domiciliado no País",
    ),
    (13, "Local do evento a que se refere o serviço"),
    (14, "Localidade do imóvel"),
    (
        15,
        "Estabelecimento do fornecedor como o local da entrega ou disponibilização do bem ao destinatário",
    ),
    (16, "Local da realização da operação e sua retirada"),
    (
        17,
        "Endereço do destinatário fornecido para entrega ou disponibilização (assim considerado o destino final indicado pelo adquirente)",
    ),
    (18, "Local onde se encontra o bem móvel material"),
    (
        19,
        "Local onde se encontra o bem móvel material (Local da retirada)",
    ),
    (
        20,
        "Local do domicílio principal do adquirente (estabelecimento matriz)",
    ),
    (21, "Domicílio principal do destinatário"),
    (22, "Local de instalação do terminal"),
    (
        23,
        "Local do domicílio principal do adquirente (assim considerada a matriz), nas aquisições realizadas de forma centralizada por contribuinte sujeito ao regime regular do IBS e da CBS que possui mais de um estabelecimento e que não estejam sujeitas a vedação à apropriação de créditos",
    ),
    (24, "Local da entrega ou disponibilização"),
    (
        25,
        "Local do estabelecimento principal do adquirente (assim considerada a matriz), conforme §4º do art. 11",
    ),
    (
        26,
        "Local do estabelecimento do agente ou de seus representados que figurem na posição devedora da liquidação financeira apurada pela Câmara de Comercialização de Energia Elétrica",
    ),
    (27, "Local do estabelecimento principal do fornecedor"),
    (28, "Local do estabelecimento principal do adquirente"),
];

/// `LocalIncidencia` (código, descrição)
const LOCAIS_INCIDENCIA: [(i32, &str); 7] = [
    (1, "Endereço do Destinatário"),
    (2, "Endereço do Tomador/Adquirente"),
    (3, "Estabelecimento do Prestador/Fornecedor"),
    (4, "Local da Prestação"),
    (5, "Local do Evento"),
    (6, "Localidade do Imóvel"),
    (7, "Localização do Trecho"),
];

#[derive(Debug, Clone)]
pub struct BaseCalculoNfseSaida {
    pub base_calculo: Option<BigDecimal>,
}

saida_json!(BaseCalculoNfseSaida {
    "baseCalculo" => base_calculo: decimal,
});

#[derive(Debug, Clone)]
pub struct CodigoDescricaoNfseSaida {
    pub codigo: Option<i32>,
    pub descricao: Option<String>,
}

saida_json!(CodigoDescricaoNfseSaida {
    "codigo" => codigo: inteiro,
    "descricao" => descricao: texto,
});

#[derive(Debug, Clone)]
pub struct ValidacaoIndicadorSaida {
    pub c_trib_nac: Option<String>,
    pub c_ind_op: Option<String>,
    pub c_class_trib: Option<String>,
    pub nbs: Option<String>,
    pub data: Option<NaiveDate>,
    pub valido: bool,
}

saida_json!(ValidacaoIndicadorSaida {
    "cTribNac" => c_trib_nac: texto,
    "cIndOp" => c_ind_op: texto,
    "cClassTrib" => c_class_trib: texto,
    "nbs" => nbs: texto,
    "dataOcorrenciaFatoGerador" => data: data,
    "valido" => valido: booleano,
});

#[derive(Debug, Clone)]
pub struct LocalOperacaoSaida {
    pub c_ind_op: Option<String>,
    pub data: Option<NaiveDate>,
    pub codigo_fornecimento: Option<i32>,
    pub fornecimento: Option<String>,
    pub codigo_incidencia: Option<i32>,
    pub incidencia: Option<String>,
}

saida_json!(LocalOperacaoSaida {
    "cIndOp" => c_ind_op: texto,
    "dataOcorrenciaFatoGerador" => data: data,
    "codigoLocalFornecimento" => codigo_fornecimento: inteiro,
    "localFornecimento" => fornecimento: texto,
    "codigoLocalIncidencia" => codigo_incidencia: inteiro,
    "localIncidencia" => incidencia: texto,
});

#[derive(Debug, Clone)]
pub struct IndicadorOperacaoSaida {
    pub c_ind_op: Option<String>,
    pub tipo_operacao: Option<String>,
    pub codigo_fornecimento: Option<i32>,
    pub fornecimento: Option<String>,
    pub codigo_incidencia: Option<i32>,
    pub incidencia: Option<String>,
    pub onerosa: Option<bool>,
    pub adquirente_exterior: Option<bool>,
}

saida_json!(IndicadorOperacaoSaida {
    "cIndOp" => c_ind_op: texto,
    "tipoOperacao" => tipo_operacao: texto,
    "codigoLocalFornecimento" => codigo_fornecimento: inteiro,
    "localFornecimento" => fornecimento: texto,
    "codigoLocalIncidencia" => codigo_incidencia: inteiro,
    "localIncidencia" => incidencia: texto,
    "prestacaoServicoOnerosa" => onerosa: logico,
    "adquirenteExterior" => adquirente_exterior: logico,
});

#[derive(Debug, Clone)]
pub struct SituacaoClassificacaoSaida {
    pub codigo_situacao: Option<String>,
    pub descricao_situacao: Option<String>,
    pub codigo_classificacao: Option<String>,
    pub descricao_classificacao: Option<String>,
}

saida_json!(SituacaoClassificacaoSaida {
    "codigoSituacaoTributaria" => codigo_situacao: texto,
    "descricaoSituacaoTributaria" => descricao_situacao: texto,
    "codigoClassificacaoTributaria" => codigo_classificacao: texto,
    "descricaoClassificacaoTributaria" => descricao_classificacao: texto,
});

/// qual dos dois enums de local (só muda a tabela)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TipoLocal {
    Fornecimento,
    Incidencia,
}

impl TipoLocal {
    fn tabela(self) -> &'static [(i32, &'static str)] {
        match self {
            Self::Fornecimento => &LOCAIS_FORNECIMENTO,
            Self::Incidencia => &LOCAIS_INCIDENCIA,
        }
    }

    /// `fromCodigo`: `None` reproduz o `IllegalArgumentException`
    fn descricao(self, codigo: i32) -> Option<&'static str> {
        self.tabela()
            .iter()
            .find(|(c, _)| *c == codigo)
            .map(|(_, d)| *d)
    }
}

fn codigo_invalido(codigo: i64) -> String {
    format!("Código inválido: {codigo}")
}

fn campo_invalido(mensagem: impl Into<String>) -> Falha {
    Falha::new(Classe::CampoInvalido, mensagem)
}

fn npe() -> Falha {
    Falha::sem_mensagem()
}

/// coluna inteira lida do banco convertida pelo `fromCodigo`
fn local_do_banco(
    tipo: TipoLocal,
    codigo: Option<i64>,
) -> Resultado<(Option<i32>, Option<String>)> {
    let Some(c) = codigo else {
        return Ok((None, None));
    };
    let descricao = i32::try_from(c)
        .ok()
        .and_then(|n| tipo.descricao(n).map(|d| (n, d)));
    match descricao {
        Some((n, d)) => Ok((Some(n), Some(d.to_string()))),
        None => Err(Falha::sistema(codigo_invalido(c))),
    }
}

fn indicador_saida(l: LinhaIndicador) -> Resultado<IndicadorOperacaoSaida> {
    let (codigo_fornecimento, fornecimento) =
        local_do_banco(TipoLocal::Fornecimento, l.local_fornecimento)?;
    let (codigo_incidencia, incidencia) =
        local_do_banco(TipoLocal::Incidencia, l.local_incidencia)?;
    Ok(IndicadorOperacaoSaida {
        c_ind_op: l.codigo,
        tipo_operacao: l.tipo_operacao,
        codigo_fornecimento,
        fornecimento,
        codigo_incidencia,
        incidencia,
        onerosa: l.onerosa.map(|v| v == 1),
        adquirente_exterior: l.adquirente_exterior.map(|v| v == 1),
    })
}

fn situacao_saida(s: SituacaoClassificacao) -> SituacaoClassificacaoSaida {
    SituacaoClassificacaoSaida {
        codigo_situacao: s.codigo_situacao,
        descricao_situacao: s.descricao_situacao,
        codigo_classificacao: s.codigo_classificacao,
        descricao_classificacao: s.descricao_classificacao,
    }
}

/// `formatarCTribNac`: "0101" vira "1.01", "010101" vira "1.01.01"
fn formatar_c_trib_nac(c: Option<&str>) -> Option<String> {
    let c = c.filter(|c| !c.is_empty())?;
    let partes = match c.len() {
        TAM_C_TRIB_NAC_CURTO => vec![c.get(..2)?, c.get(2..4)?],
        TAM_C_TRIB_NAC_LONGO => vec![c.get(..2)?, c.get(2..4)?, c.get(4..6)?],
        _ => return None,
    };
    let primeiro: i32 = partes.first()?.parse().ok()?;
    let mut s = primeiro.to_string();
    for p in partes.iter().skip(1) {
        s.push('.');
        s.push_str(p);
    }
    Some(s)
}

fn somar(parcelas: &[&Option<BigDecimal>]) -> Resultado<BigDecimal> {
    let mut total: Option<BigDecimal> = None;
    for p in parcelas {
        let v = p.as_ref().ok_or_else(npe)?;
        total = Some(match total {
            Some(t) => t.add(v),
            None => v.clone(),
        });
    }
    total.ok_or_else(npe)
}

pub fn calcular_base_calculo(e: &NfseBaseCalculoEntrada) -> Resultado<BaseCalculoNfseSaida> {
    let ano = e.ano_fato_gerador.ok_or_else(npe)?;
    if ano < ANO_INICIO_CBS_IBS {
        return Err(campo_invalido(format!(
            "O ano do fato gerador deve ser {ANO_INICIO_CBS_IBS} ou superior"
        )));
    }
    validar_extintos(
        &[("PIS", &e.pis), ("COFINS", &e.cofins)],
        ano,
        ANO_EXTINCAO_PIS_COFINS,
    )?;
    validar_extintos(&[("ISS", &e.iss)], ano, ANO_EXTINCAO_ISS)?;
    let nao_integra = somar(&[
        &e.desconto_incondicional,
        &e.v_calc_ree_rep_res,
        &e.v_calc_ded_red_ibs_cbs,
        &e.pis,
        &e.cofins,
        &e.iss,
    ])?;
    let base = e
        .valor_servico
        .as_ref()
        .ok_or_else(npe)?
        .subtract(&nao_integra);
    if base.signum() < 0 {
        return Err(campo_invalido(
            "Base de cálculo não pode ser negativa. Valores que não integram a base são maiores que o valor do serviço.",
        ));
    }
    Ok(BaseCalculoNfseSaida {
        base_calculo: Some(base),
    })
}

pub fn locais(tipo: TipoLocal, codigo: Option<i32>) -> Resultado<Vec<CodigoDescricaoNfseSaida>> {
    let saida = |(c, d): (i32, &str)| CodigoDescricaoNfseSaida {
        codigo: Some(c),
        descricao: Some(d.to_string()),
    };
    match codigo {
        None => Ok(tipo.tabela().iter().copied().map(saida).collect()),
        Some(c) => match tipo.descricao(c) {
            Some(d) => Ok(vec![saida((c, d))]),
            None => Err(campo_invalido(codigo_invalido(i64::from(c)))),
        },
    }
}

impl Servicos<'_> {
    pub fn validar_indicador_operacao(
        &self,
        e: &NfseIndicadorOperacaoEntrada,
    ) -> Resultado<ValidacaoIndicadorSaida> {
        let c_trib_nac = formatar_c_trib_nac(e.c_trib_nac.as_deref());
        let dia = e.data_fato_gerador.ok_or_else(npe)?;
        let valido = self.db.combinacao_indicador_existe(CombinacaoIndicador {
            nbs: e.nbs.as_deref(),
            c_class_trib: e.c_class_trib.as_deref(),
            c_ind_op: e.c_ind_op.as_deref(),
            c_trib_nac: c_trib_nac.as_deref(),
            dia,
        })?;
        Ok(ValidacaoIndicadorSaida {
            c_trib_nac: e.c_trib_nac.clone(),
            c_ind_op: e.c_ind_op.clone(),
            c_class_trib: e.c_class_trib.clone(),
            nbs: e.nbs.clone(),
            data: Some(dia),
            valido,
        })
    }

    pub fn local_operacao(&self, c_ind_op: &str, dia: NaiveDate) -> Resultado<LocalOperacaoSaida> {
        let (fornecimento, incidencia) =
            self.db.local_operacao(c_ind_op, dia)?.ok_or_else(|| {
                Falha::new(
                    Classe::IndicadorOperacaoNaoEncontrado,
                    format!(
                        "Indicador de operação '{c_ind_op}' não encontrado para a data {}",
                        data_java(dia)
                    ),
                )
            })?;
        let (codigo_fornecimento, fornecimento) =
            local_do_banco(TipoLocal::Fornecimento, fornecimento)?;
        let (codigo_incidencia, incidencia) = local_do_banco(TipoLocal::Incidencia, incidencia)?;
        Ok(LocalOperacaoSaida {
            c_ind_op: Some(c_ind_op.to_string()),
            data: Some(dia),
            codigo_fornecimento,
            fornecimento,
            codigo_incidencia,
            incidencia,
        })
    }

    /// com NBS sem vínculo, cai na lista geral da data
    pub fn indicadores_operacao(
        &self,
        dia: NaiveDate,
        nbs: Option<&str>,
    ) -> Resultado<Vec<IndicadorOperacaoSaida>> {
        let linhas = match nbs.filter(|n| !n.is_empty()) {
            None => self.db.indicadores_operacao(dia)?,
            Some(n) => {
                if !self.db.existe_codigo(Nomenclatura::Nbs, n, dia)? {
                    return Err(nao_encontrada(Nomenclatura::Nbs, n, dia));
                }
                let com_nbs = self.db.indicadores_operacao_nbs(dia, n)?;
                if com_nbs.is_empty() {
                    self.db.indicadores_operacao(dia)?
                } else {
                    com_nbs
                }
            }
        };
        linhas.into_iter().map(indicador_saida).collect()
    }

    /// sem vínculo para o NBS, devolve a situação da classificação padrão
    pub fn situacoes_classificacoes_nbs(
        &self,
        nbs: &str,
        dia: NaiveDate,
    ) -> Resultado<Vec<SituacaoClassificacaoSaida>> {
        if !self.db.existe_codigo(Nomenclatura::Nbs, nbs, dia)? {
            return Err(nao_encontrada(Nomenclatura::Nbs, nbs, dia));
        }
        let mut linhas = self.db.situacoes_classificacoes_nbs(nbs, dia)?;
        if linhas.is_empty() {
            linhas = self
                .db
                .situacao_da_classificacao(CCLASSTRIB_PADRAO, dia)?
                .into_iter()
                .collect();
        }
        Ok(linhas.into_iter().map(situacao_saida).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entrada(json: &str) -> NfseBaseCalculoEntrada {
        crate::json::ler_documento(json.as_bytes())
            .ok()
            .flatten()
            .unwrap_or_default()
    }

    fn base(json: &str) -> Result<String, Option<String>> {
        calcular_base_calculo(&entrada(json))
            .map(|b| b.base_calculo.map(|d| d.to_string()).unwrap_or_default())
            .map_err(|f| f.mensagem)
    }

    #[test]
    fn base_de_calculo_preserva_escala() {
        let j = r#"{"anoFatoGerador":2026,"valorServico":100.50,"descontoIncondicional":0.5,"vCalcReeRepRes":1,"vCalcDedRedIBSCBS":2,"iss":3.00}"#;
        assert_eq!(base(j), Ok("94.00".into()));
    }

    #[test]
    fn campos_extintos_em_ordem_alfabetica() {
        let j = r#"{"anoFatoGerador":2027,"valorServico":100,"pis":1,"cofins":2}"#;
        assert_eq!(
            base(j),
            Err(Some(
                "Os seguintes campos não podem ser informados a partir de 2027: COFINS, PIS".into()
            ))
        );
    }

    #[test]
    fn nulo_explicito_vira_npe() {
        assert_eq!(
            base(r#"{"anoFatoGerador":2026,"valorServico":100,"pis":null}"#),
            Err(None)
        );
    }

    #[test]
    fn formata_c_trib_nac() {
        assert_eq!(formatar_c_trib_nac(Some("0101")).as_deref(), Some("1.01"));
        assert_eq!(
            formatar_c_trib_nac(Some("140102")).as_deref(),
            Some("14.01.02")
        );
        assert_eq!(formatar_c_trib_nac(Some("12345")), None);
    }

    #[test]
    fn local_invalido_e_campo_invalido() {
        let e = locais(TipoLocal::Incidencia, Some(99))
            .err()
            .and_then(|f| f.mensagem);
        assert_eq!(e.as_deref(), Some("Código inválido: 99"));
        assert_eq!(
            locais(TipoLocal::Incidencia, None).map(|l| l.len()).ok(),
            Some(7)
        );
    }
}
