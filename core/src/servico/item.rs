//! `ProcessamentoItemService`: validações de negócio do item e disparo do cálculo

use chrono::{Datelike, NaiveDate};

use super::Servicos;
use super::base_calculo::{self, Natureza};
use super::consulta::{ID_CBS, ID_IS, TRIBUTO_IS, TRIBUTOS_CBS_IBS};
use super::imposto_seletivo::AliquotaImpostoSeletivo;
use super::nomenclatura::{
    Alvo, Modo, SiglaDfe, TAM_NBS_COMPLETO, TAM_NCM_COMPLETO, nao_encontrada,
};
use super::texto::{branco, data_java, preenchido, tamanho, vazio_apos_trim};
use super::tributo::{OperacaoCalculo, Tratamentos};
use crate::db::{ClassificacaoTributaria, Nomenclatura, TipoDfe};
use crate::decimal::BigDecimal;
use crate::erro::{Classe, Falha, Resultado};
use crate::modelo::entrada::{AliquotasNominais, AliquotasNominaisIs, Item, Operacao};
use crate::modelo::roc::Tributos;
use crate::util::arredondamento::arredondar_interno;

const ANO_ALIQUOTAS_NOMINAIS: i32 = 2027;
const UNIDADE_PADRAO: &str = "UN";

fn generico(msg: &str) -> Falha {
    Falha::new(Classe::ErroGenericoValidacao, msg)
}

/// item com os defaults e as bases de cálculo que o Java grava no próprio DTO
fn preparar(item: &Item) -> Item {
    let mut item = item.clone();
    item.quantidade.get_or_insert_with(BigDecimal::one);
    item.unidade
        .get_or_insert_with(|| UNIDADE_PADRAO.to_string());
    item
}

fn nomenclatura_is(ncm: Option<&str>, nbs: Option<&str>) -> (&'static str, String) {
    if preenchido(ncm) {
        ("NCM", ncm.unwrap_or_default().to_string())
    } else {
        (
            "NBS",
            nbs.map_or_else(|| "null".to_string(), str::to_string),
        )
    }
}

fn natureza(item: &Item) -> Option<Natureza> {
    match (
        preenchido(item.ncm.as_deref()),
        preenchido(item.nbs.as_deref()),
    ) {
        (true, false) => Some(Natureza::Bem),
        (false, true) => Some(Natureza::Servico),
        _ => None,
    }
}

fn base_negativa(base: &BigDecimal, msg: &str) -> Resultado<()> {
    if base.signum() < 0 {
        return Err(generico(msg));
    }
    Ok(())
}

fn conferir_declarada(declarada: Option<&BigDecimal>, calculada: &BigDecimal) -> Resultado<()> {
    match declarada {
        Some(d) if d.compare_to(&arredondar_interno(calculada)).is_ne() => Err(Falha::new(
            Classe::BaseCalculoComponentesInconsistente,
            format!(
                "A base de cálculo declarada ({d}) diverge da base calculada a partir dos componentes informados ({calculada})"
            ),
        )),
        _ => Ok(()),
    }
}

fn validar_base_com_componentes(op: &Operacao, item: &mut Item) -> Resultado<()> {
    let Some(componentes) = &item.componentes_bc else {
        return Ok(());
    };
    let ano = op.fato_gerador()?.year();
    let bases = base_calculo::calcular(componentes, ano, natureza(item), op.tp_doc)?;
    if let Some(is) = item.imposto_seletivo.as_mut() {
        base_negativa(
            &bases.base_is,
            "A base de cálculo do Imposto Seletivo calculada a partir dos componentes é negativa",
        )?;
        conferir_declarada(is.base_calculo.as_ref(), &bases.base_is)?;
        is.base_calculo.get_or_insert(bases.base_is);
        return Ok(());
    }
    base_negativa(
        &bases.base_rg,
        "A base de cálculo calculada a partir dos componentes é negativa",
    )?;
    conferir_declarada(item.base_calculo.as_ref(), &bases.base_rg)?;
    item.base_calculo.get_or_insert(bases.base_rg);
    Ok(())
}

