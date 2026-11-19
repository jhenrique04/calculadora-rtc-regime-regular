//! `PedagioService`: CBS/IBS do pedágio rateados pela extensão de cada trecho

use std::collections::HashSet;

use chrono::NaiveDate;

use super::Servicos;
use crate::decimal::BigDecimal;
use crate::erro::{Classe, Falha, Resultado};
use crate::json::DataHora;
use crate::modelo::pedagio::{PedagioEntrada, TrechoEntrada};
use crate::saida_json;
use crate::util::arredondamento::{dividir, dividir_por_cem, multiplicar};

const ID_CBS: i64 = 2;
const ID_IBS_UF: i64 = 3;
const ID_IBS_MUN: i64 = 4;
const CASAS_PERCENTUAL: i64 = 2;

#[derive(Debug, Clone)]
pub struct TributoPedagioSaida {
    pub aliquota: Option<BigDecimal>,
    pub aliquota_efetiva: Option<BigDecimal>,
    pub tributo_calculado: Option<BigDecimal>,
}

saida_json!(TributoPedagioSaida {
    "aliquota" => aliquota: decimal,
    "aliquotaEfetiva" => aliquota_efetiva: decimal,
    "tributoCalculado" => tributo_calculado: decimal,
});

#[derive(Debug, Clone)]
pub struct TrechoPedagioSaida {
    pub numero: Option<i32>,
    pub municipio: Option<i64>,
    pub uf: Option<String>,
    pub base_calculo: Option<BigDecimal>,
    pub extensao_trecho: Option<BigDecimal>,
    pub cbs: Option<TributoPedagioSaida>,
    pub ibs_estadual: Option<TributoPedagioSaida>,
    pub ibs_municipal: Option<TributoPedagioSaida>,
}

saida_json!(TrechoPedagioSaida {
    "numero" => numero: inteiro,
    "municipio" => municipio: inteiro,
    "uf" => uf: texto,
    "baseCalculo" => base_calculo: decimal,
    "extensaoTrecho" => extensao_trecho: decimal,
    "cbs" => cbs: objeto,
    "ibsEstadual" => ibs_estadual: objeto,
    "ibsMunicipal" => ibs_municipal: objeto,
});

#[derive(Debug, Clone)]
pub struct TributoTotalPedagioSaida {
    pub base_calculo: Option<BigDecimal>,
    pub valor_apurado: Option<BigDecimal>,
    pub valor_devido: Option<BigDecimal>,
    pub valor_tributo: Option<BigDecimal>,
}

saida_json!(TributoTotalPedagioSaida {
    "baseCalculo" => base_calculo: moeda,
    "valorApurado" => valor_apurado: moeda,
    "valorDevido" => valor_devido: moeda,
    "valorTributo" => valor_tributo: moeda,
});

#[derive(Debug, Clone)]
pub struct TotalPedagioSaida {
    pub cbs: Option<TributoTotalPedagioSaida>,
    pub ibs_estadual: Option<TributoTotalPedagioSaida>,
    pub ibs_municipal: Option<TributoTotalPedagioSaida>,
}

saida_json!(TotalPedagioSaida {
    "cbsTotal" => cbs: objeto,
    "ibsEstadualTotal" => ibs_estadual: objeto,
    "ibsMunicipalTotal" => ibs_municipal: objeto,
});

#[derive(Debug, Clone)]
pub struct PedagioSaida {
    pub data_hora_emissao: Option<DataHora>,
    pub municipio_origem: Option<i64>,
    pub uf_origem: Option<String>,
    pub cst: Option<String>,
    pub c_class_trib: Option<String>,
    pub base_calculo: Option<BigDecimal>,
    pub extensao_total: Option<BigDecimal>,
    pub trechos: Option<Vec<TrechoPedagioSaida>>,
    pub total: Option<TotalPedagioSaida>,
}

saida_json!(PedagioSaida {
    "dataHoraEmissao" => data_hora_emissao: data_hora,
    "municipioOrigem" => municipio_origem: inteiro,
    "ufMunicipioOrigem" => uf_origem: texto,
    "cst" => cst: texto,
    "cClassTrib" => c_class_trib: texto,
    "baseCalculo" => base_calculo: decimal,
    "extensaoTotal" => extensao_total: decimal,
    "trechos" => trechos: lista,
    "total" => total: objeto,
});

