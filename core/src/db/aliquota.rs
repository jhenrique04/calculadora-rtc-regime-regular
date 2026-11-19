use chrono::NaiveDate;
use rusqlite::params;

use super::{Base, data, decimal};
use crate::decimal::BigDecimal;
use crate::erro::{Classe, Falha, Resultado};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormaAplicacao {
    Substituicao,
    Acrescimo,
    Decrescimo,
}

impl FormaAplicacao {
    pub fn nome(self) -> &'static str {
        match self {
            Self::Substituicao => "SUBSTITUICAO",
            Self::Acrescimo => "ACRESCIMO",
            Self::Decrescimo => "DECRESCIMO",
        }
    }

    fn de(texto: &str) -> Option<Self> {
        match texto {
            "SUBSTITUICAO" => Some(Self::Substituicao),
            "ACRESCIMO" => Some(Self::Acrescimo),
            "DECRESCIMO" => Some(Self::Decrescimo),
            _ => None,
        }
    }
}

/// `AliquotaResultadoDTO`: as validações do construtor Java rodam aqui
#[derive(Debug, Clone)]
pub struct AliquotaResultado {
    pub valor_referencia: BigDecimal,
    pub valor_padrao: Option<BigDecimal>,
    pub forma: Option<FormaAplicacao>,
}

impl AliquotaResultado {
    fn novo(
        referencia: Option<BigDecimal>,
        padrao: Option<BigDecimal>,
        forma: Option<FormaAplicacao>,
    ) -> Resultado<Self> {
        let Some(referencia) = referencia else {
            return Err(Falha::new(
                Classe::AliquotaReferenciaNaoEncontrada,
                "Alíquota de referência não encontrada",
            ));
        };
        if referencia.signum() < 0 {
            return Err(aliquota_negativa(
                "Alíquota de referência não pode ter valor negativo",
            ));
        }
        let valor_padrao = match padrao {
            Some(p) => Some(aplicar_forma(&referencia, p, forma)?),
            None if forma.is_some() => {
                return Err(Falha::new(
                    Classe::AliquotaPadraoNaoEncontrada,
                    "Alíquota padrão não encontrada",
                ));
            }
            None => None,
        };
        Ok(Self {
            valor_referencia: referencia,
            valor_padrao,
            forma,
        })
    }

    pub fn valor_aplicavel(&self) -> &BigDecimal {
        self.valor_padrao.as_ref().unwrap_or(&self.valor_referencia)
    }
}

fn aliquota_negativa(mensagem: &str) -> Falha {
    Falha::new(Classe::AliquotaNegativa, mensagem)
}

fn aplicar_forma(
    referencia: &BigDecimal,
    padrao: BigDecimal,
    forma: Option<FormaAplicacao>,
) -> Resultado<BigDecimal> {
    if padrao.signum() < 0 {
        return Err(aliquota_negativa(
            "Alíquota padrão não pode ter valor negativo",
        ));
    }
    let valor = match forma {
        None => {
            return Err(Falha::new(
                Classe::FormaAplicacaoNaoDefinida,
                "Forma de aplicação da alíquota própria não definida",
            ));
        }
        Some(FormaAplicacao::Acrescimo) => referencia.add(&padrao),
        Some(FormaAplicacao::Decrescimo) => referencia.subtract(&padrao),
        Some(FormaAplicacao::Substituicao) => padrao,
    };
    if valor.signum() < 0 {
        return Err(aliquota_negativa(
            "Alíquota aplicável resultante não pode ter valor negativo",
        ));
    }
    Ok(valor)
}

#[derive(Debug, Clone)]
pub struct AliquotaAdRem {
    pub valor: Option<BigDecimal>,
    pub unidade_medida: Option<String>,
}

