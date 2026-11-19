//! serviços de consulta que transformam "não encontrado" na exceção Java correspondente

use chrono::NaiveDate;

use super::Servicos;
use super::texto::{data_java, exibir};
use crate::db::{
    AliquotaResultado, ClassificacaoCalculo, ClassificacaoTributaria, Fundamentacao,
    SituacaoTributaria, TipoDfe, TratamentoClassificacao, TratamentoTributario,
};
use crate::decimal::BigDecimal;
use crate::erro::{Classe, Falha, Resultado};
use crate::modelo::entrada::TipoEnteGov;

pub const ID_IS: i64 = 1;
pub const ID_CBS: i64 = 2;
pub const TRIBUTOS_CBS_IBS: &str = "CBS e IBS";
pub const TRIBUTO_IS: &str = "Imposto Seletivo";
const MSG_MULTIPLOS: &str = "Consulta retornou múltiplos valores quando apenas um era esperado.";

fn opcional<T: std::fmt::Display>(v: Option<T>) -> String {
    v.map_or_else(|| "null".to_string(), |v| v.to_string())
}

fn nome_tributo(id_tributo: i64) -> &'static str {
    if id_tributo == ID_IS {
        TRIBUTO_IS
    } else {
        TRIBUTOS_CBS_IBS
    }
}

/// `IncorrectResultSizeDataAccessException` vira `ErroGenericoValidacaoException`
pub(crate) fn sem_multiplos<T>(r: Resultado<T>) -> Resultado<T> {
    r.map_err(|f| match f.classe {
        Classe::IncorrectResultSizeDataAccess => {
            Falha::new(Classe::ErroGenericoValidacao, MSG_MULTIPLOS)
        }
        _ => f,
    })
}

