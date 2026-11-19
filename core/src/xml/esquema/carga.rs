//! carga dos XSDs: resolve `include`/`import` (inclusive include camaleão,
//! sem namespace alvo) e compila os componentes globais

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use super::dom::{self, No};
use super::padrao::Padrao;
use super::simples::{Espacos, TipoSimples, builtins};
use super::{
    ANY_TYPE, Chave, Compositor, Conteudo, Curinga, DeclElemento, Esquema, IdElemento, IdTipo,
    NS_XSD, Namespaces, Particula, Processamento, Seletor, TermoParticula, Tipo, TipoComplexo,
    Unico, UsoAtributo,
};

const BOM: &str = "\u{feff}";
const PREFIXO_ANONIMO: &str = "#AnonType_";
const ILIMITADO: &str = "unbounded";

#[derive(Debug, Clone, thiserror::Error)]
#[error("{0}")]
pub struct ErroEsquema(pub String);

type R<T> = Result<T, ErroEsquema>;

fn erro<T>(mensagem: impl Into<String>) -> R<T> {
    Err(ErroEsquema(mensagem.into()))
}

struct Doc {
    raiz: No,
    tns: Option<String>,
    camaleao: bool,
    elementos_qualificados: bool,
    atributos_qualificados: bool,
}

/// `arquivo` devolve o conteúdo de um caminho relativo à raiz dos XSDs
pub fn carregar(arquivo: &dyn Fn(&str) -> Option<&'static [u8]>, inicial: &str) -> R<Esquema> {
    let escopo = match inicial.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/"),
        None => String::new(),
    };
    let mut carga = Carga {
        arquivo,
        escopo,
        docs: Vec::new(),
        vistos: HashSet::new(),
    };
    carga.documento(inicial, None)?;
    let mut c = Compilador::novo(&carga.docs);
    c.compilar_globais()?;
    Ok(c.esquema)
}

struct Carga<'a> {
    arquivo: &'a dyn Fn(&str) -> Option<&'static [u8]>,
    escopo: String,
    docs: Vec<Doc>,
    vistos: HashSet<(String, Option<String>)>,
}

impl Carga<'_> {
    /// `incluidor` é o namespace alvo de quem fez o `include`
    fn documento(&mut self, caminho: &str, incluidor: Option<&Option<String>>) -> R<()> {
        let Some(bytes) = (self.arquivo)(caminho) else {
            return erro(format!(
                "Schema referenciado nao encontrado no classpath: xml/{caminho}"
            ));
        };
        let texto =
            std::str::from_utf8(bytes).map_err(|e| ErroEsquema(format!("{caminho}: {e}")))?;
        let raiz = dom::ler(texto.strip_prefix(BOM).unwrap_or(texto))
            .map_err(|e| ErroEsquema(format!("{caminho}: {e}")))?;
        if raiz.local != "schema" || raiz.uri.as_deref() != Some(NS_XSD) {
            return erro(format!("{caminho}: raiz não é xs:schema"));
        }
        let proprio = raiz.atributo("targetNamespace").map(str::to_string);
        let (tns, camaleao) = match (incluidor, proprio) {
            (None, p) => (p, false),
            (Some(alvo), None) => (alvo.clone(), alvo.is_some()),
            (Some(alvo), Some(p)) if alvo.as_deref() == Some(p.as_str()) => (Some(p), false),
            (Some(_), Some(p)) => {
                return erro(format!("{caminho}: include com namespace '{p}' diferente"));
            }
        };
        if !self.vistos.insert((caminho.to_string(), tns.clone())) {
            return Ok(());
        }
        let referencias: Vec<(String, String)> = raiz
            .filhos_xsd()
            .filter(|f| matches!(f.local.as_str(), "include" | "import" | "redefine"))
            .map(|f| {
                (
                    f.local.clone(),
                    f.atributo("schemaLocation").unwrap_or("").to_string(),
                )
            })
            .collect();
        self.docs.push(Doc {
            elementos_qualificados: raiz.atributo("elementFormDefault") == Some("qualified"),
            atributos_qualificados: raiz.atributo("attributeFormDefault") == Some("qualified"),
            raiz,
            tns: tns.clone(),
            camaleao,
        });
        for (tipo, local) in referencias {
            let destino = self.resolver(caminho, &local)?;
            match tipo.as_str() {
                "include" => self.documento(&destino, Some(&tns))?,
                "import" => self.documento(&destino, None)?,
                _ => return erro(format!("{caminho}: xs:redefine não suportado")),
            }
        }
        Ok(())
    }

    fn resolver(&self, base: &str, relativo: &str) -> R<String> {
        let dir = base.rsplit_once('/').map_or("", |(d, _)| d);
        let mut partes: Vec<&str> = dir.split('/').filter(|p| !p.is_empty()).collect();
        for p in relativo.split('/') {
            match p {
                "" | "." => {}
                ".." => {
                    partes.pop();
                }
                _ => partes.push(p),
            }
        }
        let destino = partes.join("/");
        if !destino.starts_with(&self.escopo) {
            return erro(format!(
                "Referencia de schema fora do escopo permitido: {relativo}"
            ));
        }
        Ok(destino)
    }
}