/// NPE de campo já validado como `@NotNull`, só alcançável por bug
fn campo_nulo(campo: &str) -> Falha {
    Falha::sistema(format!("campo obrigatório nulo: {campo}"))
}

fn trecho_nulo(metodo: &str) -> Falha {
    Falha::sistema(format!(
        "Cannot invoke \"br.gov.serpro.rtc.api.model.input.pedagio.TrechoPedagioInput.{metodo}()\" because \"trecho\" is null"
    ))
}

/// `BigDecimal.divide(divisor, escala, modo)` cai na divisão inteira com zero
fn divisao_por_zero() -> Falha {
    Falha::sistema("/ by zero")
}

fn trechos(e: &PedagioEntrada) -> Resultado<Vec<&TrechoEntrada>> {
    e.trechos
        .iter()
        .flatten()
        .map(|t| t.as_ref().ok_or_else(|| trecho_nulo("getNumero")))
        .collect()
}

fn validar_numeros_unicos(trechos: &[&TrechoEntrada]) -> Resultado<()> {
    let mut vistos = HashSet::new();
    for t in trechos {
        if !vistos.insert(t.numero) {
            let numero = t
                .numero
                .map_or_else(|| "null".to_string(), |n| n.to_string());
            return Err(Falha::new(
                Classe::CampoInvalido,
                format!("Número de trecho duplicado: {numero}"),
            ));
        }
    }
    Ok(())
}

fn tributo(aliquota: &BigDecimal, efetiva: &BigDecimal, base: &BigDecimal) -> TributoPedagioSaida {
    TributoPedagioSaida {
        aliquota: Some(aliquota.move_point_right(CASAS_PERCENTUAL)),
        aliquota_efetiva: Some(efetiva.move_point_right(CASAS_PERCENTUAL)),
        tributo_calculado: Some(multiplicar(base, efetiva)),
    }
}

fn total_do_tributo(
    trechos: &[TrechoPedagioSaida],
    base: Option<&BigDecimal>,
    campo: fn(&TrechoPedagioSaida) -> Option<&TributoPedagioSaida>,
) -> TributoTotalPedagioSaida {
    let total = trechos
        .iter()
        .filter_map(|t| campo(t).and_then(|c| c.tributo_calculado.as_ref()))
        .fold(BigDecimal::zero(), |acc, v| acc.add(v));
    TributoTotalPedagioSaida {
        base_calculo: base.cloned(),
        valor_apurado: Some(total.clone()),
        valor_devido: Some(total.clone()),
        valor_tributo: Some(total),
    }
}

/// `TotalPedagioOutput.getTotal`: a base é a do primeiro trecho
fn total(trechos: &[TrechoPedagioSaida]) -> TotalPedagioSaida {
    let base = trechos.first().and_then(|t| t.base_calculo.as_ref());
    TotalPedagioSaida {
        cbs: Some(total_do_tributo(trechos, base, |t| t.cbs.as_ref())),
        ibs_estadual: Some(total_do_tributo(trechos, base, |t| t.ibs_estadual.as_ref())),
        ibs_municipal: Some(total_do_tributo(trechos, base, |t| {
            t.ibs_municipal.as_ref()
        })),
    }
}

/// `getWarningDadosSimulados`: a partir de 2027 os dados são simulados
pub fn dados_simulados(e: &PedagioEntrada) -> bool {
    e.data_hora_emissao
        .as_ref()
        .is_some_and(|d| super::dados_abertos::dados_simulados(d.data()))
}