impl Base<'_> {
    pub fn aliquota_referencia(
        &self,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<BigDecimal>> {
        let d = data(dia);
        let v = self.unica(
            "select ar1_0.alre_valor from aliquota_referencia ar1_0 where ar1_0.alre_tbto_id=? and ? between ar1_0.alre_inicio_vigencia and coalesce(ar1_0.alre_fim_vigencia,?)",
            params![id_tributo, d, d],
            |r| decimal(r, 0),
        )?;
        Ok(v.flatten())
    }

    pub fn aliquota_padrao(
        &self,
        id_tributo: i64,
        codigo_uf: Option<i64>,
        municipio: Option<i64>,
        dia: NaiveDate,
    ) -> Resultado<Option<AliquotaResultado>> {
        let d = data(dia);
        let linha = self.unica(
            "select ar1_0.alre_valor,ap1_0.alpa_valor,ap1_0.alpa_forma_aplicacao from aliquota_referencia ar1_0 left join aliquota_padrao ap1_0 on ap1_0.alpa_alre_id=ar1_0.alre_id and ((? is null and ap1_0.alpa_muni_cd is null) or (? is not null and ap1_0.alpa_muni_cd=?)) and ((? is null and ap1_0.alpa_uf_cd is null) or (? is not null and ap1_0.alpa_uf_cd=?)) and ap1_0.alpa_inicio_vigencia<=? and (ap1_0.alpa_fim_vigencia is null or ap1_0.alpa_fim_vigencia>=?) where ar1_0.alre_tbto_id=? and ar1_0.alre_inicio_vigencia<=? and (ar1_0.alre_fim_vigencia is null or ar1_0.alre_fim_vigencia>=?)",
            params![
                municipio, municipio, municipio, codigo_uf, codigo_uf, codigo_uf, d, d, id_tributo,
                d, d
            ],
            |r| Ok((decimal(r, 0)?, decimal(r, 1)?, r.get::<_, Option<String>>(2)?)),
        )?;
        let Some((referencia, padrao, forma)) = linha else {
            return Ok(None);
        };
        let forma = forma.as_deref().and_then(FormaAplicacao::de);
        AliquotaResultado::novo(referencia, padrao, forma).map(Some)
    }

    pub fn percentual_reducao(
        &self,
        id_classificacao: i64,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<BigDecimal>> {
        let d = data(dia);
        let v = self.unica(
            "select pr1_0.pere_valor from percentual_reducao pr1_0 where pr1_0.pere_cltr_id=? and pr1_0.pere_tbto_id=? and (pr1_0.pere_inicio_vigencia<=? and (pr1_0.pere_fim_vigencia is null or pr1_0.pere_fim_vigencia>=?))",
            params![id_classificacao, id_tributo, d, d],
            |r| decimal(r, 0),
        )?;
        Ok(v.flatten())
    }

    pub fn aliquota_ad_valorem_produto(
        &self,
        ncm: &str,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<Option<BigDecimal>>> {
        let d = data(dia);
        self.unica(
            "select aav1_0.aadv_valor from aliquota_ad_valorem_produto aavp1_0 join aliquota_ad_valorem aav1_0 on aav1_0.aadv_id=aavp1_0.aavp_aadv_id where exists(select 1 from ncm n1_0 where n1_0.ncm_cd=?) and aavp1_0.aavp_ncm_cd=substr(?,1,length(aavp1_0.aavp_ncm_cd)) and aav1_0.aadv_tbto_id=? and ? between aavp1_0.aavp_inicio_vigencia and coalesce(aavp1_0.aavp_fim_vigencia,?) and not exists(select 1 from excecao_ad_valorem_produto eavp1_0 where eavp1_0.eavp_ncm_cd=substr(?,1,length(eavp1_0.eavp_ncm_cd)) and eavp1_0.eavp_aavp_id=aavp1_0.aavp_id and ? between eavp1_0.eavp_inicio_vigencia and coalesce(eavp1_0.eavp_fim_vigencia,?)) and not exists(select 1 from excecao_ad_valorem_produto eavp2_0 where eavp2_0.eavp_ncm_cd=substr(aavp1_0.aavp_ncm_cd,1,length(eavp2_0.eavp_ncm_cd)) and eavp2_0.eavp_aavp_id=aavp1_0.aavp_id and ? between eavp2_0.eavp_inicio_vigencia and coalesce(eavp2_0.eavp_fim_vigencia,?)) order by length(aavp1_0.aavp_ncm_cd) desc limit 1",
            params![ncm, ncm, id_tributo, d, d, ncm, d, d, d, d],
            |r| decimal(r, 0),
        )
    }

    pub fn existe_ncm_ad_valorem(
        &self,
        ncm: &str,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<bool> {
        let d = data(dia);
        self.existe(
            "select case when count(1)>0 then 1 else 0 end from aliquota_ad_valorem_produto aavp1_0 join aliquota_ad_valorem aav1_0 on aav1_0.aadv_id=aavp1_0.aavp_aadv_id where exists(select 1 from ncm n1_0 where n1_0.ncm_cd=?) and aavp1_0.aavp_ncm_cd=substr(?,1,length(aavp1_0.aavp_ncm_cd)) and aav1_0.aadv_tbto_id=? and ? between aavp1_0.aavp_inicio_vigencia and coalesce(aavp1_0.aavp_fim_vigencia,?) and not exists(select 1 from excecao_ad_valorem_produto eavp1_0 where eavp1_0.eavp_ncm_cd=substr(?,1,length(eavp1_0.eavp_ncm_cd)) and eavp1_0.eavp_aavp_id=aavp1_0.aavp_id and ? between eavp1_0.eavp_inicio_vigencia and coalesce(eavp1_0.eavp_fim_vigencia,?)) and not exists(select 1 from excecao_ad_valorem_produto eavp2_0 where eavp2_0.eavp_ncm_cd=substr(aavp1_0.aavp_ncm_cd,1,length(eavp2_0.eavp_ncm_cd)) and eavp2_0.eavp_aavp_id=aavp1_0.aavp_id and ? between eavp2_0.eavp_inicio_vigencia and coalesce(eavp2_0.eavp_fim_vigencia,?))",
            params![ncm, ncm, id_tributo, d, d, ncm, d, d, d, d],
        )
    }

    pub fn aliquota_ad_rem_produto(
        &self,
        ncm: &str,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<AliquotaAdRem>> {
        let d = data(dia);
        self.unica(
            "select aar1_0.aare_valor,um1_0.unmd_sigla from aliquota_ad_rem_produto aarp1_0 join aliquota_ad_rem aar1_0 on aar1_0.aare_id=aarp1_0.aarp_aare_id join unidade_medida um1_0 on um1_0.unmd_id=aar1_0.aare_unmd_id where exists(select 1 from ncm n1_0 where n1_0.ncm_cd=?) and aar1_0.aare_tbto_id=? and aarp1_0.aarp_ncm_cd=substr(?,1,length(aarp1_0.aarp_ncm_cd)) and ? between aarp1_0.aarp_inicio_vigencia and coalesce(aarp1_0.aarp_fim_vigencia,?) and not exists(select 1 from excecao_ad_rem_produto earp1_0 where earp1_0.earp_ncm_cd=substr(?,1,length(earp1_0.earp_ncm_cd)) and earp1_0.earp_aarp_id=aarp1_0.aarp_id and ? between earp1_0.earp_inicio_vigencia and coalesce(earp1_0.earp_fim_vigencia,?)) and not exists(select 1 from excecao_ad_rem_produto earp2_0 where earp2_0.earp_ncm_cd=substr(aarp1_0.aarp_ncm_cd,1,length(earp2_0.earp_ncm_cd)) and earp2_0.earp_aarp_id=aarp1_0.aarp_id and ? between earp2_0.earp_inicio_vigencia and coalesce(earp2_0.earp_fim_vigencia,?)) order by length(aarp1_0.aarp_ncm_cd) desc limit 1",
            params![ncm, id_tributo, ncm, d, d, ncm, d, d, d, d],
            |r| {
                Ok(AliquotaAdRem {
                    valor: decimal(r, 0)?,
                    unidade_medida: r.get(1)?,
                })
            },
        )
    }

    /// sem `LIMIT`: mais de uma linha é erro (tratado pelo serviço)
    pub fn aliquota_ad_valorem_servico(
        &self,
        nbs: Option<&str>,
        id_tributo: i64,
        id_classificacao: Option<i64>,
        dia: NaiveDate,
    ) -> Resultado<Option<Option<BigDecimal>>> {
        let d = data(dia);
        self.unica(
            "select aav1_0.aadv_valor from aliquota_ad_valorem_servico aavs1_0 join aliquota_ad_valorem aav1_0 on aav1_0.aadv_id=aavs1_0.aavs_aadv_id where exists(select 1 from nbs n1_0 where n1_0.nbs_cd=?) and aavs1_0.aavs_nbs_cd=substr(?,1,length(aavs1_0.aavs_nbs_cd)) and aav1_0.aadv_tbto_id=? and (? is null or aav1_0.aadv_cltr_id=?) and ? between aavs1_0.aavs_inicio_vigencia and coalesce(aavs1_0.aavs_fim_vigencia,?) and not exists(select 1 from excecao_ad_valorem_servico eavs1_0 where eavs1_0.eavs_nbs_cd=substr(?,1,length(eavs1_0.eavs_nbs_cd)) and eavs1_0.eavs_aavs_id=aavs1_0.aavs_id and ? between eavs1_0.eavs_inicio_vigencia and coalesce(eavs1_0.eavs_fim_vigencia,?)) and not exists(select 1 from excecao_ad_valorem_servico eavs2_0 where eavs2_0.eavs_nbs_cd=substr(aavs1_0.aavs_nbs_cd,1,length(eavs2_0.eavs_nbs_cd)) and eavs2_0.eavs_aavs_id=aavs1_0.aavs_id and ? between eavs2_0.eavs_inicio_vigencia and coalesce(eavs2_0.eavs_fim_vigencia,?)) order by length(aavs1_0.aavs_nbs_cd) desc",
            params![
                nbs,
                nbs,
                id_tributo,
                id_classificacao,
                id_classificacao,
                d,
                d,
                nbs,
                d,
                d,
                d,
                d
            ],
            |r| decimal(r, 0),
        )
    }

    pub fn existe_nbs_ad_valorem(
        &self,
        nbs: &str,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<bool> {
        let d = data(dia);
        self.existe(
            "select case when count(1)>0 then 1 else 0 end from aliquota_ad_valorem_servico aavs1_0 join aliquota_ad_valorem aav1_0 on aav1_0.aadv_id=aavs1_0.aavs_aadv_id where exists(select 1 from nbs n1_0 where n1_0.nbs_cd=?) and aavs1_0.aavs_nbs_cd=substr(?,1,length(aavs1_0.aavs_nbs_cd)) and aav1_0.aadv_tbto_id=? and ? between aavs1_0.aavs_inicio_vigencia and coalesce(aavs1_0.aavs_fim_vigencia,?) and not exists(select 1 from excecao_ad_valorem_servico eavs1_0 where eavs1_0.eavs_nbs_cd=substr(?,1,length(eavs1_0.eavs_nbs_cd)) and eavs1_0.eavs_aavs_id=aavs1_0.aavs_id and ? between eavs1_0.eavs_inicio_vigencia and coalesce(eavs1_0.eavs_fim_vigencia,?)) and not exists(select 1 from excecao_ad_valorem_servico eavs2_0 where eavs2_0.eavs_nbs_cd=substr(aavs1_0.aavs_nbs_cd,1,length(eavs2_0.eavs_nbs_cd)) and eavs2_0.eavs_aavs_id=aavs1_0.aavs_id and ? between eavs2_0.eavs_inicio_vigencia and coalesce(eavs2_0.eavs_fim_vigencia,?))",
            params![nbs, nbs, id_tributo, d, d, nbs, d, d, d, d],
        )
    }

    pub fn aliquota_ad_valorem_por_classificacao(
        &self,
        id_tributo: i64,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<Option<BigDecimal>> {
        let d = data(dia);
        let v = self.unica(
            "SELECT aadv.AADV_VALOR FROM ALIQUOTA_AD_VALOREM aadv WHERE aadv.AADV_TBTO_ID = ? AND aadv.AADV_CLTR_ID = ? AND ? BETWEEN aadv.AADV_INICIO_VIGENCIA AND COALESCE(aadv.AADV_FIM_VIGENCIA, ?)",
            params![id_tributo, id_classificacao, d, d],
            |r| decimal(r, 0),
        )?;
        Ok(v.flatten())
    }
}
