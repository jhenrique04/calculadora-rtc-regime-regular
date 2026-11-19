//! `DadosAbertosService`: consultas públicas de tabelas da calculadora

use chrono::{NaiveDate, Utc};

use super::Servicos;
use super::consulta::{ID_IS, TRIBUTOS_CBS_IBS, sem_multiplos};
use super::nomenclatura::{Alvo, SiglaDfe, nao_encontrada};
use super::texto::{branco, data_java};
use crate::db::{
    CodigoDescricao, CstClassificacao, LinhaClassificacao, LinhaGrupoAtor, LinhaGrupoDfe,
    Nomenclatura, TipoDfe, Vigencia,
};
use crate::decimal::BigDecimal;
use crate::erro::{Classe, Falha, Resultado};
use crate::json::{ErroEscrita, Escrever, Escritor};
use crate::saida_json;

const TRIBUTOS_IS: &str = "IS";
const AMBIENTE: &str = "offline";
const TAM_DATA_ISO: usize = 10;
const NOMENCLATURA_SEM: &str = "SEM";
const MARCADOR_NBS: &str = "NBS";
const SUFIXO_SNAPSHOT: &str = "-SNAPSHOT-";
const VALOR_IBS_TRANSICAO: i64 = 0;
const VALOR_IBS_PLENO: i64 = 100;
const ANO_INICIO_IBS: i32 = 2026;
const ANO_IBS_PLENO: i32 = 2027;
const INICIO_DADOS_SIMULADOS: (i32, u32, u32) = (2027, 1, 1);

#[derive(Debug, Clone)]
pub struct UfSaida {
    pub sigla: Option<String>,
    pub nome: Option<String>,
    pub codigo: Option<i64>,
}

saida_json!(UfSaida {
    "sigla" => sigla: texto,
    "nome" => nome: texto,
    "codigo" => codigo: inteiro,
});

#[derive(Debug, Clone)]
pub struct MunicipioSaida {
    pub codigo: Option<i64>,
    pub nome: Option<String>,
}

saida_json!(MunicipioSaida {
    "codigo" => codigo: inteiro,
    "nome" => nome: texto,
});

#[derive(Debug, Clone)]
pub struct SituacaoSaida {
    pub id: Option<i64>,
    pub codigo: Option<String>,
    pub descricao: Option<String>,
}

saida_json!(SituacaoSaida {
    "id" => id: inteiro,
    "codigo" => codigo: texto,
    "descricao" => descricao: texto,
});

#[derive(Debug, Clone)]
pub struct TipoDfeClassificacaoSaida {
    pub tipo: Option<i32>,
    pub sigla: Option<String>,
    pub descricao: Option<String>,
}

saida_json!(TipoDfeClassificacaoSaida {
    "tipo" => tipo: inteiro,
    "sigla" => sigla: texto,
    "descricao" => descricao: texto,
});

#[derive(Debug, Clone, Default)]
pub struct ClassificacaoSaida {
    pub codigo: Option<String>,
    pub descricao: Option<String>,
    pub tipo_aliquota: Option<String>,
    pub nomenclatura: Option<String>,
    pub descricao_tratamento: Option<String>,
    pub incompativel_com_suspensao: bool,
    pub exige_grupo_desoneracao: bool,
    pub possui_percentual_reducao: bool,
    pub apropriacao_adquirente_cbs: bool,
    pub apropriacao_adquirente_ibs: bool,
    pub presumido_fornecedor: bool,
    pub presumido_adquirente: bool,
    pub operacao_antecedente: Option<String>,
    pub reducao_cbs: Option<BigDecimal>,
    pub reducao_ibs_uf: Option<BigDecimal>,
    pub reducao_ibs_mun: Option<BigDecimal>,
    pub tipo_receita_bruta_sn: Option<i64>,
    pub tipos_dfe: Option<Vec<TipoDfeClassificacaoSaida>>,
    pub data_atualizacao: Option<NaiveDate>,
}

saida_json!(ClassificacaoSaida {
    "codigo" => codigo: texto,
    "descricao" => descricao: texto,
    "tipoAliquota" => tipo_aliquota: texto,
    "nomenclatura" => nomenclatura: texto,
    "descricaoTratamentoTributario" => descricao_tratamento: texto,
    "incompativelComSuspensao" => incompativel_com_suspensao: booleano,
    "exigeGrupoDesoneracao" => exige_grupo_desoneracao: booleano,
    "possuiPercentualReducao" => possui_percentual_reducao: booleano,
    "indicaApropriacaoCreditoAdquirenteCbs" => apropriacao_adquirente_cbs: booleano,
    "indicaApropriacaoCreditoAdquirenteIbs" => apropriacao_adquirente_ibs: booleano,
    "indicaCreditoPresumidoFornecedor" => presumido_fornecedor: booleano,
    "indicaCreditoPresumidoAdquirente" => presumido_adquirente: booleano,
    "creditoOperacaoAntecedente" => operacao_antecedente: texto,
    "percentualReducaoCbs" => reducao_cbs: decimal,
    "percentualReducaoIbsUf" => reducao_ibs_uf: decimal,
    "percentualReducaoIbsMun" => reducao_ibs_mun: decimal,
    "tipoReceitaBrutaSimplesNacional" => tipo_receita_bruta_sn: inteiro,
    "tiposDfeClassificacao" => tipos_dfe: lista,
    "dataAtualizacao" => data_atualizacao: data,
});

#[derive(Debug, Clone)]
pub struct NcmSaida {
    pub tributado_is: bool,
    pub tem_ad_valorem: bool,
    pub tem_ad_rem: bool,
    pub ad_valorem: Option<BigDecimal>,
    pub ad_rem: Option<BigDecimal>,
    pub capitulo: Option<String>,
    pub posicao: Option<String>,
    pub subposicao: Option<String>,
    pub item: Option<String>,
    pub subitem: Option<String>,
    pub unidade: Option<String>,
}

