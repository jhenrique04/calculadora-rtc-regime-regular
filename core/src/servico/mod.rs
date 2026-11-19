//! serviços de domínio da calculadora do regime regular

pub mod base_calculo;
pub mod base_calculo_nfse;
pub mod calculadora;
pub mod cbs_ibs;
pub mod consulta;
pub mod dados_abertos;
pub mod download;
pub mod imposto_seletivo;
pub mod item;
pub mod memoria;
pub mod nfse;
pub mod nomenclatura;
pub mod observabilidade;
pub mod pedagio;
pub mod texto;
pub mod tributo;
pub mod versao_offline;

use crate::db::Base;

/// acesso aos serviços sobre uma conexão emprestada do pool
pub struct Servicos<'a> {
    db: &'a Base<'a>,
}

impl<'a> Servicos<'a> {
    pub fn new(db: &'a Base<'a>) -> Self {
        Self { db }
    }
}