fn com_nome(ancestrais: &[String], no: &No) -> Vec<String> {
    let mut v = ancestrais.to_vec();
    v.push(no.nome().to_string());
    v
}

/// `genAnonTypeName`: nomes dos ancestrais, do mais próximo para o mais distante
fn nome_anonimo(ancestrais: &[String]) -> String {
    let mut nome = String::from(PREFIXO_ANONIMO);
    for a in ancestrais.iter().rev() {
        nome.push_str(a);
    }
    nome
}

fn numero(no: &No, atributo: &str, padrao: u32) -> R<u32> {
    match no.atributo(atributo) {
        None => Ok(padrao),
        Some(v) => v
            .trim()
            .parse()
            .map_err(|_| ErroEsquema(format!("{atributo}='{v}' inválido"))),
    }
}

fn ocorrencias(no: &No) -> R<(u32, Option<u32>)> {
    let min = numero(no, "minOccurs", 1)?;
    let max = match no.atributo("maxOccurs").map(str::trim) {
        Some(ILIMITADO) => None,
        _ => Some(numero(no, "maxOccurs", 1)?),
    };
    Ok((min, max))
}

fn tamanho(f: &No) -> R<usize> {
    let v = f.atributo("value").unwrap_or("").trim();
    v.parse()
        .map_err(|_| ErroEsquema(format!("faceta {}='{v}' inválida", f.local)))
}

fn vazia(p: &Particula) -> bool {
    match &p.termo {
        TermoParticula::Grupo(_, filhos) => filhos.iter().all(vazia),
        _ => false,
    }
}

fn conteudo(misto: bool, particula: Option<Particula>) -> Conteudo {
    let particula = particula.filter(|p| !vazia(p));
    match (misto, particula) {
        (true, p) => Conteudo::Misto(p),
        (false, None) => Conteudo::Vazio,
        (false, Some(p)) => Conteudo::Elementos(p),
    }
}

fn particula_de(c: &Conteudo) -> Option<Particula> {
    match c {
        Conteudo::Elementos(p) => Some(p.clone()),
        Conteudo::Misto(p) => p.clone(),
        Conteudo::Vazio | Conteudo::Simples(_) => None,
    }
}

