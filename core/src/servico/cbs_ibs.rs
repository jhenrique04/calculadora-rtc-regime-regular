//! `CalculoCbsIbsService`: CBS, IBS estadual e IBS municipal de um item

use std::collections::HashMap;

use chrono::NaiveDate;

use super::Servicos;
use super::consulta::ID_CBS;
use super::texto::{data_java, exibir, preenchido};
use crate::db::{ClassificacaoCalculo, TratamentoClassificacao, TratamentoTributario};
use crate::decimal::BigDecimal;
use crate::erro::{Classe, Falha, Resultado};
use crate::expressao;
use crate::modelo::entrada::{Item, TipoEnteGov};
use crate::modelo::roc::{Diferimento, ReducaoAliquota, TributacaoCompraGov};
use crate::util::arredondamento::{
    MODO, PRECISAO_INTERNA, arredondar_interno, cem, dividir_por_cem, multiplicar,
};

const CASAS_PERCENTUAL: i64 = 2;
const CLASSIFICACAO_PADRAO_FORCADA: &str = "510001";
const PBIO: i64 = 30;
const PDIF: i64 = 100;
const VARIACAO_EAC: i64 = 5;

fn aliquota_eac() -> BigDecimal {
    BigDecimal::new(1922, 4)
}

fn aliquota_gasolina_tipo_a() -> BigDecimal {
    BigDecimal::new(7925, 4)
}

pub mod var {
    pub const BASE_CALCULO: &str = "baseCalculo";
    pub const IMPOSTO_SELETIVO_CALCULADO: &str = "impostoSeletivoCalculado";
    pub const BASE_CALCULO_INFORMADA: &str = "baseCalculoInformada";
    pub const IMPOSTO_SELETIVO_INFORMADO: &str = "impostoSeletivoInformado";
    pub const ALIQUOTA_EFETIVA: &str = "aliquotaEfetiva";
    pub const REDUTOR: &str = "redutor";
    pub const AJUSTE: &str = "ajuste";
    pub const ALIQUOTA_AD_REM: &str = "aliquotaAdRem";
    pub const ALIQUOTA_AD_VALOREM: &str = "aliquotaAdValorem";
    pub const ALIQUOTA_REFERENCIA: &str = "aliquotaReferencia";
    pub const ALIQUOTA_PADRAO_OU_REFERENCIA: &str = "aliquotaPadraoOuReferencia";
    pub const PERCENTUAL_REDUCAO: &str = "percentualReducao";
    pub const PERCENTUAL_REDUCAO_COMPRA_GOV: &str = "pRedutorCompraGov";
    pub const ALIQUOTA: &str = "aliquota";
    pub const QUANTIDADE: &str = "quantidade";
    pub const TRIBUTO_CALCULADO: &str = "tributoCalculado";
    pub const PERCENTUAL_DIFERIMENTO: &str = "percentualDiferimento";
    pub const ALIQUOTA_AD_REM_PRINCIPAL: &str = "aliquotaAdRemPrincipal";
    pub const ALIQUOTA_AD_REM_SECUNDARIA: &str = "aliquotaAdRemSecundaria";
    pub const VARIACAO_PONTO_PERCENTUAL: &str = "variacaoPontoPercentual";
    pub const PERCENTUAL_BIOCOMBUSTIVEL: &str = "pBio";
}

pub type Variaveis = HashMap<&'static str, BigDecimal>;