impl Servicos<'_> {
    pub fn codigo_uf(&self, sigla: Option<&str>) -> Resultado<i64> {
        let codigo = match sigla {
            Some(s) => self.db.codigo_uf(s)?,
            None => None,
        };
        codigo.ok_or_else(|| uf_nao_encontrada(sigla))
    }

    pub fn validar_uf(&self, sigla: Option<&str>) -> Resultado<()> {
        let existe = match sigla {
            Some(s) => self.db.existe_uf(s)?,
            None => false,
        };
        if existe {
            Ok(())
        } else {
            Err(uf_nao_encontrada(sigla))
        }
    }

    pub fn validar_municipio(&self, codigo: i64, sigla_uf: &str) -> Resultado<()> {
        match self.db.validar_municipio(codigo, sigla_uf)? {
            1 => Err(municipio_nao_encontrado(codigo)),
            2 => Err(Falha::new(
                Classe::MunicipioNaoPertencente,
                format!("Município de código {codigo} não pertence à UF de sigla {sigla_uf}"),
            )),
            _ => Ok(()),
        }
    }

    pub fn uf_do_municipio(&self, codigo: i64) -> Resultado<String> {
        match self.db.sigla_uf_do_municipio(codigo)? {
            None => Err(municipio_nao_encontrado(codigo)),
            Some(Some(sigla)) => Ok(sigla),
            Some(None) => Err(Falha::sistema(
                "Cannot invoke \"br.gov.serpro.rtc.domain.model.entity.Uf.getSigla()\" because the return value of \"br.gov.serpro.rtc.domain.model.entity.Municipio.getUf()\" is null",
            )),
        }
    }

    pub fn tipo_dfe_por_tipo(&self, tp_doc: i32, dia: NaiveDate) -> Resultado<TipoDfe> {
        self.db.tipo_dfe_por_tipo(tp_doc, dia)?.ok_or_else(|| {
            Falha::new(
                Classe::TipoDfeNaoEncontrado,
                format!(
                    "Tipo de documento fiscal (tpDoc) de código {tp_doc} não encontrado ou fora de vigência na data {}",
                    data_java(dia)
                ),
            )
        })
    }

    fn classificacao(
        &self,
        codigo: Option<&str>,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<ClassificacaoTributaria> {
        let c = match codigo {
            Some(c) => self.db.classificacao_tributaria(c, id_tributo, dia)?,
            None => None,
        };
        c.ok_or_else(|| {
            Falha::new(
                Classe::ClassificacaoTributariaNaoEncontrada,
                format!(
                    "Classificação tributária não encontrada para código {} e data {} ({})",
                    exibir(codigo),
                    data_java(dia),
                    nome_tributo(id_tributo)
                ),
            )
        })
    }

    pub fn classificacao_cbs_ibs(
        &self,
        codigo: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<ClassificacaoTributaria> {
        self.classificacao(codigo, ID_CBS, dia)
    }

    pub fn classificacao_is(
        &self,
        codigo: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<ClassificacaoTributaria> {
        self.classificacao(codigo, ID_IS, dia)
    }

    pub fn classificacao_calculo(&self, id: i64) -> Resultado<ClassificacaoCalculo> {
        self.db.classificacao_calculo(id)?.ok_or_else(|| {
            Falha::new(
                Classe::ClassificacaoTributariaNaoEncontrada,
                format!("Classificação tributária não encontrada para id {id}"),
            )
        })
    }

    pub fn tratamento_classificacao(
        &self,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<TratamentoClassificacao> {
        self.db.tratamento_classificacao(id_classificacao, dia)?.ok_or_else(|| {
            Falha::new(
                Classe::TratamentoClassificacaoNaoEncontrado,
                format!(
                    "Tratamento tributário não encontrado para classificação tributária com id {id_classificacao} e data {}",
                    data_java(dia)
                ),
            )
        })
    }

    pub fn tratamento_tributario(&self, id: i64) -> Resultado<TratamentoTributario> {
        self.db.tratamento_tributario(id)?.ok_or_else(|| {
            Falha::new(
                Classe::TratamentoTributarioNaoEncontrado,
                format!("Tratamento tributário com id {id} não encontrado"),
            )
        })
    }

    pub fn validar_cst(&self, cst: Option<&str>, id_tributo: i64, dia: NaiveDate) -> Resultado<()> {
        let existe = match cst {
            Some(c) => self.db.existe_cst(c, id_tributo, dia)?,
            None => false,
        };
        if existe {
            return Ok(());
        }
        Err(Falha::new(
            Classe::SituacaoTributariaNaoEncontrada,
            format!(
                "Situação tributária (CST) não encontrada para código {} e data {} ({})",
                exibir(cst),
                data_java(dia),
                nome_tributo(id_tributo)
            ),
        ))
    }

    pub fn situacao_tributaria(
        &self,
        cst: Option<&str>,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<SituacaoTributaria>> {
        match cst {
            Some(c) => self.db.situacao_tributaria_por_codigo(c, id_tributo, dia),
            None => Ok(None),
        }
    }

    pub fn redutor_compra_gov(
        &self,
        tp_ente: Option<TipoEnteGov>,
        dia: NaiveDate,
    ) -> Resultado<BigDecimal> {
        if tp_ente.is_none() {
            return Ok(BigDecimal::zero());
        }
        self.db
            .valor_redutor_compra_gov(dia)?
            .ok_or_else(|| Falha::sistema("Redutor de compra governamental não encontrada."))
    }

    pub fn percentual_transferencia_cbs(&self, dia: NaiveDate) -> Resultado<BigDecimal> {
        self.db.percentual_transferencia_cbs(dia)?.ok_or_else(|| {
            Falha::sistema(
                "Percentual de transferência de CBS para ente federativo não encontrada.",
            )
        })
    }

    pub fn aliquota_padrao(
        &self,
        id_tributo: i64,
        codigo_uf: Option<i64>,
        municipio: Option<i64>,
        dia: NaiveDate,
    ) -> Resultado<AliquotaResultado> {
        self.db
            .aliquota_padrao(id_tributo, codigo_uf, municipio, dia)?
            .ok_or_else(|| {
                Falha::new(
                    Classe::AliquotaNaoEncontrada,
                    format!(
                        "Alíquota de referencia ou padrão não encontrada para o tributo com id {id_tributo}, UF {}, município {} e data {}",
                        opcional(codigo_uf),
                        opcional(municipio),
                        data_java(dia)
                    ),
                )
            })
    }

    pub fn aliquota_referencia(&self, id_tributo: i64, dia: NaiveDate) -> Resultado<BigDecimal> {
        self.db.aliquota_referencia(id_tributo, dia)?.ok_or_else(|| {
            Falha::new(
                Classe::AliquotaReferenciaNaoEncontrada,
                format!(
                    "Alíquota de referência não encontrada para o tributo com id {id_tributo} e data {}",
                    data_java(dia)
                ),
            )
        })
    }

    pub fn percentual_reducao(
        &self,
        id_classificacao: i64,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<BigDecimal> {
        self.db
            .percentual_reducao(id_classificacao, id_tributo, dia)?
            .ok_or_else(|| {
                Falha::new(
                    Classe::PercentualReducaoNaoEncontrado,
                    format!(
                        "Percentual de redução não encontrado para classificação tributária com id {id_classificacao} e data {}",
                        data_java(dia)
                    ),
                )
            })
    }

    pub fn fundamentacao(&self, id_classificacao: i64, dia: NaiveDate) -> Resultado<Fundamentacao> {
        self.db.fundamentacao(id_classificacao, dia)?.ok_or_else(|| {
            Falha::new(
                Classe::FundamentacaoClassificacaoNaoEncontrada,
                format!(
                    "Fundamentação legal não encontrada para classificação tributária de id {id_classificacao} e data {}",
                    data_java(dia)
                ),
            )
        })
    }

    pub fn ad_valorem_servico(
        &self,
        nbs: Option<&str>,
        id_tributo: i64,
        id_classificacao: Option<i64>,
        dia: NaiveDate,
    ) -> Resultado<Option<BigDecimal>> {
        let v = sem_multiplos(self.db.aliquota_ad_valorem_servico(
            nbs,
            id_tributo,
            id_classificacao,
            dia,
        ))?;
        Ok(v.flatten())
    }

    pub fn ad_valorem_por_classificacao(
        &self,
        id_tributo: i64,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<BigDecimal>> {
        sem_multiplos(self.db.aliquota_ad_valorem_por_classificacao(
            id_tributo,
            id_classificacao,
            dia,
        ))
    }
}

fn uf_nao_encontrada(sigla: Option<&str>) -> Falha {
    Falha::new(
        Classe::UfNaoEncontrada,
        format!("UF de sigla {} não encontrada", exibir(sigla)),
    )
}

fn municipio_nao_encontrado(codigo: i64) -> Falha {
    Falha::new(
        Classe::MunicipioNaoEncontrado,
        format!("Município de código {codigo} não encontrado"),
    )
}
