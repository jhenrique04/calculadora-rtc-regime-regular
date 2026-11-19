//! emulação do Hibernate Validator para as anotações usadas nos DTOs de entrada
//! (mensagens padrão em inglês, caminho no formato `itens[0].impostoSeletivo.cst`)

use crate::decimal::BigDecimal;
use crate::servico::texto::vazio_apos_trim;

use super::base_calculo::{
    BaseUnificadaEntrada, BaseUnificadaNfseEntrada, DocumentoAjusteNfse, LayoutMercadorias,
    LocacaoNfse, MercadoriasEntrada,
};
use super::entrada::{
    AliquotasNominais, AliquotasNominaisIs, ComponentesBc, CompraGov, ElementoBc,
    ImpostoSeletivoEntrada, Item, Operacao, TributacaoRegularEntrada,
};
use super::nfse::{NfseBaseCalculoEntrada, NfseIndicadorOperacaoEntrada};
use super::pedagio::{PedagioEntrada, TrechoEntrada};

const MSG_SOMENTE_DIGITOS: &str = "Informar somente dígitos";
const TAM_CST: usize = 3;
const TAM_CLASS_TRIB: usize = 6;
const TAM_C_IND_OP: usize = 6;
const TAM_UF: usize = 2;
const TAM_NBS: usize = 9;
const TAM_C_TRIB_NAC_CURTO: usize = 4;
const TAM_C_TRIB_NAC_LONGO: usize = 6;
const MAX_CODIGO: i64 = 9_999_999;
const MAX_TP_DOC: i64 = 999;
const DIGITOS_MOEDA: (u64, i64) = (13, 2);
const DIGITOS_QUANTIDADE: (u64, i64) = (11, 4);
const DIGITOS_ALIQUOTA: (u64, i64) = (3, 4);
const ALIQUOTA_MAXIMA: &str = "100.00";
const MINIMO_AD_REM: &str = "0.00";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Restricao {
    NotNull,
    NotEmpty,
    Size,
    Min,
    Max,
    Pattern,
    Digits,
    DecimalMin,
    DecimalMax,
    Positive,
    PositiveOrZero,
    NotBlank,
}