saida_json!(NcmSaida {
    "tributadoPeloImpostoSeletivo" => tributado_is: booleano,
    "temAliquotaAdValorem" => tem_ad_valorem: booleano,
    "temAliquotaAdRem" => tem_ad_rem: booleano,
    "aliquotaAdValorem" => ad_valorem: decimal,
    "aliquotaAdRem" => ad_rem: decimal,
    "capitulo" => capitulo: texto,
    "posicao" => posicao: texto,
    "subposicao" => subposicao: texto,
    "item" => item: texto,
    "subitem" => subitem: texto,
    "unidade" => unidade: texto,
});

#[derive(Debug, Clone)]
pub struct NbsSaida {
    pub tributado_is: bool,
    pub tem_ad_valorem: bool,
    pub ad_valorem: Option<BigDecimal>,
    pub capitulo: Option<String>,
    pub posicao: Option<String>,
    pub subposicao1: Option<String>,
    pub subposicao2: Option<String>,
    pub item: Option<String>,
}

saida_json!(NbsSaida {
    "tributadoPeloImpostoSeletivo" => tributado_is: booleano,
    "temAliquotaAdValorem" => tem_ad_valorem: booleano,
    "aliquotaAdValorem" => ad_valorem: decimal,
    "capitulo" => capitulo: texto,
    "posicao" => posicao: texto,
    "subposicao1" => subposicao1: texto,
    "subposicao2" => subposicao2: texto,
    "item" => item: texto,
});

#[derive(Debug, Clone)]
pub struct NbsListaSaida {
    pub codigo: Option<String>,
    pub descricao: Option<String>,
}

saida_json!(NbsListaSaida {
    "codigo" => codigo: texto,
    "descricao" => descricao: texto,
});

#[derive(Debug, Clone)]
pub struct NcmVigenteSaida {
    pub ncm: Option<String>,
    pub descricao: Option<String>,
}

saida_json!(NcmVigenteSaida {
    "ncm" => ncm: texto,
    "descricao" => descricao: texto,
});

#[derive(Debug, Clone)]
pub struct FundamentacaoSaida {
    pub codigo_classificacao: Option<String>,
    pub descricao_classificacao: Option<String>,
    pub codigo_situacao: Option<String>,
    pub descricao_situacao: Option<String>,
    pub conjunto_tributo: Option<String>,
    pub texto: Option<String>,
    pub texto_curto: Option<String>,
    pub referencia_normativa: Option<String>,
}

saida_json!(FundamentacaoSaida {
    "codigoClassificacaoTributaria" => codigo_classificacao: texto,
    "descricaoClassificacaoTributaria" => descricao_classificacao: texto,
    "codigoSituacaoTributaria" => codigo_situacao: texto,
    "descricaoSituacaoTributaria" => descricao_situacao: texto,
    "conjuntoTributo" => conjunto_tributo: texto,
    "texto" => texto: texto,
    "textoCurto" => texto_curto: texto,
    "referenciaNormativa" => referencia_normativa: texto,
});

#[derive(Debug, Clone)]
pub struct AliquotaSaida {
    pub referencia: Option<BigDecimal>,
    pub propria: Option<BigDecimal>,
    pub forma_aplicacao: Option<String>,
}

saida_json!(AliquotaSaida {
    "aliquotaReferencia" => referencia: decimal,
    "aliquotaPropria" => propria: decimal,
    "formaAplicacao" => forma_aplicacao: texto,
});

#[derive(Debug, Clone)]
pub struct ValidadeDfeSaida {
    pub sigla_informada: Option<String>,
    pub valido: bool,
    pub nomenclatura: Option<String>,
    pub exige_grupo_regular: bool,
    pub permite_diferimento: bool,
    pub credito_presumido: bool,
    pub exige_grupo_ibs_cbs: bool,
}

saida_json!(ValidadeDfeSaida {
    "siglaDfeInformado" => sigla_informada: texto,
    "validoParaSiglaDfeInformado" => valido: booleano,
    "nomenclatura" => nomenclatura: texto,
    "exigeGrupoTributacaoRegular" => exige_grupo_regular: booleano,
    "permiteDiferimento" => permite_diferimento: booleano,
    "possibilidadeCreditoPresumido" => credito_presumido: booleano,
    "exigeGrupoIbsCbs" => exige_grupo_ibs_cbs: booleano,
});

#[derive(Debug, Clone)]
pub struct ClassificacaoResumoSaida {
    pub codigo: Option<String>,
    pub descricao: Option<String>,
}

saida_json!(ClassificacaoResumoSaida {
    "codigo" => codigo: texto,
    "descricao" => descricao: texto,
});

#[derive(Debug, Clone)]
pub struct CstAninhadoSaida {
    pub id: Option<i64>,
    pub codigo: Option<String>,
    pub descricao: Option<String>,
    pub classificacoes: Option<Vec<ClassificacaoResumoSaida>>,
}

saida_json!(CstAninhadoSaida {
    "id" => id: inteiro,
    "codigo" => codigo: texto,
    "descricao" => descricao: texto,
    "classificacoesTributarias" => classificacoes: lista,
});

#[derive(Debug, Clone)]
pub struct VigenciaSaida {
    pub valor: Option<BigDecimal>,
    pub inicio: Option<NaiveDate>,
    pub fim: Option<NaiveDate>,
}

saida_json!(VigenciaSaida {
    "valor" => valor: decimal,
    "inicioVigencia" => inicio: data,
    "fimVigencia" => fim: data,
});

/// `NcmAplicavelOutput` / `NbsAplicavelOutput`: só muda o nome do campo do código
#[derive(Debug, Clone)]
pub struct AplicavelSaida {
    pub nomenclatura: Nomenclatura,
    pub c_class_trib: Option<String>,
    pub codigo: Option<String>,
    pub data: Option<String>,
    pub valido: bool,
}

