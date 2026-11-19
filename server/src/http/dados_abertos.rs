//! `DadosAbertosController` (`/calculadora/dados-abertos`). Os argumentos são
//! resolvidos na ordem da assinatura Java: o primeiro inválido decide o erro

use chrono::NaiveDate;
use hyper::StatusCode;
use rtc_core::db::Nomenclatura;
use rtc_core::erro::Resultado;
use rtc_core::json::Escrever;
use rtc_core::servico::Servicos;
use rtc_core::servico::dados_abertos::{FiltroAtores, PapelAtor, dados_simulados};

use super::erro::ErroHttp;
use super::pedido::{Pedido, exigir_digitos};
use super::resposta::Resposta;
use super::{App, VERSAO_APLICACAO};

const CACHE_PUBLICO_UMA_HORA: &str = "max-age=3600, public";
// header montado à mão no controller, com a ordem invertida
const CACHE_PUBLICO_MANUAL: &str = "public, max-age=3600";
const CABECALHO_AVISO: &str = "x-warning-dados-simulados";
const AVISO_CASO_GERAL: &str = "1";
const TAM_NBS: usize = 9;
const TAM_CLASS_TRIB: usize = 6;
const ID_IS: i64 = 1;
const ID_CBS: i64 = 2;
const ID_IBS_UF: i64 = 3;
const ID_IBS_MUN: i64 = 4;
const TRIBUTOS_IS: [&str; 1] = ["IS"];
const TRIBUTOS_CBS_IBS: [&str; 3] = ["CBS", "IBSUF", "IBSMun"];

type Saida = Result<Resposta, ErroHttp>;

fn consultar<T: Escrever>(
    app: &App,
    f: impl FnOnce(&Servicos<'_>) -> Resultado<T>,
) -> Result<Resposta, ErroHttp> {
    let v = app.com_servicos(f)?;
    Ok(Resposta::serializar(StatusCode::OK, &v)?)
}

fn com_cache(r: Saida) -> Saida {
    Ok(r?.com("cache-control", CACHE_PUBLICO_UMA_HORA))
}

fn com_aviso(r: Saida, dia: NaiveDate) -> Saida {
    let r = r?;
    Ok(if dados_simulados(dia) {
        r.com(CABECALHO_AVISO, AVISO_CASO_GERAL)
    } else {
        r
    })
}

pub fn ufs(app: &App, _: &Pedido<'_>) -> Saida {
    com_cache(consultar(app, |s| s.ufs()))
}

pub fn municipios(app: &App, p: &Pedido<'_>) -> Saida {
    let sigla = p.obrigatorio("siglaUf")?;
    com_cache(consultar(app, |s| s.municipios_da_uf(&sigla)))
}

pub fn csts_cbs_ibs(app: &App, p: &Pedido<'_>) -> Saida {
    let sigla = p.parametro("siglaDfe");
    let dia = p.data_obrigatoria("data")?;
    com_cache(consultar(app, |s| s.csts_cbs_ibs(sigla.as_deref(), dia)))
}

pub fn csts_is(app: &App, p: &Pedido<'_>) -> Saida {
    let sigla = p.parametro("siglaDfe");
    let dia = p.data_obrigatoria("data")?;
    com_cache(consultar(app, |s| s.csts_is(sigla.as_deref(), dia)))
}

pub fn classificacoes_por_situacao(app: &App, p: &Pedido<'_>) -> Saida {
    let id = p.longo_variavel("idSituacaoTributaria")?;
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| s.classificacoes_da_situacao(id, dia))
}

pub fn classificacoes_cst_is(app: &App, p: &Pedido<'_>) -> Saida {
    let cst = p.texto_variavel("cst");
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| s.classificacoes_do_cst(&cst, &TRIBUTOS_IS, dia))
}

pub fn classificacoes_cst_cbs_ibs(app: &App, p: &Pedido<'_>) -> Saida {
    let cst = p.texto_variavel("cst");
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| {
        s.classificacoes_do_cst(&cst, &TRIBUTOS_CBS_IBS, dia)
    })
}

pub fn classificacao_cbs_ibs(app: &App, p: &Pedido<'_>) -> Saida {
    let codigo = p.texto_variavel("cClassTrib");
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| s.classificacao_cbs_ibs_por_codigo(&codigo, dia))
}

pub fn classificacao_is(app: &App, p: &Pedido<'_>) -> Saida {
    let codigo = p.texto_variavel("cClassTrib");
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| s.classificacao_is_por_codigo(&codigo, dia))
}

pub fn classificacoes_cbs_ibs(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| s.classificacoes_cbs_ibs(dia))
}

pub fn classificacoes_por_atores(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("data")?;
    let fornecedor = p.longo("fornecedor")?;
    let adquirente = p.longo("adquirente")?;
    let sigla = p.parametro("siglaDfe");
    let filtro = FiltroAtores {
        fornecedor,
        adquirente,
        sigla_dfe: sigla.as_deref(),
    };
    com_cache(consultar(app, |s| {
        s.classificacoes_cbs_ibs_por_atores(dia, filtro)
    }))
}

pub fn classificacoes_is(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("data")?;
    com_aviso(consultar(app, |s| s.classificacoes_is(dia)), dia)
}

pub fn classificacoes_por_nbs(app: &App, p: &Pedido<'_>) -> Saida {
    let nbs = p.obrigatorio("nbs")?;
    let dia = p.data_obrigatoria("data")?;
    exigir_digitos(Some(&nbs), TAM_NBS)?;
    consultar(app, |s| s.classificacoes_por_nbs(&nbs, dia))
}