fn base_builtin(nome: &str) -> &'static str {
    match nome {
        "integer" => "decimal",
        "nonNegativeInteger" | "nonPositiveInteger" | "long" => "integer",
        "positiveInteger" | "unsignedLong" => "nonNegativeInteger",
        "negativeInteger" => "nonPositiveInteger",
        "int" => "long",
        "short" => "int",
        "byte" => "short",
        "unsignedInt" => "unsignedLong",
        "unsignedShort" => "unsignedInt",
        "unsignedByte" => "unsignedShort",
        "normalizedString" => "string",
        "token" => "normalizedString",
        "language" | "Name" | "NMTOKEN" => "token",
        "NCName" => "Name",
        "ID" | "IDREF" | "ENTITY" => "NCName",
        _ => "anySimpleType",
    }
}

struct Compilador<'d> {
    docs: &'d [Doc],
    globais_elementos: HashMap<Chave, (usize, &'d No)>,
    globais_tipos: HashMap<Chave, (usize, &'d No)>,
    esquema: Esquema,
    builtins: HashMap<String, IdTipo>,
    memo_tipos: HashMap<Chave, IdTipo>,
    memo_elementos: HashMap<Chave, IdElemento>,
    em_construcao: HashSet<IdTipo>,
}

impl<'d> Compilador<'d> {
    fn novo(docs: &'d [Doc]) -> Self {
        let mut c = Self {
            docs,
            globais_elementos: HashMap::new(),
            globais_tipos: HashMap::new(),
            esquema: Esquema::default(),
            builtins: HashMap::new(),
            memo_tipos: HashMap::new(),
            memo_elementos: HashMap::new(),
            em_construcao: HashSet::new(),
        };
        c.registrar_any_type();
        c.registrar_builtins();
        c.indexar();
        c
    }

    fn registrar_any_type(&mut self) {
        let qualquer = Arc::new(Curinga {
            namespaces: Namespaces::Qualquer,
            processamento: Processamento::Lax,
        });
        let particula = Particula {
            termo: TermoParticula::Curinga(Arc::clone(&qualquer)),
            min: 0,
            max: None,
        };
        let mut t = TipoComplexo::novo("anyType".into(), Conteudo::Misto(Some(particula)));
        t.curinga_atributos = Some(qualquer);
        self.esquema.tipos.push(Tipo::Complexo(t));
        self.esquema
            .tipos_globais
            .insert((Some(NS_XSD.into()), "anyType".into()), ANY_TYPE);
    }

    fn registrar_builtins(&mut self) {
        for t in builtins() {
            let id = self.esquema.tipos.len();
            self.builtins.insert(t.nome.clone(), id);
            self.esquema
                .tipos_globais
                .insert((Some(NS_XSD.into()), t.nome.clone()), id);
            self.esquema.tipos.push(Tipo::Simples(t));
        }
        let bases: Vec<(IdTipo, Option<IdTipo>)> = self
            .builtins
            .iter()
            .map(|(nome, id)| {
                let base = if nome == "anySimpleType" {
                    Some(ANY_TYPE)
                } else {
                    self.builtins.get(base_builtin(nome)).copied()
                };
                (*id, base)
            })
            .collect();
        for (id, base) in bases {
            if let Tipo::Simples(s) = &mut self.esquema.tipos[id] {
                s.base = base;
            }
        }
    }

    fn indexar(&mut self) {
        for (d, doc) in self.docs.iter().enumerate() {
            for no in doc.raiz.filhos_xsd() {
                let chave = (doc.tns.clone(), no.nome().to_string());
                let mapa = match no.local.as_str() {
                    "element" => &mut self.globais_elementos,
                    "complexType" | "simpleType" => &mut self.globais_tipos,
                    _ => continue,
                };
                mapa.entry(chave).or_insert((d, no));
            }
        }
    }

    fn compilar_globais(&mut self) -> R<()> {
        let mut tipos: Vec<Chave> = self.globais_tipos.keys().cloned().collect();
        tipos.sort();
        for chave in tipos {
            self.tipo_global(&chave)?;
        }
        let mut elementos: Vec<Chave> = self.globais_elementos.keys().cloned().collect();
        elementos.sort();
        for chave in elementos {
            self.elemento_global(&chave)?;
        }
        self.esquema.elementos_globais = std::mem::take(&mut self.memo_elementos);
        let tipos = std::mem::take(&mut self.memo_tipos);
        self.esquema.tipos_globais.extend(tipos);
        Ok(())
    }

    fn resolver_qname(&self, d: usize, no: &No, valor: &str) -> R<Chave> {
        let valor = valor.trim();
        let (prefixo, local) = valor.split_once(':').unwrap_or(("", valor));
        let ns = no.escopo.get(prefixo).cloned();
        if !prefixo.is_empty() && ns.is_none() {
            return erro(format!("prefixo '{prefixo}' não declarado em '{valor}'"));
        }
        let doc = &self.docs[d];
        let ns = if ns.is_none() && doc.camaleao {
            doc.tns.clone()
        } else {
            ns
        };
        Ok((ns, local.to_string()))
    }

    fn tipo_por_qname(&mut self, d: usize, no: &No, valor: &str) -> R<IdTipo> {
        let chave = self.resolver_qname(d, no, valor)?;
        if chave.0.as_deref() == Some(NS_XSD) {
            if chave.1 == "anyType" {
                return Ok(ANY_TYPE);
            }
            return match self.builtins.get(&chave.1) {
                Some(id) => Ok(*id),
                None => erro(format!("tipo built-in '{}' não suportado", chave.1)),
            };
        }
        self.tipo_global(&chave)
    }

    fn reservar_tipo(&mut self) -> IdTipo {
        let id = self.esquema.tipos.len();
        let provisorio = TipoComplexo::novo(String::new(), Conteudo::Vazio);
        self.esquema.tipos.push(Tipo::Complexo(provisorio));
        self.em_construcao.insert(id);
        id
    }

    fn tipo_global(&mut self, chave: &Chave) -> R<IdTipo> {
        if let Some(id) = self.memo_tipos.get(chave) {
            return Ok(*id);
        }
        let Some(&(d, no)) = self.globais_tipos.get(chave) else {
            return erro(format!("tipo '{}' não encontrado", chave.1));
        };
        let id = self.reservar_tipo();
        self.memo_tipos.insert(chave.clone(), id);
        let nome = no.nome().to_string();
        let tipo = if no.local == "complexType" {
            Tipo::Complexo(self.complexo(d, no, nome, &[])?)
        } else {
            Tipo::Simples(self.simples(d, no, nome, &[])?)
        };
        self.esquema.tipos[id] = tipo;
        self.em_construcao.remove(&id);
        Ok(id)
    }

    fn novo_tipo(&mut self, tipo: Tipo) -> IdTipo {
        self.esquema.tipos.push(tipo);
        self.esquema.tipos.len() - 1
    }

    fn simples_base(&self, id: IdTipo) -> R<TipoSimples> {
        if self.em_construcao.contains(&id) {
            return erro("derivação circular de tipo simples");
        }
        match &self.esquema.tipos[id] {
            Tipo::Simples(s) => Ok(s.clone()),
            Tipo::Complexo(c) => erro(format!("'{}' não é tipo simples", c.nome)),
        }
    }

    /// `ancestrais` são os nomes dos ancestrais de `no` (sem o `xs:schema`)
    fn simples(
        &mut self,
        d: usize,
        no: &No,
        nome: String,
        ancestrais: &[String],
    ) -> R<TipoSimples> {
        let anc = com_nome(ancestrais, no);
        let Some(r) = no.filho("restriction") else {
            return erro(format!(
                "tipo simples '{nome}' sem restriction (list/union não suportados)"
            ));
        };
        let anc_r = com_nome(&anc, r);
        let base = match (r.atributo("base"), r.filho("simpleType")) {
            (Some(b), _) => self.tipo_por_qname(d, r, b)?,
            (None, Some(st)) => {
                let t = self.simples(d, st, nome_anonimo(&anc_r), &anc_r)?;
                self.novo_tipo(Tipo::Simples(t))
            }
            (None, None) => return erro(format!("restriction sem base em '{nome}'")),
        };
        let mut t = self.simples_base(base)?;
        t.nome = nome;
        t.base = Some(base);
        aplicar_facetas(r, &mut t)?;
        Ok(t)
    }

    fn complexo(
        &mut self,
        d: usize,
        no: &No,
        nome: String,
        ancestrais: &[String],
    ) -> R<TipoComplexo> {
        let anc = com_nome(ancestrais, no);
        let misto = no.atributo("mixed") == Some("true");
        if let Some(sc) = no.filho("simpleContent") {
            return self.conteudo_simples(d, sc, nome, &anc);
        }
        if let Some(cc) = no.filho("complexContent") {
            return self.conteudo_complexo(d, cc, nome, misto, &anc);
        }
        let particula = self.particula_do_tipo(d, no, &anc)?;
        let mut t = TipoComplexo::novo(nome, conteudo(misto, particula));
        t.base = Some(ANY_TYPE);
        self.atributos(d, no, &anc, &mut t)?;
        Ok(t)
    }

    fn derivacao(no: &No) -> R<(&No, bool)> {
        match (no.filho("extension"), no.filho("restriction")) {
            (Some(e), _) => Ok((e, true)),
            (None, Some(r)) => Ok((r, false)),
            (None, None) => erro("conteúdo sem extension/restriction"),
        }
    }

    fn base_complexa(&self, id: IdTipo) -> R<&TipoComplexo> {
        if self.em_construcao.contains(&id) {
            return erro("derivação de tipo complexo ainda em construção");
        }
        match &self.esquema.tipos[id] {
            Tipo::Complexo(c) => Ok(c),
            Tipo::Simples(s) => erro(format!("'{}' não é tipo complexo", s.nome)),
        }
    }

    fn conteudo_complexo(
        &mut self,
        d: usize,
        cc: &No,
        nome: String,
        misto_tipo: bool,
        ancestrais: &[String],
    ) -> R<TipoComplexo> {
        let misto = cc.atributo("mixed").map_or(misto_tipo, |v| v == "true");
        let anc = com_nome(ancestrais, cc);
        let (der, extensao) = Self::derivacao(cc)?;
        let base_attr = der.atributo("base").unwrap_or("");
        let base_id = self.tipo_por_qname(d, der, base_attr)?;
        let base = self.base_complexa(base_id)?;
        let (particula_base, atributos_base, curinga_base) = (
            particula_de(&base.conteudo),
            base.atributos.clone(),
            base.curinga_atributos.clone(),
        );
        let anc_d = com_nome(&anc, der);
        let propria = self.particula_do_tipo(d, der, &anc_d)?;
        let particula = if extensao {
            match (particula_base, propria) {
                (None, p) | (p, None) => p,
                (Some(b), Some(p)) => Some(Particula {
                    termo: TermoParticula::Grupo(Compositor::Sequencia, vec![b, p]),
                    min: 1,
                    max: Some(1),
                }),
            }
        } else {
            propria
        };
        let mut t = TipoComplexo::novo(nome, conteudo(misto, particula));
        t.base = Some(base_id);
        t.atributos = atributos_base;
        t.curinga_atributos = curinga_base;
        self.atributos(d, der, &anc_d, &mut t)?;
        Ok(t)
    }

    fn conteudo_simples(
        &mut self,
        d: usize,
        sc: &No,
        nome: String,
        ancestrais: &[String],
    ) -> R<TipoComplexo> {
        let anc = com_nome(ancestrais, sc);
        let (der, extensao) = Self::derivacao(sc)?;
        let base_attr = der.atributo("base").unwrap_or("");
        let base_id = self.tipo_por_qname(d, der, base_attr)?;
        let anc_d = com_nome(&anc, der);
        let (simples, atributos_base) = match &self.esquema.tipos[base_id] {
            Tipo::Simples(_) if extensao => (base_id, Vec::new()),
            Tipo::Complexo(c) => match c.conteudo {
                Conteudo::Simples(s) => (s, c.atributos.clone()),
                _ => {
                    return erro(format!(
                        "simpleContent sobre '{}' sem conteúdo simples",
                        c.nome
                    ));
                }
            },
            Tipo::Simples(s) => {
                return erro(format!("restriction de simpleContent sobre '{}'", s.nome));
            }
        };
        let simples = if extensao {
            simples
        } else {
            let mut t = self.simples_base(simples)?;
            t.nome = nome_anonimo(&anc_d);
            t.base = Some(simples);
            aplicar_facetas(der, &mut t)?;
            self.novo_tipo(Tipo::Simples(t))
        };
        let mut t = TipoComplexo::novo(nome, Conteudo::Simples(simples));
        t.base = Some(base_id);
        t.atributos = atributos_base;
        self.atributos(d, der, &anc_d, &mut t)?;
        Ok(t)
    }

    fn particula_do_tipo(
        &mut self,
        d: usize,
        no: &No,
        ancestrais: &[String],
    ) -> R<Option<Particula>> {
        let grupo = no
            .filhos_xsd()
            .find(|f| matches!(f.local.as_str(), "sequence" | "choice" | "all" | "group"));
        match grupo {
            Some(g) => self.particula(d, g, ancestrais),
            None => Ok(None),
        }
    }

    fn particula(&mut self, d: usize, no: &No, ancestrais: &[String]) -> R<Option<Particula>> {
        let (min, max) = ocorrencias(no)?;
        if max == Some(0) {
            return Ok(None);
        }
        let termo = match no.local.as_str() {
            "element" => TermoParticula::Elemento(self.elemento_local(d, no, ancestrais)?),
            "any" => TermoParticula::Curinga(Arc::new(self.curinga(d, no))),
            "sequence" | "choice" => {
                let compositor = if no.local == "sequence" {
                    Compositor::Sequencia
                } else {
                    Compositor::Escolha
                };
                let anc = com_nome(ancestrais, no);
                let mut filhos = Vec::new();
                for f in no.filhos_xsd() {
                    if let Some(p) = self.particula(d, f, &anc)? {
                        filhos.push(p);
                    }
                }
                TermoParticula::Grupo(compositor, filhos)
            }
            outro => return erro(format!("partícula xs:{outro} não suportada")),
        };
        Ok(Some(Particula { termo, min, max }))
    }

    fn curinga(&self, d: usize, no: &No) -> Curinga {
        let tns = self.docs[d].tns.clone();
        let namespaces = match no.atributo("namespace").map(str::trim) {
            None | Some("##any") => Namespaces::Qualquer,
            Some("##other") => Namespaces::Outro(tns),
            Some(lista) => Namespaces::Lista(
                lista
                    .split_whitespace()
                    .map(|n| match n {
                        "##targetNamespace" => tns.clone(),
                        "##local" => None,
                        uri => Some(uri.to_string()),
                    })
                    .collect(),
            ),
        };
        let processamento = match no.atributo("processContents") {
            Some("skip") => Processamento::Pular,
            Some("lax") => Processamento::Lax,
            _ => Processamento::Estrito,
        };
        Curinga {
            namespaces,
            processamento,
        }
    }

    fn elemento_global(&mut self, chave: &Chave) -> R<IdElemento> {
        if let Some(id) = self.memo_elementos.get(chave) {
            return Ok(*id);
        }
        let Some(&(d, no)) = self.globais_elementos.get(chave) else {
            return erro(format!("elemento '{}' não encontrado", chave.1));
        };
        let id = self.esquema.elementos.len();
        self.esquema.elementos.push(DeclElemento {
            nome: chave.1.clone(),
            ns: chave.0.clone(),
            tipo: ANY_TYPE,
            nillable: false,
            fixo: None,
            padrao: None,
            unicos: Vec::new(),
        });
        self.memo_elementos.insert(chave.clone(), id);
        let decl = self.declaracao(d, no, chave.0.clone(), &[])?;
        self.esquema.elementos[id] = decl;
        Ok(id)
    }

    fn elemento_local(&mut self, d: usize, no: &No, ancestrais: &[String]) -> R<IdElemento> {
        if let Some(r) = no.atributo("ref") {
            let chave = self.resolver_qname(d, no, r)?;
            return self.elemento_global(&chave);
        }
        let doc = &self.docs[d];
        let qualificado = no
            .atributo("form")
            .map_or(doc.elementos_qualificados, |f| f == "qualified");
        let ns = if qualificado { doc.tns.clone() } else { None };
        let decl = self.declaracao(d, no, ns, ancestrais)?;
        self.esquema.elementos.push(decl);
        Ok(self.esquema.elementos.len() - 1)
    }

    fn declaracao(
        &mut self,
        d: usize,
        no: &No,
        ns: Option<String>,
        ancestrais: &[String],
    ) -> R<DeclElemento> {
        let Some(nome) = no.atributo("name") else {
            return erro("xs:element sem name");
        };
        let anc = com_nome(ancestrais, no);
        let tipo = self.tipo_do_elemento(d, no, &anc)?;
        Ok(DeclElemento {
            nome: nome.to_string(),
            ns,
            tipo,
            nillable: no.atributo("nillable") == Some("true"),
            fixo: no.atributo("fixed").map(str::to_string),
            padrao: no.atributo("default").map(str::to_string),
            unicos: unicos(no)?,
        })
    }

    /// `ancestrais` já inclui o próprio elemento
    fn tipo_do_elemento(&mut self, d: usize, no: &No, ancestrais: &[String]) -> R<IdTipo> {
        if no.atributo("substitutionGroup").is_some() {
            return erro("substitutionGroup não suportado");
        }
        if let Some(t) = no.atributo("type") {
            return self.tipo_por_qname(d, no, t);
        }
        if let Some(ct) = no.filho("complexType") {
            let t = self.complexo(d, ct, nome_anonimo(ancestrais), ancestrais)?;
            return Ok(self.novo_tipo(Tipo::Complexo(t)));
        }
        if let Some(st) = no.filho("simpleType") {
            let t = self.simples(d, st, nome_anonimo(ancestrais), ancestrais)?;
            return Ok(self.novo_tipo(Tipo::Simples(t)));
        }
        Ok(ANY_TYPE)
    }

    fn atributos(
        &mut self,
        d: usize,
        no: &No,
        ancestrais: &[String],
        t: &mut TipoComplexo,
    ) -> R<()> {
        for f in no.filhos_xsd() {
            match f.local.as_str() {
                "attribute" => {
                    if let Some(uso) = self.atributo(d, f, ancestrais)? {
                        t.atributos
                            .retain(|a| !(a.nome == uso.nome && a.ns == uso.ns));
                        t.atributos.push(uso);
                    }
                }
                "anyAttribute" => t.curinga_atributos = Some(Arc::new(self.curinga(d, f))),
                "attributeGroup" => return erro("xs:attributeGroup não suportado"),
                _ => {}
            }
        }
        Ok(())
    }

    fn atributo(&mut self, d: usize, no: &No, ancestrais: &[String]) -> R<Option<UsoAtributo>> {
        if no.atributo("ref").is_some() {
            return erro("xs:attribute ref não suportado");
        }
        let uso = no.atributo("use").unwrap_or("optional");
        if uso == "prohibited" {
            return Ok(None);
        }
        let Some(nome) = no.atributo("name") else {
            return erro("xs:attribute sem name");
        };
        let doc = &self.docs[d];
        let qualificado = no
            .atributo("form")
            .map_or(doc.atributos_qualificados, |f| f == "qualified");
        let ns = if qualificado { doc.tns.clone() } else { None };
        let anc = com_nome(ancestrais, no);
        let tipo = match (no.atributo("type"), no.filho("simpleType")) {
            (Some(t), _) => self.tipo_por_qname(d, no, t)?,
            (None, Some(st)) => {
                let t = self.simples(d, st, nome_anonimo(&anc), &anc)?;
                self.novo_tipo(Tipo::Simples(t))
            }
            (None, None) => self
                .builtins
                .get("anySimpleType")
                .copied()
                .unwrap_or(ANY_TYPE),
        };
        Ok(Some(UsoAtributo {
            nome: nome.to_string(),
            ns,
            tipo,
            obrigatorio: uso == "required",
            fixo: no.atributo("fixed").map(str::to_string),
        }))
    }
}

fn unicos(no: &No) -> R<Vec<Unico>> {
    let mut saida = Vec::new();
    for f in no.filhos_xsd() {
        match f.local.as_str() {
            "unique" => {}
            "key" | "keyref" => return erro(format!("xs:{} não suportado", f.local)),
            _ => continue,
        }
        let seletor = match f
            .filho("selector")
            .and_then(|s| s.atributo("xpath"))
            .map(str::trim)
        {
            Some("./*" | "*") => Seletor::Filhos,
            Some("*/*" | "./*/*") => Seletor::Netos,
            outro => return erro(format!("seletor {outro:?} não suportado")),
        };
        let campos: Vec<&No> = f.filhos_xsd().filter(|c| c.local == "field").collect();
        let atributo = match campos.as_slice() {
            [c] => c.atributo("xpath").and_then(|x| x.trim().strip_prefix('@')),
            _ => None,
        };
        let Some(atributo) = atributo else {
            return erro("xs:unique só é suportado com um campo @atributo");
        };
        saida.push(Unico {
            nome: f.nome().to_string(),
            seletor,
            atributo: atributo.to_string(),
        });
    }
    Ok(saida)
}

fn aplicar_facetas(r: &No, t: &mut TipoSimples) -> R<()> {
    let mut enumeracao = Vec::new();
    let mut padroes = Vec::new();
    for f in r.filhos_xsd() {
        let valor = f.atributo("value").unwrap_or("");
        match f.local.as_str() {
            "enumeration" => enumeracao.push(valor.to_string()),
            "pattern" => padroes.push(valor.to_string()),
            "whiteSpace" => {
                t.espacos = match valor {
                    "preserve" => Espacos::Preservar,
                    "replace" => Espacos::Substituir,
                    _ => Espacos::Colapsar,
                }
            }
            "length" => t.tamanho = Some(tamanho(f)?),
            "minLength" => t.tamanho_min = Some(tamanho(f)?),
            "maxLength" => t.tamanho_max = Some(tamanho(f)?),
            "totalDigits" => t.total_digitos = Some(tamanho(f)?),
            "fractionDigits" => t.digitos_fracao = Some(tamanho(f)?),
            "minInclusive" => t.min_inclusivo = Some(valor.trim().to_string()),
            "maxInclusive" => t.max_inclusivo = Some(valor.trim().to_string()),
            "minExclusive" => t.min_exclusivo = Some(valor.trim().to_string()),
            "maxExclusive" => t.max_exclusivo = Some(valor.trim().to_string()),
            "simpleType" => {}
            outro => return erro(format!("faceta xs:{outro} não suportada")),
        }
    }
    if !padroes.is_empty() {
        let p = Padrao::novo(&padroes.join("|")).map_err(ErroEsquema)?;
        t.padroes.push(Arc::new(p));
    }
    if !enumeracao.is_empty() {
        t.enumeracao = Some(enumeracao.iter().map(|e| t.espacos.normalizar(e)).collect());
    }
    Ok(())
}