impl Escrever for AplicavelSaida {
    fn escrever(&self, e: &mut Escritor) -> Result<(), ErroEscrita> {
        let campo = match self.nomenclatura {
            Nomenclatura::Ncm => "ncm",
            Nomenclatura::Nbs => "nbs",
        };
        e.inicio_objeto();
        e.texto("cClassTrib", &self.c_class_trib)?;
        e.texto(campo, &self.codigo)?;
        e.texto("dataOcorrenciaFatoGerador", &self.data)?;
        e.booleano("valido", &self.valido)?;
        e.fim_objeto();
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct TipoDfeSaida {
    pub codigo: Option<String>,
    pub titulo: Option<String>,
    pub descricao: Option<String>,
    pub modelo: Option<String>,
    pub ordem: Option<usize>,
}

saida_json!(TipoDfeSaida {
    "codigo" => codigo: texto,
    "titulo" => titulo: texto,
    "descricao" => descricao: texto,
    "modelo" => modelo: texto,
    "ordem" => ordem: inteiro,
});

#[derive(Debug, Clone)]
pub struct GrupoDfeSaida {
    pub id: Option<String>,
    pub titulo: Option<String>,
    pub ordem: Option<usize>,
    pub tipos: Option<Vec<TipoDfeSaida>>,
}

saida_json!(GrupoDfeSaida {
    "id" => id: texto,
    "titulo" => titulo: texto,
    "ordem" => ordem: inteiro,
    "tipos" => tipos: lista,
});

#[derive(Debug, Clone)]
pub struct NomenclaturaSaida {
    pub sigla_dfe: Option<String>,
    pub c_class_trib: Option<String>,
    pub data: Option<NaiveDate>,
    pub nomenclatura: Option<String>,
    pub obrigatorio: bool,
    pub ncm_opcional: bool,
    pub nbs_opcional: bool,
}

saida_json!(NomenclaturaSaida {
    "siglaDfe" => sigla_dfe: texto,
    "cClassTrib" => c_class_trib: texto,
    "data" => data: data,
    "nomenclatura" => nomenclatura: texto,
    "obrigatorio" => obrigatorio: booleano,
    "ncmOpcional" => ncm_opcional: booleano,
    "nbsOpcional" => nbs_opcional: booleano,
});

#[derive(Debug, Clone)]
pub struct AtorSaida {
    pub id: Option<i64>,
    pub descricao: Option<String>,
    pub ordem: Option<i64>,
}

saida_json!(AtorSaida {
    "id" => id: inteiro,
    "descricao" => descricao: texto,
    "ordem" => ordem: inteiro,
});

#[derive(Debug, Clone)]
pub struct GrupoAtorSaida {
    pub id: Option<i64>,
    pub descricao: Option<String>,
    pub ordem: Option<i64>,
    pub atores: Option<Vec<AtorSaida>>,
}

saida_json!(GrupoAtorSaida {
    "id" => id: inteiro,
    "descricao" => descricao: texto,
    "ordem" => ordem: inteiro,
    "atores" => atores: lista,
});

#[derive(Debug, Clone)]
pub struct VersaoSaida {
    pub versao_app: Option<String>,
    pub descricao_versao_app: Option<String>,
    pub versao_db: Option<String>,
    pub descricao_versao_db: Option<String>,
    pub data_versao_db: Option<String>,
    pub ambiente: Option<String>,
}

saida_json!(VersaoSaida {
    "versaoApp" => versao_app: texto,
    "descricaoVersaoApp" => descricao_versao_app: texto,
    "versaoDb" => versao_db: texto,
    "descricaoVersaoDb" => descricao_versao_db: texto,
    "dataVersaoDb" => data_versao_db: texto,
    "ambiente" => ambiente: texto,
});

/// `PapelAtorEnum` (valor gravado em `ATCL_IN_PAPEL`)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PapelAtor {
    Fornecedor,
    Adquirente,
}

impl PapelAtor {
    /// `Enum.valueOf`: nome exato, sem tolerância a maiúsculas/minúsculas
    pub fn por_nome(nome: &str) -> Option<Self> {
        match nome {
            "FORNECEDOR" => Some(Self::Fornecedor),
            "ADQUIRENTE" => Some(Self::Adquirente),
            _ => None,
        }
    }

    fn valor_banco(self) -> &'static str {
        match self {
            Self::Fornecedor => "Fornecedor",
            Self::Adquirente => "Adquirente",
        }
    }
}

/// filtros opcionais de `consultarClassificacoesTributariasCbsIbsPorAtores`
#[derive(Debug, Clone, Copy, Default)]
pub struct FiltroAtores<'a> {
    pub fornecedor: Option<i64>,
    pub adquirente: Option<i64>,
    pub sigla_dfe: Option<&'a str>,
}

/// `getWarningDadosSimuladosPorData`: dados a partir de 2027 são simulados
pub fn dados_simulados(dia: NaiveDate) -> bool {
    let (a, m, d) = INICIO_DADOS_SIMULADOS;
    NaiveDate::from_ymd_opt(a, m, d).is_some_and(|limite| dia >= limite)
}

/// `getVersaoAplicacaoDadosAbertos`: corta o sufixo de build de snapshot
fn versao_publica(versao: &str) -> &str {
    versao
        .find(SUFIXO_SNAPSHOT)
        .map_or(versao, |i| &versao[..i])
}

/// `VRBD_DATA` vem como timestamp texto; a entidade expõe só o `LocalDate`
fn data_da_versao(texto: &str) -> Option<String> {
    let dia = texto.get(..TAM_DATA_ISO)?;
    NaiveDate::parse_from_str(dia, "%Y-%m-%d")
        .ok()
        .map(data_java)
}

/// `\d+` do `String.matches` (só dígitos ASCII)
fn so_digitos(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit())
}

/// `ValidationException`: cai no handler genérico (500)
fn validacao(mensagem: &str) -> Falha {
    Falha::sistema(mensagem)
}