fn validar_base_legado(item: &mut Item) -> Resultado<()> {
    let Some(is) = &item.imposto_seletivo else {
        if item.base_calculo.is_none() {
            return Err(Falha::new(
                Classe::BaseCalculoNaoInformada,
                "A base de cálculo de CBS e IBS deve ser informada quando o grupo do Imposto Seletivo não é informado",
            ));
        }
        return Ok(());
    };
    let Some(bc_is) = &is.base_calculo else {
        return Err(generico(
            "A base de cálculo do Imposto Seletivo deve ser informada quando o grupo de Imposto Seletivo é informado",
        ));
    };
    let Some(base) = &item.base_calculo else {
        item.base_calculo = Some(arredondar_interno(&bc_is.add(&is.valor_informado())));
        return Ok(());
    };
    match &is.imposto_informado {
        Some(informado) if base.compare_to(&bc_is.add(informado)).is_ne() => Err(Falha::new(
            Classe::BaseCalculoInconsistente,
            format!(
                "A base de cálculo de CBS e IBS informada {base} difere da soma da base de cálculo do Imposto Seletivo {bc_is} com o Imposto Seletivo informado {informado}"
            ),
        )),
        None if base.compare_to(bc_is).is_lt() => Err(Falha::new(
            Classe::BaseCalculoMenorBaseCalculoImpostoSeletivo,
            format!(
                "A base de cálculo de CBS e IBS informada {base} é menor que a base de cálculo do Imposto Seletivo {bc_is}"
            ),
        )),
        _ => Ok(()),
    }
}

fn informou_aliquotas_nominais(a: Option<&AliquotasNominais>) -> bool {
    let Some(a) = a else { return false };
    a.cbs.is_some()
        || a.ibs_estadual.is_some()
        || a.ibs_municipal.is_some()
        || a.imposto_seletivo
            .as_ref()
            .is_some_and(|is| is.ad_valorem.is_some() || is.ad_rem.is_some())
}

fn validar_cst_classificacao(
    c: &ClassificacaoTributaria,
    cst: Option<&str>,
    tributos: &str,
) -> Resultado<()> {
    if Some(c.cst.as_str()) == cst {
        return Ok(());
    }
    Err(Falha::new(
        Classe::ClassificacaoTributariaNaoVinculadaSituacaoTributaria,
        format!(
            "Classificação tributária (cClassTrib) de código {} não está vinculada à situação tributária (CST) de código {} ({tributos})",
            c.codigo,
            cst.unwrap_or("null")
        ),
    ))
}

fn alvo<'a>(c: &'a ClassificacaoTributaria) -> Alvo<'a> {
    Alvo {
        id: c.id,
        codigo: &c.codigo,
        tributos: TRIBUTOS_CBS_IBS,
    }
}