pub fn situacoes_is(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| s.situacoes_tributarias(ID_IS, dia))
}

pub fn ncm(app: &App, p: &Pedido<'_>) -> Saida {
    let codigo = p.obrigatorio("ncm")?;
    let dia = p.data_obrigatoria("data")?;
    let ncm = app.com_servicos(|s| s.consultar_ncm(&codigo, dia))?;
    let r = Ok(Resposta::serializar(StatusCode::OK, &ncm)?);
    if ncm.tributado_is {
        com_aviso(r, dia)
    } else {
        r
    }
}

pub fn ncms_vigentes(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("data")?;
    com_cache(consultar(app, |s| s.ncms_vigentes(dia)))
}

pub fn nbs(app: &App, p: &Pedido<'_>) -> Saida {
    let codigo = p.obrigatorio("nbs")?;
    let dia = p.data_obrigatoria("data")?;
    let nbs = app.com_servicos(|s| s.consultar_nbs(&codigo, dia))?;
    let r = Ok(Resposta::serializar(StatusCode::OK, &nbs)?);
    if nbs.tributado_is {
        com_aviso(r, dia)
    } else {
        r
    }
}

pub fn lista_nbs(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| s.listar_nbs(dia))
}

pub fn nbs_aplicaveis(app: &App, p: &Pedido<'_>) -> Saida {
    let codigo = p.obrigatorio("cClassTrib")?;
    let dia = p.data_obrigatoria("data")?;
    exigir_digitos(Some(&codigo), TAM_CLASS_TRIB)?;
    consultar(app, |s| s.nbs_aplicaveis(&codigo, dia))
}

pub fn fundamentacoes(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| s.fundamentacoes_legais(dia))
}

pub fn aliquota_uniao(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("data")?;
    com_aviso(consultar(app, |s| s.aliquota(ID_CBS, None, None, dia)), dia)
}

pub fn aliquota_uf(app: &App, p: &Pedido<'_>) -> Saida {
    let uf = p.longo_obrigatorio("codigoUf")?;
    let dia = p.data_obrigatoria("data")?;
    com_aviso(
        consultar(app, |s| s.aliquota(ID_IBS_UF, Some(uf), None, dia)),
        dia,
    )
}

pub fn aliquota_municipio(app: &App, p: &Pedido<'_>) -> Saida {
    let municipio = p.longo_obrigatorio("codigoMunicipio")?;
    let dia = p.data_obrigatoria("data")?;
    com_aviso(
        consultar(app, |s| s.aliquota(ID_IBS_MUN, None, Some(municipio), dia)),
        dia,
    )
}

pub fn validade_dfe(app: &App, p: &Pedido<'_>) -> Saida {
    let sigla = p.texto_variavel("siglaDfe");
    let codigo = p.texto_variavel("cClassTrib");
    let dia = p.data_obrigatoria("data")?;
    consultar(app, |s| s.validade_dfe(&sigla, &codigo, dia))
}

pub fn grupos_dfe(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data("data")?;
    com_cache(consultar(app, |s| s.grupos_dfe(dia)))
}

pub fn versao(app: &App, _: &Pedido<'_>) -> Saida {
    consultar(app, |s| s.versao(VERSAO_APLICACAO))
}

pub fn redutores(app: &App, _: &Pedido<'_>) -> Saida {
    com_cache(consultar(app, |s| s.redutores_compra_governamental()))
}

pub fn transferencias_cbs(app: &App, _: &Pedido<'_>) -> Saida {
    com_cache(consultar(app, |s| s.transferencias_cbs()))
}

pub fn transferencias_ibs(app: &App, _: &Pedido<'_>) -> Saida {
    com_cache(consultar(app, |s| s.transferencias_ibs()))
}

pub fn nomenclatura(app: &App, p: &Pedido<'_>) -> Saida {
    let sigla = p.texto_variavel("siglaDfe");
    let codigo = p.texto_variavel("cClassTrib");
    let dia = p.data_obrigatoria("data")?;
    com_cache(consultar(app, |s| s.nomenclatura_dfe(&sigla, &codigo, dia)))
}

pub fn grupos_ator(app: &App, p: &Pedido<'_>) -> Saida {
    let dia = p.data_obrigatoria("data")?;
    let papel = p.enumerado("papel", PapelAtor::por_nome)?;
    com_cache(consultar(app, |s| s.grupos_ator(dia, papel)))
}

fn aplicavel(app: &App, p: &Pedido<'_>, n: Nomenclatura, nome: &'static str) -> Saida {
    let codigo_classificacao = p.obrigatorio("cClassTrib")?;
    let codigo = p.obrigatorio(nome)?;
    let dia = p.data_obrigatoria("dataOcorrenciaFatoGerador")?;
    let r = consultar(app, |s| {
        s.validar_aplicavel_saida(n, &codigo_classificacao, &codigo, dia)
    })?;
    Ok(r.com("cache-control", CACHE_PUBLICO_MANUAL))
}

pub fn nbs_aplicavel(app: &App, p: &Pedido<'_>) -> Saida {
    aplicavel(app, p, Nomenclatura::Nbs, "nbs")
}

pub fn ncm_aplicavel(app: &App, p: &Pedido<'_>) -> Saida {
    aplicavel(app, p, Nomenclatura::Ncm, "ncm")
}