fn validar_codigo(n: Nomenclatura, codigo: &str) -> Resultado<()> {
    let nome = nome_nomenclatura(n);
    if so_digitos(codigo) {
        Ok(())
    } else {
        Err(validacao(&format!(
            "O campo {nome} deve conter somente dígitos."
        )))
    }
}

fn validar_completo(n: Nomenclatura, codigo: &str) -> Resultado<()> {
    validar_codigo(n, codigo)?;
    let tamanho = tamanho_completo(n);
    if codigo.len() == tamanho {
        Ok(())
    } else {
        Err(validacao(&format!(
            "O campo {} deve ter exatamente {tamanho} dígitos.",
            nome_nomenclatura(n)
        )))
    }
}

fn nome_nomenclatura(n: Nomenclatura) -> &'static str {
    match n {
        Nomenclatura::Ncm => "NCM",
        Nomenclatura::Nbs => "NBS",
    }
}

fn tamanho_completo(n: Nomenclatura) -> usize {
    match n {
        Nomenclatura::Ncm => super::nomenclatura::TAM_NCM_COMPLETO,
        Nomenclatura::Nbs => super::nomenclatura::TAM_NBS_COMPLETO,
    }
}

fn classificacao_nao_encontrada(codigo: &str, contexto: &str, dia: NaiveDate) -> Falha {
    Falha::new(
        Classe::ClassificacaoTributariaNaoEncontrada,
        format!(
            "Classificação tributária não encontrada para código {codigo} e data {} ({contexto})",
            data_java(dia)
        ),
    )
}

fn tipo_dfe_saida(t: TipoDfe) -> TipoDfeClassificacaoSaida {
    TipoDfeClassificacaoSaida {
        tipo: t.tipo,
        sigla: Some(t.sigla),
        descricao: Some(t.descricao),
    }
}

fn classificacao_saida(
    l: LinhaClassificacao,
    tipos_dfe: Vec<TipoDfeClassificacaoSaida>,
) -> ClassificacaoSaida {
    let mut s = ClassificacaoSaida {
        codigo: l.codigo,
        descricao: l.descricao,
        tipo_aliquota: l.tipo_aliquota,
        nomenclatura: l.nomenclatura,
        descricao_tratamento: l.descricao_tratamento,
        incompativel_com_suspensao: l.incompativel_com_suspensao,
        exige_grupo_desoneracao: l.exige_grupo_desoneracao,
        possui_percentual_reducao: l.possui_percentual_reducao,
        tipos_dfe: Some(tipos_dfe),
        data_atualizacao: l.data_atualizacao,
        ..ClassificacaoSaida::default()
    };
    if let Some(c) = l.creditos {
        s.apropriacao_adquirente_cbs = c.apropriacao_adquirente_cbs;
        s.apropriacao_adquirente_ibs = c.apropriacao_adquirente_ibs;
        s.presumido_fornecedor = c.presumido_fornecedor;
        s.presumido_adquirente = c.presumido_adquirente;
        s.operacao_antecedente = c.operacao_antecedente;
        s.reducao_cbs = c.reducao_cbs;
        s.reducao_ibs_uf = c.reducao_ibs_uf;
        s.reducao_ibs_mun = c.reducao_ibs_mun;
        s.tipo_receita_bruta_sn = c.tipo_receita_bruta_sn;
    }
    s
}

fn vigencia_saida(v: Vigencia<BigDecimal>) -> VigenciaSaida {
    VigenciaSaida {
        valor: Some(v.valor),
        inicio: Some(v.inicio),
        fim: v.fim,
    }
}

fn codigo_descricao_saida(c: CodigoDescricao) -> NbsListaSaida {
    NbsListaSaida {
        codigo: c.codigo,
        descricao: c.descricao,
    }
}

/// `agruparCstsComClassificacoes`: agrupa pelo código do CST na ordem de chegada
fn agrupar_csts(linhas: Vec<CstClassificacao>) -> Vec<CstAninhadoSaida> {
    let mut grupos: Vec<CstAninhadoSaida> = Vec::new();
    for l in linhas {
        let classificacao = ClassificacaoResumoSaida {
            codigo: l.codigo_classificacao,
            descricao: l.descricao_classificacao,
        };
        let posicao = grupos
            .iter()
            .position(|g| g.codigo.as_deref() == Some(l.codigo_cst.as_str()));
        let grupo = match posicao {
            Some(i) => &mut grupos[i],
            None => {
                grupos.push(CstAninhadoSaida {
                    id: l.id_cst,
                    codigo: Some(l.codigo_cst),
                    descricao: l.descricao_cst,
                    classificacoes: Some(Vec::new()),
                });
                let ultimo = grupos.len() - 1;
                &mut grupos[ultimo]
            }
        };
        if let Some(c) = grupo.classificacoes.as_mut() {
            c.push(classificacao);
        }
    }
    grupos
}

fn agrupar_dfes(linhas: Vec<LinhaGrupoDfe>) -> Vec<GrupoDfeSaida> {
    let mut grupos: Vec<(i64, GrupoDfeSaida)> = Vec::new();
    for l in linhas {
        let i = match grupos.iter().position(|(id, _)| *id == l.grupo_id) {
            Some(i) => i,
            None => {
                grupos.push((
                    l.grupo_id,
                    GrupoDfeSaida {
                        id: Some(l.grupo_id.to_string()),
                        titulo: l.grupo_titulo,
                        ordem: None,
                        tipos: Some(Vec::new()),
                    },
                ));
                grupos.len() - 1
            }
        };
        if let Some(tipos) = grupos[i].1.tipos.as_mut() {
            tipos.push(TipoDfeSaida {
                codigo: Some(SiglaDfe::normalizar(&l.sigla)),
                titulo: l.titulo,
                descricao: None,
                modelo: l.modelo.map(|m| m.to_string()),
                ordem: Some(tipos.len() + 1),
            });
        }
    }
    grupos
        .into_iter()
        .enumerate()
        .map(|(i, (_, mut g))| {
            g.ordem = Some(i + 1);
            g
        })
        .collect()
}