impl Servicos<'_> {
    pub fn processar_item(
        &self,
        op: &Operacao,
        uf: Option<&str>,
        item: &Item,
        dia: NaiveDate,
        tipo_dfe: Option<&TipoDfe>,
    ) -> Resultado<Tributos> {
        let mut item = preparar(item);
        if item
            .componentes_bc
            .as_ref()
            .is_some_and(|c| c.possui_componentes())
        {
            validar_base_com_componentes(op, &mut item)?;
        } else {
            validar_base_legado(&mut item)?;
        }
        self.validar_ncm_nbs(item.ncm.as_deref(), item.nbs.as_deref(), dia)?;
        self.validar_cst(item.cst.as_deref(), ID_CBS, dia)?;
        self.validar_aliquotas_nominais(&item, dia)?;
        let tratamentos = self.tratamentos(&item, dia, tipo_dfe)?;
        let codigo_uf = self.codigo_uf(uf)?;
        let tp_ente = op.tp_ente_gov();
        let p_redutor = self.redutor_compra_gov(tp_ente, dia)?;
        let quantidade = item.quantidade.clone().unwrap_or_else(BigDecimal::one);
        self.calcular_tributos(&OperacaoCalculo {
            dia,
            codigo_municipio: op.municipio,
            codigo_uf,
            tp_ente,
            p_redutor,
            item: &item,
            quantidade: &quantidade,
            tratamentos,
        })
    }

    fn validar_ncm_nbs(
        &self,
        ncm: Option<&str>,
        nbs: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<()> {
        let tem_ncm = !vazio_apos_trim(ncm);
        let tem_nbs = !vazio_apos_trim(nbs);
        if tem_ncm && tem_nbs {
            return Err(Falha::new(
                Classe::NcmNbsSimultaneas,
                "Não é permitido informar NCM e NBS simultaneamente para um item",
            ));
        }
        if let Some(c) = ncm.filter(|_| tem_ncm)
            && !self.db.existe_codigo(Nomenclatura::Ncm, c, dia)?
        {
            return Err(nao_encontrada(Nomenclatura::Ncm, c, dia));
        }
        if let Some(c) = nbs.filter(|_| tem_nbs)
            && !self.db.existe_codigo(Nomenclatura::Nbs, c, dia)?
        {
            return Err(nao_encontrada(Nomenclatura::Nbs, c, dia));
        }
        Ok(())
    }

    fn validar_aliquotas_nominais(&self, item: &Item, dia: NaiveDate) -> Resultado<()> {
        if dia.year() < ANO_ALIQUOTAS_NOMINAIS {
            if informou_aliquotas_nominais(item.aliquotas_nominais.as_ref()) {
                return Err(Falha::new(
                    Classe::AliquotasNominaisInformadasIndevidamente,
                    "As alíquotas nominais não podem ser informadas para operações com fato gerador anterior a 01/01/2027, pois a calculadora resolve as alíquotas vigentes",
                ));
            }
            return Ok(());
        }
        let exige = self
            .situacao_tributaria(item.cst.as_deref(), ID_CBS, dia)?
            .is_some_and(|s| s.grupo_ibs_cbs);
        if !exige {
            return Ok(());
        }
        let completas = item.aliquotas_nominais.as_ref().is_some_and(|a| {
            a.cbs.is_some() && a.ibs_estadual.is_some() && a.ibs_municipal.is_some()
        });
        if completas {
            return Ok(());
        }
        Err(Falha::new(
            Classe::AliquotasNominaisNaoInformadas,
            "As alíquotas nominais (os três campos cbs, ibsEstadual e ibsMunicipal) devem ser informadas para operações com fato gerador a partir de 01/01/2027",
        ))
    }

    fn validar_vinculo(
        &self,
        tipo_dfe: Option<&TipoDfe>,
        c: &ClassificacaoTributaria,
        dia: NaiveDate,
    ) -> Resultado<()> {
        match tipo_dfe {
            Some(t) => self.validar_vinculo_tipo_dfe(t, alvo(c), dia),
            None => Ok(()),
        }
    }

    /// classificação CBS/IBS com as checagens de CST e de tipo de DFe
    fn classificacao_validada(
        &self,
        codigo: Option<&str>,
        cst: Option<&str>,
        item: &Item,
        dia: NaiveDate,
        tipo_dfe: Option<&TipoDfe>,
    ) -> Resultado<ClassificacaoTributaria> {
        let c = self.classificacao_cbs_ibs(codigo, dia)?;
        if !(vazio_apos_trim(item.ncm.as_deref()) && vazio_apos_trim(item.nbs.as_deref())) {
            validar_cst_classificacao(&c, cst, TRIBUTOS_CBS_IBS)?;
        }
        self.validar_vinculo(tipo_dfe, &c, dia)?;
        Ok(c)
    }

    fn tratamentos(
        &self,
        item: &Item,
        dia: NaiveDate,
        tipo_dfe: Option<&TipoDfe>,
    ) -> Resultado<Tratamentos> {
        let ncm = item.ncm.as_deref();
        let nbs = item.nbs.as_deref();
        let mut classificacao = self.classificacao_validada(
            item.c_class_trib.as_deref(),
            item.cst.as_deref(),
            item,
            dia,
            tipo_dfe,
        )?;
        let mut cbs_ibs = self.tratamento_classificacao(classificacao.id, dia)?;
        let mut desoneracao = None;
        let mut tem_desoneracao = false;
        if cbs_ibs.exige_grupo_desoneracao {
            let Some(tr) = &item.tributacao_regular else {
                return Err(Falha::new(
                    Classe::TributacaoRegularNaoInformada,
                    format!(
                        "Dados da tributação regular devem ser informados para cClassTrib {} e CST {}",
                        item.c_class_trib.as_deref().unwrap_or("null"),
                        item.cst.as_deref().unwrap_or("null")
                    ),
                ));
            };
            classificacao = self.classificacao_validada(
                tr.c_class_trib.as_deref(),
                tr.cst.as_deref(),
                item,
                dia,
                tipo_dfe,
            )?;
            let regular = self.tratamento_classificacao(classificacao.id, dia)?;
            if regular.incompativel_com_suspensao {
                return Err(Falha::new(
                    Classe::IncompatibilidadeSuspensao,
                    format!(
                        "Classificação tributária (cClassTrib) {} e situação tributária (CST) {} incompatíveis com suspensão",
                        tr.c_class_trib.as_deref().unwrap_or("null"),
                        tr.cst.as_deref().unwrap_or("null")
                    ),
                ));
            }
            desoneracao = Some(std::mem::replace(&mut cbs_ibs, regular));
            tem_desoneracao = true;
        } else if let (Some(tr), Some(t)) = (&item.tributacao_regular, tipo_dfe) {
            let c = self.classificacao_cbs_ibs(tr.c_class_trib.as_deref(), dia)?;
            self.validar_vinculo(Some(t), &c, dia)?;
        }
        self.validar_nomenclatura_cbs_ibs(&classificacao, ncm, nbs, dia, tipo_dfe)?;
        let aliquota_is = self.analisar_aliquota_is(item, dia)?;
        let imposto_seletivo =
            self.validar_imposto_seletivo(item, dia, tipo_dfe, aliquota_is.as_ref())?;
        Ok(Tratamentos {
            cbs_ibs,
            imposto_seletivo,
            desoneracao,
            aliquota_is,
            tem_desoneracao,
        })
    }

    fn validar_nomenclatura_cbs_ibs(
        &self,
        c: &ClassificacaoTributaria,
        ncm: Option<&str>,
        nbs: Option<&str>,
        dia: NaiveDate,
        tipo_dfe: Option<&TipoDfe>,
    ) -> Resultado<()> {
        if let Some(t) = tipo_dfe {
            let sigla = SiglaDfe::por_sigla(&t.sigla)?;
            return self.validar_nomenclatura(sigla, alvo(c), ncm, nbs, dia, Modo::Permissivo);
        }
        let sem_ncm = vazio_apos_trim(ncm);
        let sem_nbs = vazio_apos_trim(nbs);
        if sem_ncm && sem_nbs {
            self.validar_aplicavel(Nomenclatura::Ncm, ncm, alvo(c), dia)?;
            self.validar_aplicavel(Nomenclatura::Nbs, nbs, alvo(c), dia)?;
        }
        if !sem_ncm && sem_nbs {
            self.validar_aplicavel(Nomenclatura::Ncm, ncm, alvo(c), dia)?;
        }
        if !sem_nbs && sem_ncm {
            self.validar_aplicavel(Nomenclatura::Nbs, nbs, alvo(c), dia)?;
        }
        Ok(())
    }

    fn validar_imposto_seletivo(
        &self,
        item: &Item,
        dia: NaiveDate,
        tipo_dfe: Option<&TipoDfe>,
        aliquota_is: Option<&AliquotaImpostoSeletivo>,
    ) -> Resultado<Option<crate::db::TratamentoClassificacao>> {
        let ncm = item.ncm.as_deref();
        let nbs = item.nbs.as_deref();
        let (nome, codigo) = nomenclatura_is(ncm, nbs);
        if let (Some(t), Some(_)) = (tipo_dfe, aliquota_is)
            && !SiglaDfe::is_aplicavel_por_sigla(&t.sigla)
        {
            let tp = t.tipo.map_or_else(|| "null".to_string(), |v| v.to_string());
            return Err(Falha::new(
                Classe::ImpostoSeletivoNaoAdmitidoTipoDfe,
                format!(
                    "O tipo de documento fiscal (tpDoc) {tp} não admite Imposto Seletivo, exigido para {nome} de código {codigo}"
                ),
            ));
        }
        let nao_informado = || {
            Falha::new(
                Classe::ImpostoSeletivoNaoInformado,
                format!(
                    "Dados do Imposto Seletivo devem ser informados para {nome} {codigo} e data {}",
                    data_java(dia)
                ),
            )
        };
        let Some(aliquota) = aliquota_is else {
            if dia.year() >= ANO_ALIQUOTAS_NOMINAIS && self.existe_incidencia_is(ncm, nbs, dia)? {
                if item.imposto_seletivo.is_none() {
                    return Err(nao_informado());
                }
                return Err(Falha::new(
                    Classe::AliquotaImpostoSeletivoNaoInformada,
                    format!(
                        "A alíquota do Imposto Seletivo deve ser informada em aliquotasNominais.impostoSeletivo para {nome} {codigo} e data {}, pois ainda não foi definida em lei",
                        data_java(dia)
                    ),
                ));
            }
            return Ok(None);
        };
        let Some(is) = &item.imposto_seletivo else {
            return Err(nao_informado());
        };
        validar_quantidade_e_unidade(item, aliquota)?;
        self.validar_cst(is.cst.as_deref(), ID_IS, dia)?;
        let c = self.classificacao_is(is.c_class_trib.as_deref(), dia)?;
        validar_cst_classificacao(&c, is.cst.as_deref(), TRIBUTO_IS)?;
        Ok(Some(self.tratamento_classificacao(c.id, dia)?))
    }

    fn existe_incidencia_is(
        &self,
        ncm: Option<&str>,
        nbs: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<bool> {
        if let Some(n) = ncm.filter(|n| preenchido(Some(n)) && tamanho(Some(n)) == TAM_NCM_COMPLETO)
        {
            return Ok(self.db.existe_ncm_ad_valorem(n, ID_IS, dia)?
                || self.db.aliquota_ad_rem_produto(n, ID_IS, dia)?.is_some());
        }
        if let Some(n) = nbs.filter(|n| preenchido(Some(n)) && tamanho(Some(n)) == TAM_NBS_COMPLETO)
        {
            return self.db.existe_nbs_ad_valorem(n, ID_IS, dia);
        }
        Ok(false)
    }

    fn analisar_aliquota_is(
        &self,
        item: &Item,
        dia: NaiveDate,
    ) -> Resultado<Option<AliquotaImpostoSeletivo>> {
        let informadas: Option<&AliquotasNominaisIs> = item
            .imposto_seletivo
            .as_ref()
            .and(item.aliquotas_nominais.as_ref())
            .and_then(|a| a.imposto_seletivo.as_ref());
        let unidade_is = item
            .imposto_seletivo
            .as_ref()
            .and_then(|is| is.unidade.clone());
        if informadas.is_some_and(|a| a.ad_rem.is_some()) && branco(unidade_is.as_deref()) {
            return Err(generico(
                "A unidade de medida do Imposto Seletivo deve ser informada quando a alíquota ad rem é informada pelo usuário",
            ));
        }
        let mut ad_valorem = informadas.and_then(|a| a.ad_valorem.clone());
        let mut ad_rem = informadas.and_then(|a| a.ad_rem.clone());
        let mut unidade = if ad_rem.is_some() { unidade_is } else { None };
        let ncm = item.ncm.as_deref();
        let nbs = item.nbs.as_deref();
        if let Some(n) = ncm.filter(|n| preenchido(Some(n)) && tamanho(Some(n)) == TAM_NCM_COMPLETO)
        {
            if ad_valorem.is_none() {
                ad_valorem = self
                    .db
                    .aliquota_ad_valorem_produto(n, ID_IS, dia)?
                    .flatten();
            }
            if ad_rem.is_none()
                && let Some(banco) = self.db.aliquota_ad_rem_produto(n, ID_IS, dia)?
            {
                ad_rem = banco.valor;
                unidade = banco.unidade_medida;
            }
        } else if let Some(n) =
            nbs.filter(|n| preenchido(Some(n)) && tamanho(Some(n)) == TAM_NBS_COMPLETO)
            && ad_valorem.is_none()
        {
            ad_valorem = self.ad_valorem_servico(Some(n), ID_IS, None, dia)?;
        }
        if ad_valorem.is_none() && ad_rem.is_none() {
            return Ok(None);
        }
        Ok(Some(AliquotaImpostoSeletivo {
            ad_valorem,
            ad_rem,
            unidade_medida: unidade,
        }))
    }
}

fn validar_quantidade_e_unidade(item: &Item, aliquota: &AliquotaImpostoSeletivo) -> Resultado<()> {
    if let (Some(_), Some(is)) = (&aliquota.ad_rem, &item.imposto_seletivo) {
        match &is.quantidade {
            None => {
                return Err(generico(
                    "A quantidade do Imposto Seletivo deve ser informada para alíquota ad rem",
                ));
            }
            Some(q) if q.signum() <= 0 => {
                return Err(generico(
                    "A quantidade do Imposto Seletivo deve ser maior do que zero para alíquota ad rem",
                ));
            }
            Some(_) => {}
        }
        if is.unidade.is_none() {
            return Err(generico(
                "A unidade de medida do Imposto Seletivo deve ser informada para alíquota ad rem",
            ));
        }
    }
    if item.quantidade.as_ref().is_some_and(|q| q.signum() <= 0) {
        return Err(generico("A quantidade do item deve ser maior do que zero"));
    }
    Ok(())
}