/// `Map.ofEntries` com `requireNonNullElse(v, ZERO)`
pub fn variaveis(pares: &[(&'static str, Option<&BigDecimal>)]) -> Variaveis {
    pares
        .iter()
        .map(|(k, v)| (*k, v.cloned().unwrap_or_default()))
        .collect()
}

pub fn avaliar(expr: &str, vars: &Variaveis) -> Resultado<BigDecimal> {
    expressao::avaliar(expr, vars).map_err(|e| Falha::sistema(e.to_string()))
}

pub fn dividir(dividendo: &BigDecimal, divisor: &BigDecimal) -> Resultado<BigDecimal> {
    dividendo
        .divide_scale(divisor, PRECISAO_INTERNA, MODO)
        .map_err(|e| Falha::sistema(e.to_string()))
}

pub fn npe(descricao: &str) -> Falha {
    Falha::sistema(descricao.to_string())
}

fn mover_por_cento(v: &BigDecimal) -> BigDecimal {
    v.move_point_right(CASAS_PERCENTUAL)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tributo {
    Cbs,
    IbsEstadual,
    IbsMunicipal,
}

impl Tributo {
    pub fn codigo(self) -> i64 {
        match self {
            Self::Cbs => ID_CBS,
            Self::IbsEstadual => 3,
            Self::IbsMunicipal => 4,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct EtapaMonofasiaSaida {
    pub quantidade: Option<BigDecimal>,
    pub aliquota_ad_rem: Option<BigDecimal>,
    pub valor: Option<BigDecimal>,
}

#[derive(Debug, Clone, Default)]
pub struct DiferimentoMonofasiaSaida {
    pub valor_diferimento: Option<BigDecimal>,
    pub percentual_diferimento: Option<BigDecimal>,
}

#[derive(Debug, Clone, Default)]
pub struct MonofasiaSaida {
    pub monofasico: Option<EtapaMonofasiaSaida>,
    pub sujeito_retencao: Option<EtapaMonofasiaSaida>,
    pub retido: Option<EtapaMonofasiaSaida>,
    pub diferido: Option<DiferimentoMonofasiaSaida>,
}

#[derive(Debug, Clone, Default)]
pub struct TributacaoRegularSaida {
    pub cst: Option<String>,
    pub c_class_trib: Option<String>,
    pub base_calculo: Option<BigDecimal>,
    pub aliquota_efetiva: Option<BigDecimal>,
    pub tributo_devido: Option<BigDecimal>,
}

#[derive(Debug, Clone, Default)]
pub struct CbsIbsSaida {
    pub aliquota: Option<BigDecimal>,
    pub base_calculo: Option<BigDecimal>,
    pub quantidade: Option<BigDecimal>,
    pub tributo_calculado: Option<BigDecimal>,
    pub tributo_devido: Option<BigDecimal>,
    pub grupo_reducao: Option<ReducaoAliquota>,
    pub tributacao_regular: Option<TributacaoRegularSaida>,
    pub compra_gov: Option<TributacaoCompraGov>,
    pub grupo_diferimento: Option<Diferimento>,
    pub grupo_monofasia: Option<MonofasiaSaida>,
    pub memoria_calculo: Option<String>,
}

pub struct ParametrosCbsIbs<'a> {
    pub tributo: Tributo,
    pub codigo_uf: Option<i64>,
    pub codigo_municipio: Option<i64>,
    pub item: &'a Item,
    pub quantidade: &'a BigDecimal,
    pub tratamento: &'a TratamentoClassificacao,
    pub is_calculado: &'a BigDecimal,
    pub tem_tributacao_regular: bool,
    pub dia: NaiveDate,
    pub tp_ente: Option<TipoEnteGov>,
    pub p_redutor: &'a BigDecimal,
}

/// alíquota de partida conforme o tipo de alíquota da classificação
#[derive(Default)]
struct AliquotaBase {
    valor: Option<BigDecimal>,
    informada_aplicada: bool,
    dividida_por_cem: bool,
    ad_rem_principal: Option<BigDecimal>,
    ad_rem_secundaria: Option<BigDecimal>,
    variacao_ponto_percentual: Option<BigDecimal>,
    p_bio: Option<BigDecimal>,
    p_dif: Option<BigDecimal>,
}

/// resultados intermediários das fórmulas
struct Resultados {
    aliquota: Option<BigDecimal>,
    aliquota_efetiva: Option<BigDecimal>,
    base_calculo: BigDecimal,
    tributo_calculado: BigDecimal,
    tributo_devido: Option<BigDecimal>,
    percentual_diferimento: Option<BigDecimal>,
    valor_diferimento: Option<BigDecimal>,
}

fn expressao_usa_aliquota(expr: Option<&str>) -> bool {
    let Some(e) = expr.filter(|e| preenchido(Some(e))) else {
        return false;
    };
    // `matches(".*\\baliquota\\b.*")`: o `.` não casa terminadores de linha
    if e.chars()
        .any(|c| matches!(c, '\n' | '\r' | '\u{0085}' | '\u{2028}' | '\u{2029}'))
    {
        return false;
    }
    let palavra = |c: char| c.is_ascii_alphanumeric() || c == '_';
    e.match_indices(var::ALIQUOTA).any(|(i, m)| {
        let antes = e[..i].chars().next_back();
        let depois = e[i + m.len()..].chars().next();
        !antes.is_some_and(palavra) && !depois.is_some_and(palavra)
    })
}

fn texto_util(e: &Option<String>) -> Option<&str> {
    e.as_deref().filter(|s| preenchido(Some(s)))
}

impl Servicos<'_> {
    pub fn calcular_cbs_ibs(&self, p: &ParametrosCbsIbs<'_>) -> Resultado<Option<CbsIbsSaida>> {
        let id_tributo = p.tributo.codigo();
        let tratamento = self.tratamento_tributario(p.tratamento.id_tratamento)?;
        let classificacao = self.classificacao_calculo(p.tratamento.id_classificacao)?;
        let situacao = self.situacao_tributaria(p.item.cst.as_deref(), id_tributo, p.dia)?;
        let (cst, c_class_trib) = match (&p.item.tributacao_regular, p.tem_tributacao_regular) {
            (Some(tr), true) => (tr.cst.clone(), tr.c_class_trib.clone()),
            _ => (None, None),
        };
        if tratamento.possui_ajuste {
            return Err(falta_implementacao(&classificacao.codigo));
        }
        let percentual_reducao = if tratamento.possui_percentual_reducao {
            let pr = self.percentual_reducao(classificacao.id, id_tributo, p.dia)?;
            Some(dividir_por_cem(&pr))
        } else {
            None
        };
        let base = self.aliquota_base(
            p,
            &tratamento,
            &classificacao,
            cst.as_deref(),
            c_class_trib.as_deref(),
        )?;
        let r = self.avaliar_formulas(p, &tratamento, &base, percentual_reducao.as_ref())?;
        let Some(situacao) = situacao else {
            return Err(npe(
                "Cannot invoke \"br.gov.serpro.rtc.domain.model.entity.SituacaoTributaria.getInGrupoIbsCbs()\" because \"situacaoTributaria\" is null",
            ));
        };
        if !situacao.grupo_ibs_cbs {
            return Ok(None);
        }
        montar_saida(
            p,
            &classificacao,
            &base,
            r,
            percentual_reducao,
            cst,
            c_class_trib,
            tratamento.possui_percentual_reducao,
        )
        .map(Some)
    }

    fn aliquota_base(
        &self,
        p: &ParametrosCbsIbs<'_>,
        tratamento: &TratamentoTributario,
        classificacao: &ClassificacaoCalculo,
        cst: Option<&str>,
        c_class_trib: Option<&str>,
    ) -> Resultado<AliquotaBase> {
        let id_tributo = p.tributo.codigo();
        let tipo = if classificacao.codigo == CLASSIFICACAO_PADRAO_FORCADA {
            "Padrão"
        } else {
            classificacao.tipo_aliquota.as_str()
        };
        let informada = aliquota_nominal(p.tributo, p.item);
        let com_informada =
            |buscar: &dyn Fn() -> Resultado<BigDecimal>| -> Resultado<AliquotaBase> {
                Ok(match informada {
                    Some(a) => AliquotaBase {
                        valor: Some(dividir_por_cem(a)),
                        informada_aplicada: true,
                        dividida_por_cem: true,
                        ..AliquotaBase::default()
                    },
                    None => AliquotaBase {
                        valor: Some(buscar()?),
                        dividida_por_cem: true,
                        ..AliquotaBase::default()
                    },
                })
            };
        match tipo {
            "Padrão" => com_informada(&|| {
                let a = self.aliquota_padrao(id_tributo, p.codigo_uf, p.codigo_municipio, p.dia)?;
                Ok(dividir_por_cem(a.valor_aplicavel()))
            }),
            "Uniforme nacional (referência)" => com_informada(&|| {
                Ok(dividir_por_cem(
                    &self.aliquota_referencia(id_tributo, p.dia)?,
                ))
            }),
            "Uniforme setorial" if tratamento.possui_monofasia => Ok(AliquotaBase {
                ad_rem_principal: Some(aliquota_gasolina_tipo_a()),
                ad_rem_secundaria: Some(aliquota_eac()),
                p_bio: Some(dividir_por_cem(&BigDecimal::from_i64(PBIO))),
                p_dif: Some(dividir_por_cem(&BigDecimal::from_i64(PDIF))),
                variacao_ponto_percentual: Some(BigDecimal::from_i64(VARIACAO_EAC)),
                ..AliquotaBase::default()
            }),
            "Uniforme setorial" => com_informada(&|| {
                self.aliquota_uniforme_setorial(
                    p.item.nbs.as_deref(),
                    id_tributo,
                    classificacao.id,
                    p.dia,
                )
            }),
            "Fixa" => com_informada(&|| self.aliquota_fixa(id_tributo, classificacao.id, p.dia)),
            "Sem alíquota" => Ok(match informada {
                Some(a) => AliquotaBase {
                    valor: Some(dividir_por_cem(a)),
                    informada_aplicada: true,
                    dividida_por_cem: true,
                    ..AliquotaBase::default()
                },
                None => AliquotaBase {
                    valor: Some(BigDecimal::zero()),
                    ..AliquotaBase::default()
                },
            }),
            "Alíquotas Combinadas (Ad Valorem e Ad Rem)" => Ok(AliquotaBase::default()),
            outro => Err(Falha::new(
                Classe::TipoAliquotaDesconhecido,
                format!(
                    "Tipo de alíquota ({outro}) desconhecido para cClassTrib {} e CST {}",
                    exibir(c_class_trib),
                    exibir(cst)
                ),
            )),
        }
    }

    fn aliquota_uniforme_setorial(
        &self,
        nbs: Option<&str>,
        id_tributo: i64,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<BigDecimal> {
        let aliquota =
            match self.ad_valorem_servico(nbs, id_tributo, Some(id_classificacao), dia)? {
                Some(a) => a,
                None => self
                    .ad_valorem_por_classificacao(id_tributo, id_classificacao, dia)?
                    .ok_or_else(|| {
                        Falha::new(
                            Classe::ErroGenericoValidacao,
                            format!(
                                "Alíquota uniforme setorial não encontrada em {}",
                                data_java(dia)
                            ),
                        )
                    })?,
            };
        Ok(dividir_por_cem(&aliquota))
    }

    fn aliquota_fixa(
        &self,
        id_tributo: i64,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<BigDecimal> {
        let aliquota = self
            .ad_valorem_por_classificacao(id_tributo, id_classificacao, dia)?
            .ok_or_else(|| {
                Falha::new(
                    Classe::ErroGenericoValidacao,
                    format!("Alíquota fixa não encontrada em {}", data_java(dia)),
                )
            })?;
        Ok(dividir_por_cem(&aliquota))
    }

    fn avaliar_formulas(
        &self,
        p: &ParametrosCbsIbs<'_>,
        t: &TratamentoTributario,
        b: &AliquotaBase,
        percentual_reducao: Option<&BigDecimal>,
    ) -> Resultado<Resultados> {
        use var::*;
        let is_informado = p
            .item
            .imposto_seletivo
            .as_ref()
            .map(|is| is.valor_informado());
        let bc_informada = p.item.base_calculo.as_ref();
        let q = Some(p.quantidade);
        // variáveis sempre nulas no Java (viram zero no mapa)
        let comuns = [
            (PERCENTUAL_REDUCAO, percentual_reducao),
            (ALIQUOTA_PADRAO_OU_REFERENCIA, None),
            (ALIQUOTA_REFERENCIA, None),
            (ALIQUOTA_AD_VALOREM, None),
            (ALIQUOTA_AD_REM, None),
            (AJUSTE, None),
            (REDUTOR, None),
        ];
        let monofasia = [
            (ALIQUOTA_AD_REM_PRINCIPAL, b.ad_rem_principal.as_ref()),
            (ALIQUOTA_AD_REM_SECUNDARIA, b.ad_rem_secundaria.as_ref()),
            (
                VARIACAO_PONTO_PERCENTUAL,
                b.variacao_ponto_percentual.as_ref(),
            ),
            (PERCENTUAL_BIOCOMBUSTIVEL, b.p_bio.as_ref()),
            (PERCENTUAL_DIFERIMENTO, b.p_dif.as_ref()),
        ];
        let montar = |proprias: &[(&'static str, Option<&BigDecimal>)], com_mono: bool| {
            let mut pares: Vec<(&'static str, Option<&BigDecimal>)> = proprias.to_vec();
            pares.extend_from_slice(&comuns);
            if com_mono {
                pares.extend_from_slice(&monofasia);
            }
            variaveis(&pares)
        };

        let expr_aliquota = texto_util(&t.expressao_aliquota);
        let aliquota =
            if b.informada_aplicada && !expressao_usa_aliquota(t.expressao_aliquota.as_deref()) {
                b.valor.clone()
            } else if let Some(e) = expr_aliquota {
                Some(avaliar(
                    e,
                    &montar(&[(ALIQUOTA, b.valor.as_ref()), (QUANTIDADE, q)], true),
                )?)
            } else {
                None
            };

        let aliquota_efetiva = match texto_util(&t.expressao_aliquota_efetiva) {
            Some(e) => {
                let v = montar(
                    &[
                        (QUANTIDADE, q),
                        (ALIQUOTA, aliquota.as_ref()),
                        (PERCENTUAL_REDUCAO_COMPRA_GOV, Some(p.p_redutor)),
                    ],
                    true,
                );
                Some(avaliar(e, &v)?)
            }
            None => None,
        };

        let mut base_calculo = avaliar(
            &t.expressao_base_calculo,
            &montar(
                &[
                    (QUANTIDADE, q),
                    (ALIQUOTA, aliquota.as_ref()),
                    (ALIQUOTA_EFETIVA, aliquota_efetiva.as_ref()),
                    (IMPOSTO_SELETIVO_INFORMADO, is_informado.as_ref()),
                    (BASE_CALCULO_INFORMADA, bc_informada),
                    (IMPOSTO_SELETIVO_CALCULADO, Some(p.is_calculado)),
                ],
                true,
            ),
        )?;
        if base_calculo.signum() < 0 {
            base_calculo = BigDecimal::zero();
        }

        let com_base = |extra: Option<(&'static str, &BigDecimal)>, com_mono: bool| {
            let mut pares: Vec<(&'static str, Option<&BigDecimal>)> = Vec::new();
            if let Some((k, v)) = extra {
                pares.push((k, Some(v)));
            }
            pares.extend_from_slice(&[
                (QUANTIDADE, q),
                (BASE_CALCULO, Some(&base_calculo)),
                (ALIQUOTA, aliquota.as_ref()),
                (ALIQUOTA_EFETIVA, aliquota_efetiva.as_ref()),
                (BASE_CALCULO_INFORMADA, bc_informada),
                (IMPOSTO_SELETIVO_INFORMADO, is_informado.as_ref()),
                (IMPOSTO_SELETIVO_CALCULADO, Some(p.is_calculado)),
            ]);
            montar(&pares, com_mono)
        };

        let mut tributo_calculado = avaliar(&t.expressao_tributo_calculado, &com_base(None, true))?;
        let mut tributo_devido = match texto_util(&t.expressao_tributo_devido) {
            Some(e) => Some(avaliar(
                e,
                &com_base(Some((TRIBUTO_CALCULADO, &tributo_calculado)), true),
            )?),
            None => None,
        };

        let mut percentual_diferimento = None;
        let mut valor_diferimento = None;
        if let (Some(e_pdif), Some(e_vdif)) = (
            texto_util(&t.expressao_percentual_diferimento),
            texto_util(&t.expressao_valor_diferimento),
        ) {
            let pdif = avaliar(
                e_pdif,
                &com_base(Some((TRIBUTO_CALCULADO, &tributo_calculado)), false),
            )?;
            let mut v6 = com_base(Some((TRIBUTO_CALCULADO, &tributo_calculado)), false);
            v6.insert(PERCENTUAL_DIFERIMENTO, pdif.clone());
            let vdif = avaliar(e_vdif, &v6)?;
            tributo_calculado = arredondar_interno(&tributo_calculado.subtract(&vdif));
            // o Java subtrai o diferimento duas vezes do devido (usa o calculado já reduzido)
            let mut devido = arredondar_interno(&tributo_calculado.subtract(&vdif));
            if tributo_calculado.signum() < 0 {
                tributo_calculado = BigDecimal::zero();
            }
            if devido.signum() < 0 {
                devido = BigDecimal::zero();
            }
            tributo_devido = Some(devido);
            percentual_diferimento = Some(pdif);
            valor_diferimento = Some(vdif);
        }

        Ok(Resultados {
            aliquota,
            aliquota_efetiva,
            base_calculo,
            tributo_calculado,
            tributo_devido,
            percentual_diferimento,
            valor_diferimento,
        })
    }
}

fn falta_implementacao(codigo: &str) -> Falha {
    Falha::new(
        Classe::ErroFaltaImplementacao,
        format!(
            "A classificação tributária {codigo} ainda não possui tratamento tributário implementado na Calculadora. Essa funcionalidade está prevista no roadmap e será disponibilizada em versão futura."
        ),
    )
}

fn aliquota_nominal(tributo: Tributo, item: &Item) -> Option<&BigDecimal> {
    let a = item.aliquotas_nominais.as_ref()?;
    match tributo {
        Tributo::Cbs => a.cbs.as_ref(),
        Tributo::IbsEstadual => a.ibs_estadual.as_ref(),
        Tributo::IbsMunicipal => a.ibs_municipal.as_ref(),
    }
}

#[allow(clippy::too_many_arguments)]
fn montar_saida(
    p: &ParametrosCbsIbs<'_>,
    classificacao: &ClassificacaoCalculo,
    b: &AliquotaBase,
    r: Resultados,
    percentual_reducao: Option<BigDecimal>,
    cst: Option<String>,
    c_class_trib: Option<String>,
    possui_percentual_reducao: bool,
) -> Resultado<CbsIbsSaida> {
    let sem_reducao =
        !classificacao.grupo_reducao || (p.tem_tributacao_regular && p.tp_ente.is_none());
    let grupo_reducao = if sem_reducao {
        None
    } else {
        grupo_reducao(
            classificacao,
            r.aliquota_efetiva.as_ref(),
            percentual_reducao.as_ref(),
            b.dividida_por_cem,
        )
    };
    let compra_gov = grupo_compra_gov(
        p.tp_ente,
        p.tributo,
        r.aliquota_efetiva.as_ref(),
        r.tributo_devido.as_ref(),
    )?;
    let tributacao_regular = if p.tem_tributacao_regular {
        Some(grupo_desoneracao(
            cst,
            c_class_trib,
            &r.base_calculo,
            r.aliquota.as_ref(),
            &percentual_reducao.unwrap_or_default(),
            b.dividida_por_cem,
            possui_percentual_reducao,
        )?)
    } else {
        None
    };
    let grupo_diferimento = match (&r.valor_diferimento, &r.percentual_diferimento) {
        (Some(v), Some(pd)) => Some(Diferimento {
            p_dif: Some(mover_por_cento(pd)),
            v_dif: Some(v.clone()),
        }),
        _ => None,
    };
    let grupo_monofasia = grupo_monofasia(
        classificacao,
        p.quantidade,
        r.aliquota.as_ref(),
        r.aliquota_efetiva.as_ref(),
        &r.tributo_calculado,
        r.tributo_devido.as_ref(),
        b.variacao_ponto_percentual.as_ref(),
    )?;
    let zero_se_regular = |v: Option<BigDecimal>| {
        if p.tem_tributacao_regular {
            Some(BigDecimal::zero())
        } else {
            v
        }
    };
    let aliquota = match (&r.aliquota, b.dividida_por_cem) {
        (Some(a), true) => Some(mover_por_cento(a)),
        (a, _) => a.clone(),
    };
    Ok(CbsIbsSaida {
        aliquota: zero_se_regular(aliquota),
        base_calculo: Some(r.base_calculo),
        quantidade: Some(p.quantidade.clone()),
        tributo_calculado: zero_se_regular(Some(r.tributo_calculado)),
        tributo_devido: zero_se_regular(r.tributo_devido),
        grupo_reducao,
        tributacao_regular,
        compra_gov,
        grupo_diferimento,
        grupo_monofasia: Some(grupo_monofasia),
        memoria_calculo: None,
    })
}

fn grupo_reducao(
    classificacao: &ClassificacaoCalculo,
    aliquota_efetiva: Option<&BigDecimal>,
    percentual_reducao: Option<&BigDecimal>,
    dividida_por_cem: bool,
) -> Option<ReducaoAliquota> {
    let (Some(pr), Some(ef)) = (percentual_reducao, aliquota_efetiva) else {
        return None;
    };
    if !classificacao.grupo_reducao {
        return None;
    }
    Some(ReducaoAliquota {
        p_red_aliq: Some(mover_por_cento(pr)),
        p_aliq_efet: Some(if dividida_por_cem {
            mover_por_cento(ef)
        } else {
            ef.clone()
        }),
    })
}

fn grupo_compra_gov(
    tp_ente: Option<TipoEnteGov>,
    tributo: Tributo,
    aliquota: Option<&BigDecimal>,
    valor: Option<&BigDecimal>,
) -> Resultado<Option<TributacaoCompraGov>> {
    if tp_ente.is_none() {
        return Ok(None);
    }
    let aliquota = aliquota
        .map(mover_por_cento)
        .ok_or_else(|| npe("Cannot invoke \"java.math.BigDecimal.movePointRight(int)\" because \"aliquota\" is null"))?;
    let valor = valor.cloned();
    // o builder começa com zeros (@Builder.Default) e só preenche o tributo calculado
    let mut g = TributacaoCompraGov::default();
    match tributo {
        Tributo::Cbs => {
            g.p_aliq_cbs = Some(aliquota);
            g.v_trib_cbs = valor;
        }
        Tributo::IbsEstadual => {
            g.p_aliq_ibs_uf = Some(aliquota);
            g.v_trib_ibs_uf = valor;
        }
        Tributo::IbsMunicipal => {
            g.p_aliq_ibs_mun = Some(aliquota);
            g.v_trib_ibs_mun = valor;
        }
    }
    Ok(Some(g))
}

fn grupo_desoneracao(
    cst: Option<String>,
    c_class_trib: Option<String>,
    base_calculo: &BigDecimal,
    aliquota: Option<&BigDecimal>,
    percentual_reducao: &BigDecimal,
    dividida_por_cem: bool,
    possui_percentual_reducao: bool,
) -> Resultado<TributacaoRegularSaida> {
    let aliquota_efetiva = if possui_percentual_reducao {
        aliquota.map(|a| multiplicar(a, &BigDecimal::one().subtract(percentual_reducao)))
    } else {
        aliquota.cloned()
    };
    let tributo_devido = aliquota_efetiva
        .as_ref()
        .map(|a| multiplicar(base_calculo, a));
    let aliquota_efetiva = match (aliquota_efetiva, dividida_por_cem) {
        (Some(a), true) => Some(mover_por_cento(&a)),
        (None, true) => {
            return Err(npe(
                "Cannot invoke \"java.math.BigDecimal.movePointRight(int)\" because \"aliquotaEfetiva\" is null",
            ));
        }
        (a, false) => a,
    };
    Ok(TributacaoRegularSaida {
        cst,
        c_class_trib,
        base_calculo: Some(base_calculo.clone()),
        aliquota_efetiva,
        tributo_devido,
    })
}

fn fator_biocombustivel() -> Resultado<BigDecimal> {
    let pbio = BigDecimal::from_i64(PBIO);
    dividir(&pbio, &cem().subtract(&pbio))
}

fn exigir<'a>(v: Option<&'a BigDecimal>, descricao: &str) -> Resultado<&'a BigDecimal> {
    v.ok_or_else(|| npe(descricao))
}

fn grupo_monofasia(
    c: &ClassificacaoCalculo,
    quantidade: &BigDecimal,
    ad_rem_principal: Option<&BigDecimal>,
    ad_rem_secundaria: Option<&BigDecimal>,
    tributo_calculado: &BigDecimal,
    tributo_devido: Option<&BigDecimal>,
    variacao_ponto_percentual: Option<&BigDecimal>,
) -> Resultado<MonofasiaSaida> {
    let mut g = MonofasiaSaida::default();
    if c.mono_padrao {
        let (qtd, ad_rem) = if c.mono_ret {
            let variacao = exigir(
                variacao_ponto_percentual,
                "Cannot read field \"scale\" because \"multiplicand\" is null",
            )?;
            (
                dividir(&quantidade.multiply(variacao), &cem())?,
                ad_rem_secundaria,
            )
        } else {
            (quantidade.clone(), ad_rem_principal)
        };
        g.monofasico = Some(EtapaMonofasiaSaida {
            quantidade: Some(qtd),
            aliquota_ad_rem: ad_rem.cloned(),
            valor: Some(tributo_calculado.clone()),
        });
    }
    if c.mono_reten {
        g.sujeito_retencao = Some(EtapaMonofasiaSaida {
            quantidade: Some(multiplicar(quantidade, &fator_biocombustivel()?)),
            aliquota_ad_rem: ad_rem_secundaria.cloned(),
            valor: tributo_devido.cloned(),
        });
    }
    if c.mono_ret {
        g.retido = Some(EtapaMonofasiaSaida {
            quantidade: Some(quantidade.clone()),
            aliquota_ad_rem: ad_rem_principal.cloned(),
            valor: tributo_devido.cloned(),
        });
    }
    if c.mono_diferimento {
        let devido = exigir(
            tributo_devido,
            "Cannot invoke \"java.math.BigDecimal.negate()\" because \"subtrahend\" is null",
        )?;
        let diferido = tributo_calculado.subtract(devido);
        g.diferido = Some(DiferimentoMonofasiaSaida {
            percentual_diferimento: Some(mover_por_cento(&dividir(&diferido, tributo_calculado)?)),
            valor_diferimento: Some(diferido),
        });
    }
    Ok(g)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detecta_variavel_aliquota_como_palavra() {
        assert!(expressao_usa_aliquota(Some(
            "aliquota * (1 - percentualReducao)"
        )));
        assert!(!expressao_usa_aliquota(Some("aliquotaEfetiva * 2")));
        assert!(!expressao_usa_aliquota(Some("aliquota\n")));
        assert!(!expressao_usa_aliquota(Some("  ")));
        assert!(!expressao_usa_aliquota(None));
    }
}