fn agrupar_atores(linhas: Vec<LinhaGrupoAtor>) -> Vec<GrupoAtorSaida> {
    let mut grupos: Vec<GrupoAtorSaida> = Vec::new();
    for l in linhas {
        let i = match grupos.iter().position(|g| g.id == Some(l.grupo_id)) {
            Some(i) => i,
            None => {
                grupos.push(GrupoAtorSaida {
                    id: Some(l.grupo_id),
                    descricao: l.grupo_descricao,
                    ordem: Some(l.grupo_ordem),
                    atores: Some(Vec::new()),
                });
                grupos.len() - 1
            }
        };
        if let Some(atores) = grupos[i].atores.as_mut() {
            atores.push(AtorSaida {
                id: Some(l.ator_id),
                descricao: l.ator_descricao,
                ordem: Some(l.ator_ordem),
            });
        }
    }
    grupos
}

fn transferencia_ibs(valor: i64, inicio: NaiveDate, fim: Option<NaiveDate>) -> VigenciaSaida {
    VigenciaSaida {
        valor: Some(BigDecimal::from_i64(valor)),
        inicio: Some(inicio),
        fim,
    }
}

fn primeiro_dia(ano: i32) -> Resultado<NaiveDate> {
    NaiveDate::from_ymd_opt(ano, 1, 1).ok_or_else(|| Falha::sistema("data inválida"))
}

fn ultimo_dia(ano: i32) -> Resultado<NaiveDate> {
    NaiveDate::from_ymd_opt(ano, 12, 31).ok_or_else(|| Falha::sistema("data inválida"))
}

