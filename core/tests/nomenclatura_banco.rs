//! testes Java que chamam serviço e repositório direto, sem passar por HTTP
//! (Teste_tpdoc_3 modo rígido e Teste_tpdoc_9), contra o banco das migrações

use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use rtc_core::db::{Nomenclatura, Pool};
use rtc_core::erro::{Classe, Resultado};
use rtc_core::servico::Servicos;
use rtc_core::servico::nomenclatura::{Alvo, Modo, SiglaDfe};

const TRIBUTOS: &str = "CBS e IBS";
const CLASSIFICACAO_SEM_NOMENCLATURA: &str = "200044";
const CLASSIFICACAO_COM_EXCECAO: &str = "200003";
const ID_CLASSIFICACAO_COM_VIGENCIA_2027: i64 = 139;
const NCM_VALIDO: &str = "87100000";
const NBS_VALIDA: &str = "115012000";
const NCM_EXCEPCIONADO: &str = "02074300";
const NCM_NAO_EXCEPCIONADO: &str = "02071100";
const NCM_FORA_DO_ANEXO: &str = "24021000";

fn dia(ano: i32, mes: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(ano, mes, 1).unwrap_or_else(|| panic!("data {ano}-{mes}"))
}

struct Banco {
    dir: PathBuf,
    pool: Pool,
}

impl Banco {
    fn gerar() -> Self {
        let dir =
            std::env::temp_dir().join(format!("rtc-core-nomenclatura-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap_or_else(|e| panic!("{}: {e}", dir.display()));
        let arquivo = dir.join("calculadora-pro.db");
        let sql = Path::new(env!("CARGO_MANIFEST_DIR")).join("../flyway/sql");
        rtc_banco::migrar(&sql, &arquivo).unwrap_or_else(|e| panic!("gerar banco: {e}"));
        let pool = Pool::abrir(&arquivo).unwrap_or_else(|e| panic!("abrir banco: {e:?}"));
        Self { dir, pool }
    }
}

impl Drop for Banco {
    fn drop(&mut self) {
        if let Err(e) = std::fs::remove_dir_all(&self.dir) {
            eprintln!("remover {}: {e}", self.dir.display());
        }
    }
}

fn classe<T>(r: Resultado<T>) -> Option<Classe> {
    r.err().map(|f| f.classe)
}

#[test]
fn nomenclatura_contra_o_banco() -> Resultado<()> {
    let banco = Banco::gerar();
    let db = banco.pool.base()?;
    let s = Servicos::new(&db);

    // teste_tpdoc_3: no modo rígido a NFCom não aceita NCM/NBS sem exigência
    let data = dia(2026, 1);
    let c = s.classificacao_cbs_ibs(Some(CLASSIFICACAO_SEM_NOMENCLATURA), data)?;
    let alvo = Alvo {
        id: c.id,
        codigo: &c.codigo,
        tributos: TRIBUTOS,
    };
    let rigido =
        |ncm, nbs| s.validar_nomenclatura(SiglaDfe::Nfcom, alvo, ncm, nbs, data, Modo::Rigido);
    let nao_permitida = Some(Classe::NomenclaturaNaoPermitidaTipoDfe);
    assert_eq!(classe(rigido(Some(NCM_VALIDO), None)), nao_permitida);
    assert_eq!(classe(rigido(None, Some(NBS_VALIDA))), nao_permitida);
    assert_eq!(classe(rigido(None, None)), None);

    // teste_tpdoc_9: anexo filtrado pela vigência
    let (em_2026, em_2027) = (dia(2026, 6), dia(2027, 6));
    for n in [Nomenclatura::Ncm, Nomenclatura::Nbs] {
        assert!(!db.classificacao_tem_aplicavel(n, ID_CLASSIFICACAO_COM_VIGENCIA_2027, em_2026)?);
        assert!(db.classificacao_tem_aplicavel(n, ID_CLASSIFICACAO_COM_VIGENCIA_2027, em_2027)?);
    }

    // teste_tpdoc_9: exceção do anexo rejeita o NCM
    let c = s.classificacao_cbs_ibs(Some(CLASSIFICACAO_COM_EXCECAO), em_2026)?;
    let alvo = Alvo {
        id: c.id,
        codigo: &c.codigo,
        tributos: TRIBUTOS,
    };
    let ncm = Nomenclatura::Ncm;
    assert!(db.tem_aplicavel(ncm, NCM_EXCEPCIONADO, c.id, em_2026)?);
    assert!(db.tem_excecao_aplicavel(ncm, NCM_EXCEPCIONADO, c.id, em_2026)?);
    assert!(!db.tem_aplicavel_sem_excecao(ncm, NCM_EXCEPCIONADO, c.id, em_2026)?);
    assert!(db.tem_aplicavel(ncm, NCM_NAO_EXCEPCIONADO, c.id, em_2026)?);
    assert!(!db.tem_excecao_aplicavel(ncm, NCM_NAO_EXCEPCIONADO, c.id, em_2026)?);
    let criticar = |codigo| s.validar_aplicavel(ncm, Some(codigo), alvo, em_2026);
    let nao_vinculada = Some(Classe::NcmNaoVinculada);
    assert_eq!(classe(criticar(NCM_EXCEPCIONADO)), nao_vinculada);
    assert_eq!(classe(criticar(NCM_NAO_EXCEPCIONADO)), None);
    assert_eq!(classe(criticar(NCM_FORA_DO_ANEXO)), nao_vinculada);
    Ok(())
}