impl Servicos<'_> {
    fn aliquota_padrao_decimal(
        &self,
        id_tributo: i64,
        uf: Option<i64>,
        municipio: Option<i64>,
        dia: NaiveDate,
    ) -> Resultado<BigDecimal> {
        let a = self.aliquota_padrao(id_tributo, uf, municipio, dia)?;
        Ok(dividir_por_cem(a.valor_aplicavel()))
    }

    fn calcular_trecho(
        &self,
        t: &TrechoEntrada,
        base: &BigDecimal,
        extensao_total: &BigDecimal,
        dia: NaiveDate,
    ) -> Resultado<TrechoPedagioSaida> {
        let codigo_uf = self.codigo_uf(t.uf.as_deref())?;
        let cbs = self.aliquota_padrao_decimal(ID_CBS, None, None, dia)?;
        let ibs_uf = self.aliquota_padrao_decimal(ID_IBS_UF, Some(codigo_uf), None, dia)?;
        let ibs_mun = self.aliquota_padrao_decimal(ID_IBS_MUN, None, t.municipio, dia)?;
        let extensao = t.extensao.as_ref().ok_or_else(|| campo_nulo("extensao"))?;
        let efetiva = |aliquota: &BigDecimal| {
            let produto = aliquota.multiply(extensao);
            dividir(&produto, extensao_total).ok_or_else(divisao_por_zero)
        };
        Ok(TrechoPedagioSaida {
            numero: t.numero,
            municipio: t.municipio,
            uf: t.uf.clone(),
            base_calculo: Some(base.clone()),
            extensao_trecho: Some(extensao.clone()),
            cbs: Some(tributo(&cbs, &efetiva(&cbs)?, base)),
            ibs_estadual: Some(tributo(&ibs_uf, &efetiva(&ibs_uf)?, base)),
            ibs_municipal: Some(tributo(&ibs_mun, &efetiva(&ibs_mun)?, base)),
        })
    }

    pub fn calcular_pedagio(&self, e: &PedagioEntrada) -> Resultado<PedagioSaida> {
        let lista = trechos(e)?;
        validar_numeros_unicos(&lista)?;
        let origem = e
            .municipio_origem
            .ok_or_else(|| campo_nulo("codigoMunicipioOrigem"))?;
        self.validar_municipio(origem, e.uf_origem.as_deref().unwrap_or_default())?;
        for t in &lista {
            let municipio = t.municipio.ok_or_else(|| campo_nulo("municipio"))?;
            self.validar_municipio(municipio, t.uf.as_deref().unwrap_or_default())?;
        }
        let dia = e
            .data_hora_emissao
            .as_ref()
            .ok_or_else(|| campo_nulo("dataHoraEmissao"))?
            .data();
        let base = e
            .base_calculo
            .as_ref()
            .ok_or_else(|| campo_nulo("baseCalculo"))?;
        let extensao_total = e.extensao_total();
        let mut saida = lista
            .iter()
            .map(|t| self.calcular_trecho(t, base, &extensao_total, dia))
            .collect::<Resultado<Vec<_>>>()?;
        saida.sort_by_key(|t| t.numero);
        Ok(PedagioSaida {
            data_hora_emissao: e.data_hora_emissao,
            municipio_origem: e.municipio_origem,
            uf_origem: e.uf_origem.clone(),
            cst: e.cst.clone(),
            c_class_trib: e.c_class_trib.clone(),
            base_calculo: e.base_calculo.clone(),
            extensao_total: Some(extensao_total),
            total: Some(total(&saida)),
            trechos: Some(saida),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trecho(numero: i32) -> TrechoEntrada {
        TrechoEntrada {
            numero: Some(numero),
            ..TrechoEntrada::default()
        }
    }

    #[test]
    fn numero_duplicado() {
        let (a, b, c) = (trecho(1), trecho(2), trecho(1));
        let msg = validar_numeros_unicos(&[&a, &b, &c])
            .err()
            .and_then(|f| f.mensagem);
        assert_eq!(msg.as_deref(), Some("Número de trecho duplicado: 1"));
    }

    #[test]
    fn total_usa_base_do_primeiro_trecho() {
        let d = |s: &str| s.parse::<BigDecimal>().ok();
        let t = TrechoPedagioSaida {
            numero: Some(1),
            municipio: None,
            uf: None,
            base_calculo: d("200"),
            extensao_trecho: None,
            cbs: Some(TributoPedagioSaida {
                aliquota: None,
                aliquota_efetiva: None,
                tributo_calculado: d("0.18000000"),
            }),
            ibs_estadual: None,
            ibs_municipal: None,
        };
        let tot = total(&[t.clone(), t]);
        let cbs = tot.cbs.and_then(|c| c.valor_tributo).map(|v| v.to_string());
        assert_eq!(cbs.as_deref(), Some("0.36000000"));
        let ibs = tot
            .ibs_estadual
            .and_then(|c| c.valor_tributo)
            .map(|v| v.to_string());
        assert_eq!(ibs.as_deref(), Some("0"));
    }
}