impl Restricao {
    /// nome simples da anotação (`annotationType().getSimpleName()`)
    pub fn nome(self) -> &'static str {
        match self {
            Self::NotNull => "NotNull",
            Self::NotEmpty => "NotEmpty",
            Self::Size => "Size",
            Self::Min => "Min",
            Self::Max => "Max",
            Self::Pattern => "Pattern",
            Self::Digits => "Digits",
            Self::DecimalMin => "DecimalMin",
            Self::DecimalMax => "DecimalMax",
            Self::Positive => "Positive",
            Self::PositiveOrZero => "PositiveOrZero",
            Self::NotBlank => "NotBlank",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violacao {
    pub caminho: String,
    pub mensagem: String,
    pub restricao: Restricao,
    /// `getInvalidValue().toString()`; `None` quando o valor é nulo
    pub valor: Option<String>,
}

#[derive(Default)]
struct Coletor {
    prefixo: String,
    violacoes: Vec<Violacao>,
}

impl Coletor {
    fn caminho(&self, nome: &str) -> String {
        if self.prefixo.is_empty() {
            nome.to_string()
        } else {
            format!("{}.{nome}", self.prefixo)
        }
    }

    fn violar(
        &mut self,
        nome: &str,
        restricao: Restricao,
        mensagem: String,
        valor: Option<String>,
    ) {
        let caminho = self.caminho(nome);
        self.violacoes.push(Violacao {
            caminho,
            mensagem,
            restricao,
            valor,
        });
    }

    fn com_prefixo(&mut self, prefixo: String, validar: impl FnOnce(&mut Self)) {
        let anterior = std::mem::replace(&mut self.prefixo, prefixo);
        validar(self);
        self.prefixo = anterior;
    }

    fn aninhado<T>(&mut self, nome: &str, v: &Option<T>, validar: impl FnOnce(&mut Self, &T)) {
        let Some(v) = v else { return };
        let anterior = std::mem::take(&mut self.prefixo);
        self.prefixo = if anterior.is_empty() {
            nome.to_string()
        } else {
            format!("{anterior}.{nome}")
        };
        validar(self, v);
        self.prefixo = anterior;
    }

    fn not_null<T>(&mut self, nome: &str, v: &Option<T>) -> bool {
        if v.is_none() {
            self.violar(nome, Restricao::NotNull, "must not be null".into(), None);
            return false;
        }
        true
    }

    /// `@NotNull @Pattern("\\d+") @Size(n, n)`, o trio usado em CST e cClassTrib
    fn codigo(&mut self, nome: &str, v: &Option<String>, tamanho: usize) {
        if !self.not_null(nome, v) {
            return;
        }
        let Some(s) = v else { return };
        if s.is_empty() || !s.bytes().all(|b| b.is_ascii_digit()) {
            self.violar(
                nome,
                Restricao::Pattern,
                MSG_SOMENTE_DIGITOS.into(),
                Some(s.clone()),
            );
        }
        self.tamanho(nome, v, tamanho, tamanho);
    }

    fn tamanho(&mut self, nome: &str, v: &Option<String>, min: usize, max: usize) {
        let Some(s) = v else { return };
        let n = s.encode_utf16().count();
        if n < min || n > max {
            self.violar(
                nome,
                Restricao::Size,
                format!("size must be between {min} and {max}"),
                Some(s.clone()),
            );
        }
    }

    fn intervalo(&mut self, nome: &str, v: Option<i64>, min: i64, max: i64) {
        let Some(n) = v else { return };
        if n < min {
            self.violar(
                nome,
                Restricao::Min,
                format!("must be greater than or equal to {min}"),
                Some(n.to_string()),
            );
        }
        if n > max {
            self.violar(
                nome,
                Restricao::Max,
                format!("must be less than or equal to {max}"),
                Some(n.to_string()),
            );
        }
    }

    fn positivo(&mut self, nome: &str, sinal: Option<i32>, valor: impl FnOnce() -> String) {
        if sinal.is_some_and(|s| s <= 0) {
            self.violar(
                nome,
                Restricao::Positive,
                "must be greater than 0".into(),
                Some(valor()),
            );
        }
    }

    /// `@NotBlank` com mensagem própria: nulo também viola
    fn nao_branco(&mut self, nome: &str, v: &Option<String>, mensagem: &str) {
        if vazio_apos_trim(v.as_deref()) {
            self.violar(nome, Restricao::NotBlank, mensagem.into(), v.clone());
        }
    }

    fn nao_nulo(&mut self, nome: &str, v: &Option<String>, mensagem: &str) {
        if v.is_none() {
            self.violar(nome, Restricao::NotNull, mensagem.into(), None);
        }
    }

    /// `@Pattern` de dígitos com tamanhos aceitos (nulo passa)
    fn so_digitos_de(
        &mut self,
        nome: &str,
        v: &Option<String>,
        tamanhos: &[usize],
        mensagem: &str,
    ) {
        let Some(s) = v else { return };
        let ok = tamanhos.contains(&s.len()) && s.bytes().all(|b| b.is_ascii_digit());
        if !ok {
            self.violar(nome, Restricao::Pattern, mensagem.into(), Some(s.clone()));
        }
    }

    /// trio `@NotBlank @NotNull @Pattern` dos códigos da NFS-e
    fn codigo_nfse(&mut self, nome: &str, v: &Option<String>, tamanho: usize) {
        self.nao_branco(nome, v, &format!("O campo {nome} é obrigatório"));
        self.nao_nulo(nome, v, &format!("O campo {nome} não pode ser nulo"));
        self.so_digitos_de(
            nome,
            v,
            &[tamanho],
            &format!("O campo {nome} deve conter exatamente {tamanho} dígitos numéricos"),
        );
    }

    fn positivo_ou_zero(&mut self, nome: &str, v: &Option<BigDecimal>) {
        let Some(d) = v else { return };
        if d.signum() < 0 {
            self.violar(
                nome,
                Restricao::PositiveOrZero,
                "must be greater than or equal to 0".into(),
                Some(d.to_string()),
            );
        }
    }

    /// `DigitsValidatorForNumber`: BigDecimal entra sem `stripTrailingZeros`
    fn digitos(&mut self, nome: &str, v: &Option<BigDecimal>, (inteiros, fracao): (u64, i64)) {
        let Some(d) = v else { return };
        let escala = d.scale();
        let tam_inteiro = i128::from(d.precision()) - i128::from(escala);
        let tam_fracao = escala.max(0);
        if tam_inteiro > i128::from(inteiros) || tam_fracao > fracao {
            self.violar(
                nome,
                Restricao::Digits,
                format!(
                    "numeric value out of bounds (<{inteiros} digits>.<{fracao} digits> expected)"
                ),
                Some(d.to_string()),
            );
        }
    }

    fn decimal_max(&mut self, nome: &str, v: &Option<BigDecimal>, limite: &str) {
        let (Some(d), Ok(max)) = (v, limite.parse::<BigDecimal>()) else {
            return;
        };
        if d.compare_to(&max).is_gt() {
            self.violar(
                nome,
                Restricao::DecimalMax,
                format!("must be less than or equal to {limite}"),
                Some(d.to_string()),
            );
        }
    }

    fn decimal_min(&mut self, nome: &str, v: &Option<BigDecimal>, limite: &str) {
        let (Some(d), Ok(min)) = (v, limite.parse::<BigDecimal>()) else {
            return;
        };
        if d.compare_to(&min).is_lt() {
            self.violar(
                nome,
                Restricao::DecimalMin,
                format!("must be greater than or equal to {limite}"),
                Some(d.to_string()),
            );
        }
    }

    fn moeda(&mut self, nome: &str, v: &Option<BigDecimal>) {
        self.positivo_ou_zero(nome, v);
        self.digitos(nome, v, DIGITOS_MOEDA);
    }

    fn aliquota_nominal(&mut self, nome: &str, v: &Option<BigDecimal>) {
        self.positivo_ou_zero(nome, v);
        self.digitos(nome, v, DIGITOS_ALIQUOTA);
        self.decimal_max(nome, v, ALIQUOTA_MAXIMA);
    }
}

fn validar_compra_gov(c: &mut Coletor, g: &CompraGov) {
    c.not_null("tpEnteGov", &g.tp_ente_gov);
}

fn validar_imposto_seletivo(c: &mut Coletor, is: &ImpostoSeletivoEntrada) {
    c.codigo("cst", &is.cst, TAM_CST);
    c.codigo("cClassTrib", &is.c_class_trib, TAM_CLASS_TRIB);
    c.not_null("baseCalculo", &is.base_calculo);
    c.moeda("baseCalculo", &is.base_calculo);
    c.positivo_ou_zero("quantidade", &is.quantidade);
    c.digitos("quantidade", &is.quantidade, DIGITOS_QUANTIDADE);
    c.moeda("impostoInformado", &is.imposto_informado);
}

fn validar_tributacao_regular(c: &mut Coletor, t: &TributacaoRegularEntrada) {
    c.codigo("cst", &t.cst, TAM_CST);
    c.codigo("cClassTrib", &t.c_class_trib, TAM_CLASS_TRIB);
}

fn validar_componentes(c: &mut Coletor, comp: &ComponentesBc) {
    for e in ElementoBc::TODOS {
        c.positivo_ou_zero(e.tag(), &comp.valores[e as usize]);
    }
}

fn validar_aliquotas_is(c: &mut Coletor, a: &AliquotasNominaisIs) {
    c.aliquota_nominal("adValorem", &a.ad_valorem);
    c.decimal_min("adRem", &a.ad_rem, MINIMO_AD_REM);
}

fn validar_aliquotas(c: &mut Coletor, a: &AliquotasNominais) {
    c.aliquota_nominal("cbs", &a.cbs);
    c.aliquota_nominal("ibsEstadual", &a.ibs_estadual);
    c.aliquota_nominal("ibsMunicipal", &a.ibs_municipal);
    c.aninhado("impostoSeletivo", &a.imposto_seletivo, validar_aliquotas_is);
}

fn validar_item_em(c: &mut Coletor, item: &Item) {
    if c.not_null("numero", &item.numero) {
        c.intervalo("numero", item.numero.map(i64::from), 1, MAX_CODIGO);
    }
    c.codigo("cst", &item.cst, TAM_CST);
    c.codigo("cClassTrib", &item.c_class_trib, TAM_CLASS_TRIB);
    c.moeda("baseCalculo", &item.base_calculo);
    c.digitos("quantidade", &item.quantidade, DIGITOS_QUANTIDADE);
    c.aninhado(
        "impostoSeletivo",
        &item.imposto_seletivo,
        validar_imposto_seletivo,
    );
    c.aninhado(
        "tributacaoRegular",
        &item.tributacao_regular,
        validar_tributacao_regular,
    );
    c.aninhado("gComponentesBC", &item.componentes_bc, validar_componentes);
    c.aninhado(
        "aliquotasNominais",
        &item.aliquotas_nominais,
        validar_aliquotas,
    );
}

fn validar_itens(c: &mut Coletor, itens: &Option<Vec<Option<Item>>>) {
    match itens {
        None => {
            c.violar(
                "itens",
                Restricao::NotEmpty,
                "must not be empty".into(),
                None,
            );
        }
        Some(lista) if lista.is_empty() => {
            c.violar(
                "itens",
                Restricao::NotEmpty,
                "must not be empty".into(),
                Some("[]".into()),
            );
        }
        Some(lista) => {
            // elementos nulos não são validados em cascata
            for (i, item) in lista.iter().enumerate() {
                if let Some(item) = item {
                    let anterior = std::mem::replace(&mut c.prefixo, format!("itens[{i}]"));
                    validar_item_em(c, item);
                    c.prefixo = anterior;
                }
            }
        }
    }
}

/// `validator.validate(operacao)` com cascata para `gCompraGov` e `itens`
pub fn validar_operacao(op: &Operacao) -> Vec<Violacao> {
    let mut c = Coletor::default();
    c.not_null("id", &op.id);
    c.not_null("versao", &op.versao);
    if c.not_null("municipio", &op.municipio) {
        c.intervalo("municipio", op.municipio, 0, MAX_CODIGO);
    }
    c.tamanho("uf", &op.uf, 2, 2);
    c.intervalo("tpDoc", op.tp_doc.map(i64::from), 1, MAX_TP_DOC);
    c.aninhado("gCompraGov", &op.compra_gov, validar_compra_gov);
    validar_itens(&mut c, &op.itens);
    c.violacoes
}

pub fn validar_nfse_base_calculo(e: &NfseBaseCalculoEntrada) -> Vec<Violacao> {
    let mut c = Coletor::default();
    ano_positivo(&mut c, e.ano_fato_gerador);
    c.not_null("valorServico", &e.valor_servico);
    c.positivo(
        "valorServico",
        e.valor_servico.as_ref().map(BigDecimal::signum),
        || {
            e.valor_servico
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_default()
        },
    );
    for (nome, v) in [
        ("descontoIncondicional", &e.desconto_incondicional),
        ("vCalcReeRepRes", &e.v_calc_ree_rep_res),
        ("vCalcDedRedIBSCBS", &e.v_calc_ded_red_ibs_cbs),
        ("iss", &e.iss),
        ("pis", &e.pis),
        ("cofins", &e.cofins),
    ] {
        c.positivo_ou_zero(nome, v);
    }
    c.violacoes
}

pub fn validar_nfse_indicador(e: &NfseIndicadorOperacaoEntrada) -> Vec<Violacao> {
    let mut c = Coletor::default();
    c.so_digitos_de(
        "cTribNac",
        &e.c_trib_nac,
        &[TAM_C_TRIB_NAC_CURTO, TAM_C_TRIB_NAC_LONGO],
        "O campo cTribNac deve conter exatamente 4 ou 6 dígitos numéricos",
    );
    c.codigo_nfse("cIndOp", &e.c_ind_op, TAM_C_IND_OP);
    c.codigo_nfse("cClassTrib", &e.c_class_trib, TAM_CLASS_TRIB);
    c.codigo_nfse("nbs", &e.nbs, TAM_NBS);
    if e.data_fato_gerador.is_none() {
        c.violar(
            "dataOcorrenciaFatoGerador",
            Restricao::NotNull,
            "O campo dataOcorrenciaFatoGerador é obrigatório".into(),
            None,
        );
    }
    c.violacoes
}

fn validar_trecho(c: &mut Coletor, t: &TrechoEntrada) {
    if c.not_null("numero", &t.numero) {
        c.intervalo("numero", t.numero.map(i64::from), 1, i64::MAX);
    }
    if c.not_null("municipio", &t.municipio) {
        c.intervalo("municipio", t.municipio, 0, MAX_CODIGO);
    }
    if c.not_null("uf", &t.uf) {
        c.tamanho("uf", &t.uf, TAM_UF, TAM_UF);
    }
    c.not_null("extensao", &t.extensao);
}

fn validar_trechos(c: &mut Coletor, trechos: &Option<Vec<Option<TrechoEntrada>>>) {
    match trechos {
        None => c.violar(
            "trechos",
            Restricao::NotEmpty,
            "must not be empty".into(),
            None,
        ),
        Some(lista) if lista.is_empty() => {
            c.violar(
                "trechos",
                Restricao::NotEmpty,
                "must not be empty".into(),
                Some("[]".into()),
            );
        }
        Some(lista) => {
            for (i, t) in lista.iter().enumerate() {
                if let Some(t) = t {
                    let anterior = std::mem::replace(&mut c.prefixo, format!("trechos[{i}]"));
                    validar_trecho(c, t);
                    c.prefixo = anterior;
                }
            }
        }
    }
}

pub fn validar_pedagio(e: &PedagioEntrada) -> Vec<Violacao> {
    let mut c = Coletor::default();
    c.not_null("dataHoraEmissao", &e.data_hora_emissao);
    if c.not_null("codigoMunicipioOrigem", &e.municipio_origem) {
        c.intervalo("codigoMunicipioOrigem", e.municipio_origem, 0, MAX_CODIGO);
    }
    if c.not_null("ufMunicipioOrigem", &e.uf_origem) {
        c.tamanho("ufMunicipioOrigem", &e.uf_origem, TAM_UF, TAM_UF);
    }
    if c.not_null("cst", &e.cst) {
        c.tamanho("cst", &e.cst, TAM_CST, TAM_CST);
    }
    if c.not_null("cClassTrib", &e.c_class_trib) {
        c.tamanho(
            "cClassTrib",
            &e.c_class_trib,
            TAM_CLASS_TRIB,
            TAM_CLASS_TRIB,
        );
    }
    if c.not_null("baseCalculo", &e.base_calculo) {
        c.moeda("baseCalculo", &e.base_calculo);
    }
    validar_trechos(&mut c, &e.trechos);
    c.violacoes
}

/// `@Valid BaseCalculoUnificadaInput` (componentes sob `componentes.`)
pub fn validar_base_unificada(e: &BaseUnificadaEntrada) -> Vec<Violacao> {
    let mut c = Coletor::default();
    c.not_null("anoFatoGerador", &e.ano_fato_gerador);
    c.not_null("tipoDocumento", &e.tipo_documento);
    c.not_null("natureza", &e.natureza);
    c.com_prefixo("componentes".into(), |c| {
        validar_componentes(c, &e.componentes)
    });
    c.violacoes
}

fn ano_positivo(c: &mut Coletor, ano: Option<i32>) {
    c.not_null("anoFatoGerador", &ano);
    c.positivo("anoFatoGerador", ano.map(i32::signum), || {
        ano.map(|a| a.to_string()).unwrap_or_default()
    });
}

/// `@Valid` das entradas de base de mercadorias (IS e CBS/IBS)
pub fn validar_mercadorias<L: LayoutMercadorias>(e: &MercadoriasEntrada<L>) -> Vec<Violacao> {
    let mut c = Coletor::default();
    ano_positivo(&mut c, e.ano_fato_gerador);
    for campo in L::CAMPOS {
        c.positivo_ou_zero(campo, e.valor(campo));
    }
    c.violacoes
}

fn validar_documento_nfse(c: &mut Coletor, d: &DocumentoAjusteNfse) {
    c.nao_branco("tpAjusteBC", &d.tp_ajuste_bc, "must not be blank");
    if c.not_null("vAjusteAplic", &d.v_ajuste_aplic) {
        c.positivo_ou_zero("vAjusteAplic", &d.v_ajuste_aplic);
    }
}

fn validar_locacao_nfse(c: &mut Coletor, l: &LocacaoNfse) {
    for (nome, v) in [
        ("pCopropriedade", &l.p_copropriedade),
        ("vTotOper", &l.v_tot_oper),
        ("vDescIncondTot", &l.v_desc_incond_tot),
        ("somaAjustesLocImoveis", &l.soma_ajustes_loc_imoveis),
    ] {
        if c.not_null(nome, v) {
            c.positivo_ou_zero(nome, v);
        }
    }
}

/// `@Valid BaseCalculoUnificadaNfseInput`; documentos nulos não cascateiam
pub fn validar_base_unificada_nfse(e: &BaseUnificadaNfseEntrada) -> Vec<Violacao> {
    let mut c = Coletor::default();
    c.not_null("anoFatoGerador", &e.ano_fato_gerador);
    c.not_null("opSimpNac", &e.op_simp_nac);
    c.not_null("situacao", &e.situacao);
    for (i, d) in e.documentos.iter().flatten().enumerate() {
        if let Some(d) = d {
            c.com_prefixo(format!("documentos[{i}]"), |c| validar_documento_nfse(c, d));
        }
    }
    c.aninhado("locacao", &e.locacao, validar_locacao_nfse);
    c.violacoes
}

/// `validator.validate(item)`: caminhos relativos ao item
pub fn validar_item(item: &Item) -> Vec<Violacao> {
    let mut c = Coletor::default();
    validar_item_em(&mut c, item);
    c.violacoes
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::ler_documento;

    fn violacoes(json: &str) -> Vec<(String, String, Option<String>)> {
        let op: Option<Operacao> = ler_documento(json.as_bytes()).unwrap_or_default();
        let op = op.unwrap_or_default();
        validar_operacao(&op)
            .into_iter()
            .map(|v| (v.caminho, v.mensagem, v.valor))
            .collect()
    }

    #[test]
    fn mensagens_padrao() {
        let v = violacoes(
            r#"{"municipio":-1,"uf":"ABC","tpDoc":0,"itens":[{"numero":0,"cst":"1a","cClassTrib":"000001","baseCalculo":1.234,
            "aliquotasNominais":{"cbs":100.01},"impostoSeletivo":{"cst":"000","cClassTrib":"000001"}},null]}"#,
        );
        let tem = |c: &str, m: &str| v.iter().any(|(cc, mm, _)| cc == c && mm == m);
        assert!(tem("id", "must not be null"));
        assert!(tem("municipio", "must be greater than or equal to 0"));
        assert!(tem("uf", "size must be between 2 and 2"));
        assert!(tem("tpDoc", "must be greater than or equal to 1"));
        assert!(tem("itens[0].numero", "must be greater than or equal to 1"));
        assert!(tem("itens[0].cst", "Informar somente dígitos"));
        assert!(tem("itens[0].cst", "size must be between 3 and 3"));
        assert!(tem(
            "itens[0].baseCalculo",
            "numeric value out of bounds (<13 digits>.<2 digits> expected)"
        ));
        assert!(tem(
            "itens[0].aliquotasNominais.cbs",
            "must be less than or equal to 100.00"
        ));
        assert!(tem(
            "itens[0].impostoSeletivo.baseCalculo",
            "must not be null"
        ));
        assert!(!v.iter().any(|(c, _, _)| c.starts_with("itens[1]")));
    }

    #[test]
    fn lista_vazia() {
        let v = violacoes(r#"{"id":"1","versao":"1","municipio":1,"itens":[]}"#);
        assert_eq!(
            v,
            vec![(
                "itens".into(),
                "must not be empty".into(),
                Some("[]".into())
            )]
        );
    }
}