impl Servicos<'_> {
    pub fn ufs(&self) -> Resultado<Vec<UfSaida>> {
        Ok(self
            .db
            .ufs()?
            .into_iter()
            .map(|u| UfSaida {
                sigla: Some(u.sigla),
                nome: Some(u.nome),
                codigo: Some(u.codigo),
            })
            .collect())
    }

    pub fn municipios_da_uf(&self, sigla_uf: &str) -> Resultado<Vec<MunicipioSaida>> {
        Ok(self
            .db
            .municipios_da_uf(sigla_uf)?
            .into_iter()
            .map(|m| MunicipioSaida {
                codigo: Some(m.codigo),
                nome: Some(m.nome),
            })
            .collect())
    }

    pub fn situacoes_tributarias(
        &self,
        id_tributo: i64,
        dia: NaiveDate,
    ) -> Resultado<Vec<SituacaoSaida>> {
        Ok(self
            .db
            .situacoes_do_tributo(id_tributo, dia)?
            .into_iter()
            .map(|s| SituacaoSaida {
                id: Some(s.id),
                codigo: Some(s.codigo),
                descricao: Some(s.descricao),
            })
            .collect())
    }

    fn tipos_dfe_saida(
        &self,
        id_classificacao: i64,
        dia: NaiveDate,
    ) -> Resultado<Vec<TipoDfeClassificacaoSaida>> {
        Ok(self
            .db
            .tipos_dfe_da_classificacao(id_classificacao, dia)?
            .into_iter()
            .map(tipo_dfe_saida)
            .collect())
    }

    fn com_tipos_dfe(
        &self,
        linhas: Vec<LinhaClassificacao>,
        dia: NaiveDate,
    ) -> Resultado<Vec<ClassificacaoSaida>> {
        linhas
            .into_iter()
            .map(|l| {
                let tipos = self.tipos_dfe_saida(l.id, dia)?;
                Ok(classificacao_saida(l, tipos))
            })
            .collect()
    }

    pub fn classificacoes_da_situacao(
        &self,
        id_situacao: i64,
        dia: NaiveDate,
    ) -> Resultado<Vec<ClassificacaoSaida>> {
        let linhas = self.db.classificacoes_da_situacao(id_situacao, dia)?;
        self.com_tipos_dfe(linhas, dia)
    }

    pub fn classificacoes_do_cst(
        &self,
        cst: &str,
        siglas_tributo: &[&str],
        dia: NaiveDate,
    ) -> Resultado<Vec<ClassificacaoSaida>> {
        let linhas = self.db.classificacoes_do_cst(cst, siglas_tributo, dia)?;
        self.com_tipos_dfe(linhas, dia)
    }

    pub fn classificacoes_cbs_ibs(&self, dia: NaiveDate) -> Resultado<Vec<ClassificacaoSaida>> {
        let mut linhas = self.db.classificacoes_cbs_ibs(None, dia)?;
        // a listagem não preenche tipoReceitaBrutaSimplesNacional, só a consulta por código
        for l in &mut linhas {
            if let Some(c) = l.creditos.as_mut() {
                c.tipo_receita_bruta_sn = None;
            }
        }
        self.com_tipos_dfe(linhas, dia)
    }

    pub fn classificacoes_cbs_ibs_por_atores(
        &self,
        dia: NaiveDate,
        filtro: FiltroAtores<'_>,
    ) -> Resultado<Vec<ClassificacaoSaida>> {
        let classificacoes = self.classificacoes_cbs_ibs(dia)?;
        if filtro.fornecedor.is_none() && filtro.adquirente.is_none() && filtro.sigla_dfe.is_none()
        {
            return Ok(classificacoes);
        }
        let sigla = filtro.sigla_dfe.map(SiglaDfe::normalizar);
        let admitidos = self.db.classificacoes_por_atores(
            dia,
            filtro.fornecedor,
            filtro.adquirente,
            sigla.as_deref(),
        )?;
        Ok(classificacoes
            .into_iter()
            .filter(|c| c.codigo.as_ref().is_some_and(|cod| admitidos.contains(cod)))
            .collect())
    }

    pub fn classificacao_cbs_ibs_por_codigo(
        &self,
        codigo: &str,
        dia: NaiveDate,
    ) -> Resultado<ClassificacaoSaida> {
        let linha = self
            .db
            .classificacoes_cbs_ibs(Some(codigo), dia)?
            .into_iter()
            .next();
        let linha =
            linha.ok_or_else(|| classificacao_nao_encontrada(codigo, TRIBUTOS_CBS_IBS, dia))?;
        let tipos = self.tipos_dfe_saida(linha.id, dia)?;
        Ok(classificacao_saida(linha, tipos))
    }

    pub fn classificacao_is_por_codigo(
        &self,
        codigo: &str,
        dia: NaiveDate,
    ) -> Resultado<ClassificacaoSaida> {
        let linha = self
            .db
            .classificacoes_is(Some(codigo), dia)?
            .into_iter()
            .next();
        let linha = linha.ok_or_else(|| classificacao_nao_encontrada(codigo, TRIBUTOS_IS, dia))?;
        let tipos = self.tipos_dfe_saida(linha.id, dia)?;
        Ok(classificacao_saida(linha, tipos))
    }

    pub fn classificacoes_is(&self, dia: NaiveDate) -> Resultado<Vec<ClassificacaoSaida>> {
        let linhas = self.db.classificacoes_is(None, dia)?;
        self.com_tipos_dfe(linhas, dia)
    }

    fn descricoes(
        &self,
        n: Nomenclatura,
        codigo: &str,
        limites: &[i64],
        dia: NaiveDate,
    ) -> Resultado<Vec<Option<String>>> {
        limites
            .iter()
            .map(|&l| self.db.descricao_nomenclatura(n, codigo, l, dia))
            .collect()
    }

    pub fn consultar_ncm(&self, ncm: &str, dia: NaiveDate) -> Resultado<NcmSaida> {
        validar_codigo(Nomenclatura::Ncm, ncm)?;
        if !self.db.existe_codigo(Nomenclatura::Ncm, ncm, dia)? {
            return Err(nao_encontrada(Nomenclatura::Ncm, ncm, dia));
        }
        let ad_valorem = self.db.aliquota_ad_valorem_produto(ncm, ID_IS, dia)?;
        let ad_rem = self.db.aliquota_ad_rem_produto(ncm, ID_IS, dia)?;
        let mut d = self
            .descricoes(Nomenclatura::Ncm, ncm, &[2, 4, 6, 7, 8], dia)?
            .into_iter();
        Ok(NcmSaida {
            tributado_is: ad_valorem.is_some() || ad_rem.is_some(),
            tem_ad_valorem: ad_valorem.is_some(),
            tem_ad_rem: ad_rem.is_some(),
            ad_valorem: ad_valorem.flatten(),
            ad_rem: ad_rem.as_ref().and_then(|a| a.valor.clone()),
            capitulo: d.next().flatten(),
            posicao: d.next().flatten(),
            subposicao: d.next().flatten(),
            item: d.next().flatten(),
            subitem: d.next().flatten(),
            unidade: ad_rem.and_then(|a| a.unidade_medida),
        })
    }

    pub fn consultar_nbs(&self, nbs: &str, dia: NaiveDate) -> Resultado<NbsSaida> {
        validar_codigo(Nomenclatura::Nbs, nbs)?;
        if !self.db.existe_codigo(Nomenclatura::Nbs, nbs, dia)? {
            return Err(nao_encontrada(Nomenclatura::Nbs, nbs, dia));
        }
        let ad_valorem = self.ad_valorem_servico_linha(nbs, dia)?;
        let mut d = self
            .descricoes(Nomenclatura::Nbs, nbs, &[5, 6, 7, 8, 9], dia)?
            .into_iter();
        Ok(NbsSaida {
            tributado_is: ad_valorem.is_some(),
            tem_ad_valorem: ad_valorem.is_some(),
            ad_valorem: ad_valorem.flatten(),
            capitulo: d.next().flatten(),
            posicao: d.next().flatten(),
            subposicao1: d.next().flatten(),
            subposicao2: d.next().flatten(),
            item: d.next().flatten(),
        })
    }

    /// `buscarAliquotaAdValoremDto`: a linha existir basta para "tem alíquota"
    fn ad_valorem_servico_linha(
        &self,
        nbs: &str,
        dia: NaiveDate,
    ) -> Resultado<Option<Option<BigDecimal>>> {
        sem_multiplos(
            self.db
                .aliquota_ad_valorem_servico(Some(nbs), ID_IS, None, dia),
        )
    }

    pub fn listar_nbs(&self, dia: NaiveDate) -> Resultado<Vec<NbsListaSaida>> {
        Ok(self
            .db
            .nbs_vigentes(dia)?
            .into_iter()
            .map(codigo_descricao_saida)
            .collect())
    }

    pub fn ncms_vigentes(&self, dia: NaiveDate) -> Resultado<Vec<NcmVigenteSaida>> {
        Ok(self
            .db
            .ncms_vigentes(dia)?
            .into_iter()
            .map(|c| NcmVigenteSaida {
                ncm: c.codigo,
                descricao: c.descricao,
            })
            .collect())
    }

    pub fn fundamentacoes_legais(&self, dia: NaiveDate) -> Resultado<Vec<FundamentacaoSaida>> {
        let mut saida = Vec::new();
        for f in self.db.fundamentacoes_legais(dia)? {
            // a mensagem da exceção tem "s%" e o String.format estoura antes de montá-la
            let id_tributo = self
                .db
                .tributo_da_situacao(f.id_situacao, dia)?
                .ok_or_else(|| Falha::sistema("Flags = ' '"))?;
            if id_tributo == ID_IS {
                continue;
            }
            saida.push(FundamentacaoSaida {
                codigo_classificacao: Some(f.codigo_classificacao),
                descricao_classificacao: Some(f.descricao_classificacao),
                codigo_situacao: Some(f.codigo_situacao),
                descricao_situacao: Some(f.descricao_situacao),
                conjunto_tributo: Some(TRIBUTOS_CBS_IBS.into()),
                texto: Some(f.texto),
                texto_curto: Some(f.texto_curto),
                referencia_normativa: Some(f.referencia_normativa),
            });
        }
        Ok(saida)
    }

    pub fn aliquota(
        &self,
        id_tributo: i64,
        codigo_uf: Option<i64>,
        municipio: Option<i64>,
        dia: NaiveDate,
    ) -> Resultado<AliquotaSaida> {
        let a = self.aliquota_padrao(id_tributo, codigo_uf, municipio, dia)?;
        Ok(AliquotaSaida {
            referencia: Some(a.valor_referencia),
            propria: a.valor_padrao,
            forma_aplicacao: a.forma.map(|f| f.nome().to_string()),
        })
    }

    pub fn validade_dfe(
        &self,
        sigla: &str,
        codigo: &str,
        dia: NaiveDate,
    ) -> Resultado<ValidadeDfeSaida> {
        let v = self
            .db
            .validade_dfe(codigo, dia)?
            .ok_or_else(|| classificacao_nao_encontrada(codigo, sigla, dia))?;
        let tipos = self
            .db
            .tipos_dfe_da_classificacao(v.id_classificacao, dia)?;
        let sigla_enum = SiglaDfe::por_sigla(sigla)?;
        let ligado = |x: Option<i64>| x.is_some_and(|x| x != 0);
        Ok(ValidadeDfeSaida {
            sigla_informada: Some(sigla.to_string()),
            valido: tipos.iter().any(|t| sigla_enum.corresponde(&t.sigla)),
            nomenclatura: v.nomenclatura,
            exige_grupo_regular: ligado(v.exige_grupo_regular),
            permite_diferimento: ligado(v.permite_diferimento),
            credito_presumido: v.credito_presumido == Some(1),
            exige_grupo_ibs_cbs: v.exige_grupo_ibs_cbs == Some(1),
        })
    }

    pub fn csts_cbs_ibs(
        &self,
        sigla: Option<&str>,
        dia: NaiveDate,
    ) -> Resultado<Vec<CstAninhadoSaida>> {
        let linhas = match sigla.filter(|s| !branco(Some(s))) {
            None => self.db.csts_cbs_ibs(None, dia)?,
            Some(s) => {
                let chave = SiglaDfe::por_sigla(s)?.chave();
                self.db.csts_cbs_ibs(Some(chave), dia)?
            }
        };
        Ok(agrupar_csts(linhas))
    }

    pub fn csts_is(&self, sigla: Option<&str>, dia: NaiveDate) -> Resultado<Vec<CstAninhadoSaida>> {
        if let Some(s) = sigla.filter(|s| !branco(Some(s))) {
            let chave = SiglaDfe::por_sigla(s)?.chave();
            if !SiglaDfe::is_aplicavel_por_sigla(chave) {
                return Ok(Vec::new());
            }
        }
        Ok(agrupar_csts(self.db.csts_is(dia)?))
    }

    pub fn redutores_compra_governamental(&self) -> Resultado<Vec<VigenciaSaida>> {
        Ok(self
            .db
            .redutores_compra_gov()?
            .into_iter()
            .map(vigencia_saida)
            .collect())
    }

    pub fn transferencias_cbs(&self) -> Resultado<Vec<VigenciaSaida>> {
        Ok(self
            .db
            .transferencias_cbs()?
            .into_iter()
            .map(vigencia_saida)
            .collect())
    }

    /// tabela fixa no serviço Java (não vem da base)
    pub fn transferencias_ibs(&self) -> Resultado<Vec<VigenciaSaida>> {
        Ok(vec![
            transferencia_ibs(
                VALOR_IBS_TRANSICAO,
                primeiro_dia(ANO_INICIO_IBS)?,
                Some(ultimo_dia(ANO_INICIO_IBS)?),
            ),
            transferencia_ibs(VALOR_IBS_PLENO, primeiro_dia(ANO_IBS_PLENO)?, None),
        ])
    }

    pub fn nbs_aplicaveis(
        &self,
        c_class_trib: &str,
        dia: NaiveDate,
    ) -> Resultado<Vec<NbsListaSaida>> {
        let c = self.classificacao_cbs_ibs(Some(c_class_trib), dia)?;
        let nomenclatura = c.nomenclatura.as_deref().ok_or_else(|| {
            Falha::sistema("Cannot invoke \"String.contains(java.lang.CharSequence)\" because the return value of \"br.gov.serpro.rtc.domain.model.dto.ClassificacaoTributariaDTO.nomenclatura()\" is null")
        })?;
        if !nomenclatura.contains(MARCADOR_NBS) {
            return Ok(Vec::new());
        }
        if !self.db.existe_vinculo_nbs(c.id)? {
            return self.listar_nbs(dia);
        }
        Ok(self
            .db
            .nbs_aplicaveis(c.id, dia)?
            .into_iter()
            .map(codigo_descricao_saida)
            .collect())
    }

    pub fn classificacoes_por_nbs(&self, nbs: &str, dia: NaiveDate) -> Resultado<Vec<String>> {
        if !self.db.existe_codigo(Nomenclatura::Nbs, nbs, dia)? {
            return Err(nao_encontrada(Nomenclatura::Nbs, nbs, dia));
        }
        let mut codigos = self.db.classificacoes_vinculadas_nbs(nbs, dia)?;
        codigos.extend(self.db.classificacoes_servico_sem_vinculo_nbs(dia)?);
        Ok(codigos)
    }

    /// `validarNcmAplicavel` / `validarNbsAplicavel`: "não vinculada" vira `valido: false`
    pub fn validar_aplicavel_saida(
        &self,
        n: Nomenclatura,
        c_class_trib: &str,
        codigo: &str,
        dia: NaiveDate,
    ) -> Resultado<AplicavelSaida> {
        validar_completo(n, codigo)?;
        let c = self.classificacao_cbs_ibs(Some(c_class_trib), dia)?;
        let alvo = Alvo {
            id: c.id,
            codigo: c_class_trib,
            tributos: TRIBUTOS_CBS_IBS,
        };
        let valido = match self.validar_aplicavel(n, Some(codigo), alvo, dia) {
            Ok(()) => true,
            Err(f) if matches!(f.classe, Classe::NcmNaoVinculada | Classe::NbsNaoVinculada) => {
                false
            }
            Err(f) => return Err(f),
        };
        Ok(AplicavelSaida {
            nomenclatura: n,
            c_class_trib: Some(c_class_trib.to_string()),
            codigo: Some(codigo.to_string()),
            data: Some(data_java(dia)),
            valido,
        })
    }

    /// sem data, usa o dia corrente (`LocalDate.now()` com o fuso do container, UTC)
    pub fn grupos_dfe(&self, dia: Option<NaiveDate>) -> Resultado<Vec<GrupoDfeSaida>> {
        let dia = dia.unwrap_or_else(|| Utc::now().date_naive());
        Ok(agrupar_dfes(self.db.grupos_dfe(dia)?))
    }

    pub fn nomenclatura_dfe(
        &self,
        sigla: &str,
        c_class_trib: &str,
        dia: NaiveDate,
    ) -> Resultado<NomenclaturaSaida> {
        let (sigla_enum, nomenclatura) =
            self.determinar_nomenclatura_por_sigla(sigla, c_class_trib, dia)?;
        let sem = nomenclatura == NOMENCLATURA_SEM;
        Ok(NomenclaturaSaida {
            sigla_dfe: Some(sigla.to_string()),
            c_class_trib: Some(c_class_trib.to_string()),
            data: Some(dia),
            nomenclatura: Some(nomenclatura.to_string()),
            obrigatorio: !sem,
            ncm_opcional: sem && matches!(sigla_enum, SiglaDfe::Nfe | SiglaDfe::Nfce),
            nbs_opcional: sem && sigla_enum == SiglaDfe::Nfse,
        })
    }

    pub fn grupos_ator(
        &self,
        dia: NaiveDate,
        papel: Option<PapelAtor>,
    ) -> Resultado<Vec<GrupoAtorSaida>> {
        let linhas = self
            .db
            .grupos_ator(dia, papel.map(PapelAtor::valor_banco))?;
        Ok(agrupar_atores(linhas))
    }

    pub fn versao(&self, versao_app: &str) -> Resultado<VersaoSaida> {
        let v = self.db.ultima_versao()?.ok_or_else(|| {
            Falha::sistema("Cannot invoke \"br.gov.serpro.rtc.domain.model.entity.VersaoBaseDados.getNumeroVersao()\" because the return value of \"br.gov.serpro.rtc.domain.service.VersaoBaseDadosService.getUltimaVersao()\" is null")
        })?;
        Ok(VersaoSaida {
            versao_app: Some(versao_publica(versao_app).to_string()),
            descricao_versao_app: Some(versao_app.to_string()),
            versao_db: Some(v.numero),
            descricao_versao_db: v.descricao,
            data_versao_db: v.data.as_deref().and_then(data_da_versao),
            ambiente: Some(AMBIENTE.into()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linha_cst(cst: &str, classificacao: &str) -> CstClassificacao {
        CstClassificacao {
            id_cst: Some(1),
            codigo_cst: cst.into(),
            descricao_cst: None,
            codigo_classificacao: Some(classificacao.into()),
            descricao_classificacao: None,
        }
    }

    #[test]
    fn agrupa_csts_na_ordem_de_chegada() {
        let g = agrupar_csts(vec![
            linha_cst("200", "200001"),
            linha_cst("000", "000001"),
            linha_cst("200", "200002"),
        ]);
        assert_eq!(g.len(), 2);
        assert_eq!(g[0].codigo.as_deref(), Some("200"));
        assert_eq!(g[0].classificacoes.as_ref().map(Vec::len), Some(2));
    }

    #[test]
    fn numera_grupos_e_tipos_de_dfe() {
        let linha = |grupo, sigla: &str| LinhaGrupoDfe {
            grupo_id: grupo,
            grupo_titulo: None,
            sigla: sigla.into(),
            titulo: None,
            modelo: Some(55),
        };
        let g = agrupar_dfes(vec![linha(1, "NF-e"), linha(1, "NFC e"), linha(2, "NFSe")]);
        assert_eq!(g[1].ordem, Some(2));
        let tipos = g[0]
            .tipos
            .as_ref()
            .map(|t| (t[1].codigo.clone(), t[1].ordem));
        assert_eq!(tipos, Some((Some("NFCE".into()), Some(2))));
    }

    #[test]
    fn dados_simulados_a_partir_de_2027() {
        let d = |a, m, dd| NaiveDate::from_ymd_opt(a, m, dd).unwrap_or_default();
        assert!(!dados_simulados(d(2026, 12, 31)));
        assert!(dados_simulados(d(2027, 1, 1)));
    }

    #[test]
    fn valida_codigo_numerico() {
        assert!(validar_codigo(Nomenclatura::Ncm, "").is_err());
        assert!(validar_codigo(Nomenclatura::Ncm, "１２").is_err());
        assert!(validar_completo(Nomenclatura::Nbs, "12345678").is_err());
        assert!(validar_completo(Nomenclatura::Nbs, "123456789").is_ok());
    }

    #[test]
    fn data_da_versao_ignora_horario() {
        assert_eq!(
            data_da_versao("2026-09-30 00:00:00").as_deref(),
            Some("2026-09-30")
        );
        assert_eq!(data_da_versao("2026-09-30").as_deref(), Some("2026-09-30"));
    }

    #[test]
    fn versao_sem_snapshot() {
        assert_eq!(versao_publica("1.6.1-SNAPSHOT-abc"), "1.6.1");
        assert_eq!(versao_publica("1.6.1"), "1.6.1");
    }
}
