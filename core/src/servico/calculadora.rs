use chrono::NaiveDate;

use super::Servicos;
use super::nomenclatura::SiglaDfe;
use crate::db::TipoDfe;
use crate::erro::{Classe, Falha, Resultado};
use crate::modelo::entrada::{Item, Operacao};
use crate::modelo::roc::{CompraGovSaida, Objeto, OperacaoConsumo, Roc, Tributos, ValoresTotais};

const ULTIMO_DIA_2025: (i32, u32, u32) = (2025, 12, 31);
const ULTIMO_DIA_2026: (i32, u32, u32) = (2026, 12, 31);
const CST_ALIQUOTAS_FICTICIAS: [&str; 3] = ["010", "220", "221"];

/// `TipoWarningDadosSimulados` alcançáveis: a partir de 2027 o Java sai antes
/// dos casos de compra governamental/IS, que ficaram inalcançáveis
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AvisoDadosSimulados {
    CasoAliquotasFicticias = 5,
    CasoCalculosSimulados = 6,
}

fn data((a, m, d): (i32, u32, u32)) -> NaiveDate {
    NaiveDate::from_ymd_opt(a, m, d).unwrap_or(NaiveDate::MIN)
}

/// contexto da operação já validado (UF resolvida, data e tipo de DFe)
pub struct Contexto {
    pub uf: String,
    pub dia: NaiveDate,
    pub tipo_dfe: Option<TipoDfe>,
}

pub fn item_nulo() -> Falha {
    Falha::sistema(
        "Cannot invoke \"br.gov.serpro.rtc.api.model.input.ItemOperacaoInput.getNumero()\" because \"item\" is null",
    )
}

impl Servicos<'_> {
    /// UF, município, data do fato gerador e tipo de DFe, na ordem do Java
    pub fn contexto(&self, op: &Operacao) -> Resultado<Contexto> {
        let uf = match &op.uf {
            Some(uf) => uf.clone(),
            None => match op.municipio {
                Some(m) => self.uf_do_municipio(m)?,
                None => return Err(Falha::sistema("The given id must not be null")),
            },
        };
        self.validar_uf(Some(&uf))?;
        self.validar_municipio(op.municipio.unwrap_or_default(), &uf)?;
        let dia = op.fato_gerador()?;
        let tipo_dfe = match op.tp_doc {
            Some(tp) => Some(self.tipo_dfe(tp, dia)?),
            None => None,
        };
        Ok(Contexto { uf, dia, tipo_dfe })
    }

    fn tipo_dfe(&self, tp_doc: i32, dia: NaiveDate) -> Resultado<TipoDfe> {
        let t = self.tipo_dfe_por_tipo(tp_doc, dia)?;
        let sigla = SiglaDfe::por_sigla(&t.sigla)?;
        if sigla.sem_tratamento_calculadora() {
            return Err(Falha::new(
                Classe::ErroFaltaImplementacao,
                format!(
                    "Tipo de documento fiscal (tpDoc) de código {tp_doc} ({}) não possui tratamento implementado na Calculadora",
                    sigla.sigla()
                ),
            ));
        }
        Ok(t)
    }

    pub fn processar_item_no_contexto(
        &self,
        op: &Operacao,
        ctx: &Contexto,
        item: &Item,
    ) -> Resultado<Tributos> {
        self.processar_item(op, Some(&ctx.uf), item, ctx.dia, ctx.tipo_dfe.as_ref())
    }

    pub fn calcular_regime_geral(&self, op: &Operacao) -> Resultado<Roc> {
        let ctx = self.contexto(op)?;
        let mut objetos = Vec::new();
        for item in op.itens.iter().flatten() {
            let item = item.as_ref().ok_or_else(item_nulo)?;
            objetos.push(Objeto {
                n_obj: item.numero,
                trib_calc: Some(self.processar_item_no_contexto(op, &ctx, item)?),
                calculo_simulado: item.aliquotas_nominais.is_some(),
            });
        }
        // nullsFirst + ordenação estável, como o sorted() do Java
        objetos.sort_by_key(|o| o.n_obj);
        let tributos: Vec<&Tributos> = objetos
            .iter()
            .filter_map(|o| o.trib_calc.as_ref())
            .collect();
        let total = ValoresTotais::de(&tributos);
        Ok(Roc {
            oper: self.operacao_consumo(op, ctx.dia)?,
            objetos: Some(objetos),
            total: Some(total),
        })
    }

    pub fn operacao_consumo(
        &self,
        op: &Operacao,
        dia: NaiveDate,
    ) -> Resultado<Option<OperacaoConsumo>> {
        let Some(g) = &op.compra_gov else {
            return Ok(None);
        };
        Ok(Some(OperacaoConsumo {
            g_compra_gov: Some(CompraGovSaida {
                tp_ente_gov: g.tp_ente_gov.map(|t| t.codigo()),
                p_redutor: Some(self.redutor_compra_gov(op.tp_ente_gov(), dia)?),
                tp_oper_gov: g.tp_oper_gov.map(|t| t.0),
            }),
        }))
    }
}

pub fn aviso_dados_simulados(op: &Operacao) -> Resultado<Option<AvisoDadosSimulados>> {
    let dia = op.fato_gerador()?;
    if dia > data(ULTIMO_DIA_2026) {
        return Ok(Some(AvisoDadosSimulados::CasoCalculosSimulados));
    }
    let ficticias = op.itens.iter().flatten().flatten().any(|item| {
        item.cst
            .as_deref()
            .is_some_and(|c| CST_ALIQUOTAS_FICTICIAS.contains(&c))
    });
    if ficticias && dia > data(ULTIMO_DIA_2025) {
        return Ok(Some(AvisoDadosSimulados::CasoAliquotasFicticias));
    }
    Ok(None)
}
