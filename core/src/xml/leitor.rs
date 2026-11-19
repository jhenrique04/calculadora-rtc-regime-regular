//! scanner XML que reproduz o Xerces do JDK: mesmas mensagens e mesma posição
//! (linha/coluna do próximo caractere a ler) em que cada erro é detectado

use std::collections::HashMap;
use std::fmt;

use super::caracteres::{eh_espaco, eh_nome, eh_nome_inicial, invalido};

pub const NS_XML: &str = "http://www.w3.org/XML/1998/namespace";
pub const NS_XMLNS: &str = "http://www.w3.org/2000/xmlns/";
const PREFIXO_XML: &str = "xml";
const PREFIXO_XMLNS: &str = "xmlns";
const TAMANHO_ALVO_RESERVADO: usize = 3;
const BASE_DECIMAL: u32 = 10;
const BASE_HEX: u32 = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Posicao {
    pub linha: u32,
    pub coluna: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErroXml {
    pub posicao: Posicao,
    pub mensagem: String,
}

impl fmt::Display for ErroXml {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Erro na linha {}, coluna {}: {}",
            self.posicao.linha, self.posicao.coluna, self.mensagem
        )
    }
}

#[derive(Debug, Clone)]
pub struct Nome {
    pub bruto: String,
    pub prefixo: Option<String>,
    pub local: String,
    pub uri: Option<String>,
}

impl Nome {
    fn separar(bruto: String) -> Self {
        // como o scanQName: só o primeiro ':' separa prefixo e parte local
        match bruto.split_once(':') {
            Some((p, l)) => Self {
                prefixo: Some(p.to_string()),
                local: l.to_string(),
                bruto,
                uri: None,
            },
            None => Self {
                prefixo: None,
                local: bruto.clone(),
                bruto,
                uri: None,
            },
        }
    }

    /// `QName.toString()` do Xerces, usado em algumas mensagens
    fn descricao(&self) -> String {
        let mut partes = Vec::new();
        if let Some(p) = &self.prefixo {
            partes.push(format!("prefix=\"{p}\""));
        }
        partes.push(format!("localpart=\"{}\"", self.local));
        partes.push(format!("rawname=\"{}\"", self.bruto));
        partes.join(",")
    }
}

#[derive(Debug, Clone)]
pub struct Atributo {
    pub nome: Nome,
    pub valor: String,
}

#[derive(Debug, Clone)]
pub struct Elemento {
    pub nome: Nome,
    pub atributos: Vec<Atributo>,
}

/// ligações prefixo → namespace em escopo; `None` desfaz a ligação
#[derive(Debug)]
pub struct Namespaces {
    ligacoes: Vec<(String, Option<String>)>,
    marcas: Vec<usize>,
}

impl Namespaces {
    fn novo() -> Self {
        Self {
            ligacoes: vec![(PREFIXO_XML.into(), Some(NS_XML.into()))],
            marcas: Vec::new(),
        }
    }

    /// prefixo vazio é o namespace padrão
    pub fn uri(&self, prefixo: &str) -> Option<&str> {
        self.ligacoes
            .iter()
            .rev()
            .find(|(p, _)| p == prefixo)
            .and_then(|(_, u)| u.as_deref())
    }

    pub fn visiveis(&self) -> HashMap<String, String> {
        let mut mapa = HashMap::new();
        for (p, u) in &self.ligacoes {
            match u {
                Some(u) => mapa.insert(p.clone(), u.clone()),
                None => mapa.remove(p),
            };
        }
        mapa
    }

    fn abrir(&mut self) {
        self.marcas.push(self.ligacoes.len());
    }

    fn fechar(&mut self) {
        if let Some(m) = self.marcas.pop() {
            self.ligacoes.truncate(m);
        }
    }

    fn no_contexto_atual(&self, prefixo: &str) -> bool {
        let inicio = self.marcas.last().copied().unwrap_or(0);
        self.ligacoes[inicio..].iter().any(|(p, _)| p == prefixo)
    }

    fn declarar(&mut self, prefixo: &str, uri: Option<String>) {
        self.ligacoes.push((prefixo.to_string(), uri));
    }
}

pub trait Manipulador {
    fn inicio(&mut self, e: &Elemento, ns: &Namespaces, pos: Posicao) -> Result<(), ErroXml>;
    fn texto(&mut self, t: &str);
    fn fim(&mut self, pos: Posicao) -> Result<(), ErroXml>;
}

pub fn analisar(texto: &str, m: &mut impl Manipulador) -> Result<(), ErroXml> {
    Leitor {
        cs: texto.chars().collect(),
        i: 0,
        linha: 1,
        coluna: 1,
        m,
        pilha: Vec::new(),
        ns: Namespaces::novo(),
        em_marcacao: false,
        viu_doctype: false,
        texto: String::new(),
    }
    .documento()
}

struct Leitor<'a, M> {
    cs: Vec<char>,
    i: usize,
    linha: u32,
    coluna: u32,
    m: &'a mut M,
    pilha: Vec<String>,
    ns: Namespaces,
    em_marcacao: bool,
    viu_doctype: bool,
    texto: String,
}

type R<T = ()> = Result<T, ErroXml>;

impl<M: Manipulador> Leitor<'_, M> {
    fn bruto(&self, k: usize) -> Option<char> {
        self.cs.get(k).copied()
    }

    /// `\r` e `\r\n` chegam como `\n`, igual à normalização do Xerces
    fn espiar(&self) -> Option<char> {
        match self.bruto(self.i) {
            Some('\r') => Some('\n'),
            c => c,
        }
    }

    fn avancar(&mut self) {
        match self.bruto(self.i) {
            None => {}
            Some('\r') => {
                self.i += 1;
                if self.bruto(self.i) == Some('\n') {
                    self.i += 1;
                }
                self.nova_linha();
            }
            Some('\n') => {
                self.i += 1;
                self.nova_linha();
            }
            Some(c) => {
                self.i += 1;
                // o Java conta colunas em unidades UTF-16
                self.coluna += u32::try_from(c.len_utf16()).unwrap_or(1);
            }
        }
    }

    fn nova_linha(&mut self) {
        self.linha += 1;
        self.coluna = 1;
    }

    fn pos(&self) -> Posicao {
        Posicao {
            linha: self.linha,
            coluna: self.coluna,
        }
    }

    fn erro<T>(&self, mensagem: impl Into<String>) -> R<T> {
        Err(ErroXml {
            posicao: self.pos(),
            mensagem: mensagem.into(),
        })
    }

    /// fim do documento no meio de marcação ou com elemento aberto
    fn fim_inesperado<T>(&self) -> R<T> {
        if self.em_marcacao || !self.pilha.is_empty() {
            self.erro("XML document structures must start and end within the same entity.")
        } else {
            self.erro("Premature end of file.")
        }
    }

    fn pular(&mut self, c: char) -> bool {
        if self.espiar() == Some(c) {
            self.avancar();
            true
        } else {
            false
        }
    }

    /// atômico como o `skipString`: em caso de falha a posição não muda
    fn pular_texto(&mut self, s: &str) -> bool {
        let casa = s
            .chars()
            .enumerate()
            .all(|(k, c)| self.bruto(self.i + k) == Some(c));
        if casa {
            for _ in s.chars() {
                self.avancar();
            }
        }
        casa
    }

    fn pular_espacos(&mut self) -> bool {
        let mut viu = false;
        while self.espiar().is_some_and(eh_espaco) {
            self.avancar();
            viu = true;
        }
        viu
    }

    fn ler_nome(&mut self) -> Option<String> {
        let primeiro = self.espiar().filter(|c| eh_nome_inicial(*c))?;
        let mut nome = String::from(primeiro);
        self.avancar();
        while let Some(c) = self.espiar().filter(|c| eh_nome(*c)) {
            nome.push(c);
            self.avancar();
        }
        Some(nome)
    }

    fn documento(&mut self) -> R {
        self.declaracao_xml()?;
        self.prologo()?;
        self.inicio_elemento()?;
        self.conteudo()?;
        self.epilogo()
    }

    fn declaracao_xml(&mut self) -> R {
        if !"<?xml"
            .chars()
            .enumerate()
            .all(|(k, c)| self.bruto(k) == Some(c))
        {
            return Ok(());
        }
        let seguinte = self.bruto(5);
        self.pular_texto("<?xml");
        self.em_marcacao = true;
        if seguinte.is_some_and(|c| eh_espaco(c) || c == '\r') {
            self.pseudo_atributos()?;
        } else {
            // `<?xmlfoo ...?>` é instrução de processamento comum
            let mut alvo = String::from(PREFIXO_XML);
            while let Some(c) = self.espiar().filter(|c| eh_nome(*c)) {
                alvo.push(c);
                self.avancar();
            }
            self.dados_pi(&alvo)?;
        }
        self.em_marcacao = false;
        Ok(())
    }

    fn pseudo_atributos(&mut self) -> R {
        let mut estado = EstadoDecl::Versao;
        let mut viu_dados = false;
        let mut espaco = self.pular_espacos();
        loop {
            match self.espiar() {
                None => return self.fim_inesperado(),
                Some('?') => break,
                Some(_) => {}
            }
            viu_dados = true;
            let (nome, valor) = self.pseudo_atributo()?;
            estado = self.avaliar_pseudo(estado, &nome, &valor, espaco)?;
            espaco = self.pular_espacos();
        }
        if !viu_dados {
            return self.erro("The version is required in the XML declaration.");
        }
        if !self.pular('?') || !self.pular('>') {
            return self.erro("The XML declaration must end with \"?>\".");
        }
        Ok(())
    }

    fn avaliar_pseudo(
        &self,
        estado: EstadoDecl,
        nome: &str,
        valor: &str,
        espaco: bool,
    ) -> R<EstadoDecl> {
        match (estado, nome) {
            (EstadoDecl::Versao, "version") => {
                if !espaco {
                    return self.erro("White space is required before the version pseudo attribute in the XML declaration.");
                }
                if valor != "1.0" && valor != "1.1" {
                    return self.erro(format!(
                        "XML version \"{valor}\" is not supported, only XML 1.0 is supported."
                    ));
                }
                Ok(EstadoDecl::Codificacao)
            }
            (EstadoDecl::Versao, _) => self.erro("The version is required in the XML declaration."),
            (EstadoDecl::Codificacao, "encoding") => {
                if !espaco {
                    return self.erro("White space is required before the encoding pseudo attribute in the XML declaration.");
                }
                Ok(EstadoDecl::Autonomo)
            }
            (EstadoDecl::Codificacao | EstadoDecl::Autonomo, "standalone") => {
                if !espaco {
                    return self.erro("White space is required before the standalone pseudo attribute in the XML declaration.");
                }
                if valor != "yes" && valor != "no" {
                    return self.erro(format!(
                        "The standalone document declaration value must be \"yes\" or \"no\", not \"{valor}\"."
                    ));
                }
                Ok(EstadoDecl::Fim)
            }
            (EstadoDecl::Codificacao, _) => {
                self.erro("The encoding declaration is required in the text declaration.")
            }
            (EstadoDecl::Autonomo, _) => {
                self.erro("The standalone name in XML declaration may be misspelled.")
            }
            (EstadoDecl::Fim, _) => self.erro("No more pseudo attributes are allowed."),
        }
    }

    fn pseudo_atributo(&mut self) -> R<(String, String)> {
        let Some(nome) = self.ler_nome() else {
            return self.erro("A pseudo attribute name is expected.");
        };
        self.pular_espacos();
        if !self.pular('=') {
            return self.erro(format!(
                "The ' = ' character must follow \"{nome}\" in the XML declaration."
            ));
        }
        self.pular_espacos();
        let aspas = match self.espiar() {
            Some(q @ ('"' | '\'')) => q,
            None => return self.fim_inesperado(),
            Some(_) => {
                return self.erro(format!(
                    "The value following \"{nome}\" in the XML declaration must be a quoted string."
                ));
            }
        };
        self.avancar();
        let mut valor = String::new();
        loop {
            match self.espiar() {
                None => return self.fim_inesperado(),
                Some(c) if c == aspas => break,
                Some(c) if invalido(c) => {
                    return self.erro(format!(
                        "An invalid XML character (Unicode: 0x{:x}) was found in the XML declaration.",
                        u32::from(c)
                    ));
                }
                Some(c) => {
                    valor.push(c);
                    self.avancar();
                }
            }
        }
        self.avancar();
        Ok((nome, valor))
    }

    fn prologo(&mut self) -> R {
        loop {
            self.pular_espacos();
            match self.espiar() {
                None => return self.fim_inesperado(),
                Some('<') => {
                    self.avancar();
                    self.em_marcacao = true;
                    if self.espiar().is_some_and(eh_nome_inicial) {
                        return Ok(());
                    }
                    self.marcacao_prologo()?;
                    self.em_marcacao = false;
                }
                Some('&') => {
                    self.avancar();
                    return self.erro("The reference is not allowed in prolog.");
                }
                Some(_) => return self.erro("Content is not allowed in prolog."),
            }
        }
    }

    fn marcacao_prologo(&mut self) -> R {
        if self.pular('?') {
            return self.pi();
        }
        if !self.pular('!') {
            return self
                .erro("The markup in the declarations of the document must be well-formed.");
        }
        if self.pular('-') {
            if !self.pular('-') {
                return self.erro("Comment must start with \"<!--\".");
            }
            return self.comentario();
        }
        if self.pular_texto("DOCTYPE") {
            if self.viu_doctype {
                return self.erro("Already seen doctype.");
            }
            self.viu_doctype = true;
            return self.doctype();
        }
        self.erro("The markup in the declarations of the document must be well-formed.")
    }

    /// A DTD não é usada na validação: só é consumida até o `>` final
    fn doctype(&mut self) -> R {
        let mut aspas: Option<char> = None;
        let mut subconjunto = false;
        loop {
            let Some(c) = self.espiar() else {
                return self.fim_inesperado();
            };
            self.avancar();
            match (aspas, c) {
                (Some(q), _) if c == q => aspas = None,
                (Some(_), _) => {}
                (None, '"' | '\'') => aspas = Some(c),
                (None, '[') => subconjunto = true,
                (None, ']') => subconjunto = false,
                (None, '>') if !subconjunto => return Ok(()),
                (None, _) => {}
            }
        }
    }

    fn comentario(&mut self) -> R {
        loop {
            match self.espiar() {
                None => return self.fim_inesperado(),
                Some('-') if self.bruto(self.i + 1) == Some('-') => {
                    self.avancar();
                    self.avancar();
                    if self.pular('>') {
                        return Ok(());
                    }
                    return self.erro("The string \"--\" is not permitted within comments.");
                }
                Some(c) if invalido(c) => {
                    return self.erro(format!(
                        "An invalid XML character (Unicode: 0x{:x}) was found in the comment.",
                        u32::from(c)
                    ));
                }
                Some(_) => self.avancar(),
            }
        }
    }

    fn pi(&mut self) -> R {
        let Some(alvo) = self.ler_nome() else {
            return self.erro("The processing instruction must begin with the name of the target.");
        };
        self.dados_pi(&alvo)
    }

    fn dados_pi(&mut self, alvo: &str) -> R {
        if alvo.chars().count() == TAMANHO_ALVO_RESERVADO && alvo.eq_ignore_ascii_case(PREFIXO_XML)
        {
            return self.erro(
                "The processing instruction target matching \"[xX][mM][lL]\" is not allowed.",
            );
        }
        if !self.pular_espacos() {
            if self.pular_texto("?>") {
                return Ok(());
            }
            return self.erro(
                "White space is required between the processing instruction target and data.",
            );
        }
        loop {
            match self.espiar() {
                None => return self.fim_inesperado(),
                Some('?') if self.bruto(self.i + 1) == Some('>') => {
                    self.avancar();
                    self.avancar();
                    return Ok(());
                }
                Some(c) if invalido(c) => {
                    return self.erro(format!(
                        "An invalid XML character (Unicode: 0x{:x}) was found in the processing instruction.",
                        u32::from(c)
                    ));
                }
                Some(_) => self.avancar(),
            }
        }
    }

    fn conteudo(&mut self) -> R {
        while !self.pilha.is_empty() {
            match self.espiar() {
                None => return self.fim_inesperado(),
                Some('<') => {
                    self.avancar();
                    self.em_marcacao = true;
                    self.marcacao_conteudo()?;
                    self.em_marcacao = false;
                }
                Some('&') => {
                    self.avancar();
                    self.referencia()?;
                }
                Some(']') => self.colchetes()?,
                Some(c) if invalido(c) => {
                    return self.erro(format!(
                        "An invalid XML character (Unicode: 0x{:x}) was found in the element content of the document.",
                        u32::from(c)
                    ));
                }
                Some(c) => {
                    self.texto.push(c);
                    self.avancar();
                }
            }
        }
        Ok(())
    }

    fn colchetes(&mut self) -> R {
        self.avancar();
        self.texto.push(']');
        if self.pular(']') {
            self.texto.push(']');
            while self.pular(']') {
                self.texto.push(']');
            }
            if self.pular('>') {
                return self.erro(
                    "The character sequence \"]]>\" must not appear in content unless used to mark the end of a CDATA section.",
                );
            }
        }
        Ok(())
    }

    fn marcacao_conteudo(&mut self) -> R {
        if self.pular('?') {
            return self.pi();
        }
        if self.pular('!') {
            if self.pular('-') {
                if !self.pular('-') {
                    return self.erro("Comment must start with \"<!--\".");
                }
                return self.comentario();
            }
            if self.pular_texto("[CDATA[") {
                return self.cdata();
            }
            return self.erro(
                "The content of elements must consist of well-formed character data or markup.",
            );
        }
        if self.pular('/') {
            return self.fim_elemento();
        }
        if self.espiar().is_some_and(eh_nome_inicial) {
            return self.inicio_elemento();
        }
        self.erro("The content of elements must consist of well-formed character data or markup.")
    }

    fn cdata(&mut self) -> R {
        loop {
            match self.espiar() {
                None => return self.fim_inesperado(),
                Some(']') if self.pular_texto("]]>") => return Ok(()),
                Some(c) if invalido(c) => {
                    return self.erro(format!(
                        "An invalid XML character (Unicode: 0x{:x}) was found in the CDATA section.",
                        u32::from(c)
                    ));
                }
                Some(c) => {
                    self.texto.push(c);
                    self.avancar();
                }
            }
        }
    }

    fn referencia(&mut self) -> R {
        let c = self.valor_referencia()?;
        self.texto.push(c);
        Ok(())
    }

    /// chamado logo depois do `&`
    fn valor_referencia(&mut self) -> R<char> {
        if self.pular('#') {
            return self.referencia_caractere();
        }
        let Some(nome) = self.ler_nome() else {
            return self
                .erro("The entity name must immediately follow the '&' in the entity reference.");
        };
        if !self.pular(';') {
            return self.erro(format!(
                "The reference to entity \"{nome}\" must end with the ';' delimiter."
            ));
        }
        match nome.as_str() {
            "lt" => Ok('<'),
            "gt" => Ok('>'),
            "amp" => Ok('&'),
            "apos" => Ok('\''),
            "quot" => Ok('"'),
            _ => self.erro(format!(
                "The entity \"{nome}\" was referenced, but not declared."
            )),
        }
    }

    fn referencia_caractere(&mut self) -> R<char> {
        let hex = self.pular('x');
        let base = if hex { BASE_HEX } else { BASE_DECIMAL };
        let mut digitos = String::new();
        while let Some(c) = self.espiar().filter(|c| c.is_digit(base)) {
            digitos.push(c);
            self.avancar();
        }
        if digitos.is_empty() {
            return if hex {
                self.erro("A hexadecimal representation must immediately follow the \"&#x\" in a character reference.")
            } else {
                self.erro("A decimal representation must immediately follow the \"&#\" in a character reference.")
            };
        }
        if !self.pular(';') {
            return self.erro("The character reference must end with the ';' delimiter.");
        }
        let valor = i32::from_str_radix(&digitos, base)
            .ok()
            .and_then(|v| u32::try_from(v).ok())
            .and_then(char::from_u32)
            .filter(|c| !invalido(*c));
        match valor {
            Some(c) => Ok(c),
            None => {
                let x = if hex { "x" } else { "" };
                self.erro(format!(
                    "Character reference \"&#{x}{digitos}\" is an invalid XML character."
                ))
            }
        }
    }

    fn entregar_texto(&mut self) {
        if !self.texto.is_empty() {
            let t = std::mem::take(&mut self.texto);
            self.m.texto(&t);
        }
    }

    /// chamado com o `<` já consumido e um caractere inicial de nome à frente
    fn inicio_elemento(&mut self) -> R {
        let bruto = self.ler_nome().unwrap_or_default();
        self.ns.abrir();
        let mut atributos = Vec::new();
        let vazio = loop {
            let espaco = self.pular_espacos();
            match self.espiar() {
                Some('>') => {
                    self.avancar();
                    break false;
                }
                Some('/') => {
                    self.avancar();
                    if !self.pular('>') {
                        return self.erro_tag_aberta(&bruto);
                    }
                    break true;
                }
                Some(c) if espaco && eh_nome_inicial(c) => {
                    let a = self.atributo(&bruto)?;
                    atributos.push(a);
                }
                None => return self.fim_inesperado(),
                Some(_) => return self.erro_tag_aberta(&bruto),
            }
        };
        let elemento = self.ligar_namespaces(bruto, atributos)?;
        self.em_marcacao = false;
        self.entregar_texto();
        let pos = self.pos();
        self.m.inicio(&elemento, &self.ns, pos)?;
        if vazio {
            self.m.fim(pos)?;
            self.ns.fechar();
        } else {
            self.pilha.push(elemento.nome.bruto);
        }
        Ok(())
    }

    fn erro_tag_aberta<T>(&self, elemento: &str) -> R<T> {
        self.erro(format!(
            "Element type \"{elemento}\" must be followed by either attribute specifications, \">\" or \"/>\"."
        ))
    }

    fn atributo(&mut self, elemento: &str) -> R<Atributo> {
        let nome = Nome::separar(self.ler_nome().unwrap_or_default());
        self.pular_espacos();
        if !self.pular('=') {
            return self.erro(format!(
                "Attribute name \"{}\" associated with an element type \"{elemento}\" must be followed by the ' = ' character.",
                nome.bruto
            ));
        }
        self.pular_espacos();
        let valor = self.valor_atributo(elemento, &nome.bruto)?;
        let declaracao = match nome.prefixo.as_deref() {
            Some(PREFIXO_XMLNS) => Some(nome.local.clone()),
            None if nome.local == PREFIXO_XMLNS => Some(String::new()),
            _ => None,
        };
        if let Some(prefixo) = declaracao {
            if !prefixo.is_empty() && valor.is_empty() {
                return self.erro(format!(
                    "The value of the attribute \"{}\" is invalid. Prefixed namespace bindings may not be empty.",
                    nome.descricao()
                ));
            }
            if self.ns.no_contexto_atual(&prefixo) {
                return self.erro(format!(
                    "Attribute \"{}\" was already specified for element \"{elemento}\".",
                    nome.bruto
                ));
            }
            self.ns
                .declarar(&prefixo, (!valor.is_empty()).then(|| valor.clone()));
        }
        Ok(Atributo { nome, valor })
    }

    fn valor_atributo(&mut self, elemento: &str, nome: &str) -> R<String> {
        let aspas = match self.espiar() {
            Some(q @ ('"' | '\'')) => q,
            None => return self.fim_inesperado(),
            Some(_) => {
                return self.erro(format!(
                    "Open quote is expected for attribute \"{nome}\" associated with an  element type  \"{elemento}\"."
                ));
            }
        };
        self.avancar();
        let mut valor = String::new();
        loop {
            match self.espiar() {
                None => return self.fim_inesperado(),
                Some(c) if c == aspas => {
                    self.avancar();
                    return Ok(valor);
                }
                Some('<') => {
                    return self.erro(format!(
                        "The value of attribute \"{nome}\" associated with an element type \"{elemento}\" must not contain the '<' character."
                    ));
                }
                Some('&') => {
                    self.avancar();
                    let c = self.valor_referencia()?;
                    valor.push(c);
                }
                Some('\t' | '\n') => {
                    valor.push(' ');
                    self.avancar();
                }
                Some(c) if invalido(c) => {
                    return self.erro(format!(
                        "An invalid XML character (Unicode: 0x{:x}) was found in the value of attribute \"{nome}\" and element is \"{elemento}\".",
                        u32::from(c)
                    ));
                }
                Some(c) => {
                    valor.push(c);
                    self.avancar();
                }
            }
        }
    }

    /// verificações que o Xerces faz só depois de ler a tag inteira
    fn ligar_namespaces(&self, bruto: String, mut atributos: Vec<Atributo>) -> R<Elemento> {
        let mut nome = Nome::separar(bruto);
        if nome.prefixo.as_deref() == Some(PREFIXO_XMLNS) {
            return self.erro(format!(
                "Element \"{}\" cannot have \"xmlns\" as its prefix.",
                nome.bruto
            ));
        }
        nome.uri = self
            .ns
            .uri(nome.prefixo.as_deref().unwrap_or(""))
            .map(str::to_string);
        if let (Some(p), None) = (&nome.prefixo, &nome.uri) {
            return self.erro(format!(
                "The prefix \"{p}\" for element \"{}\" is not bound.",
                nome.bruto
            ));
        }
        for a in &mut atributos {
            a.nome.uri = match a.nome.prefixo.as_deref() {
                None if a.nome.local == PREFIXO_XMLNS => Some(NS_XMLNS.into()),
                None => None,
                Some(PREFIXO_XMLNS) => Some(NS_XMLNS.into()),
                Some(p) => match self.ns.uri(p) {
                    Some(u) => Some(u.to_string()),
                    None => {
                        return self.erro(format!(
                            "The prefix \"{p}\" for attribute \"{}\" associated with an element type \"{}\" is not bound.",
                            a.nome.bruto, nome.bruto
                        ));
                    }
                },
            };
        }
        self.verificar_duplicados(&nome.bruto, &atributos)?;
        Ok(Elemento { nome, atributos })
    }

    fn verificar_duplicados(&self, elemento: &str, atributos: &[Atributo]) -> R {
        for (k, a) in atributos.iter().enumerate() {
            let repetido = atributos[k + 1..]
                .iter()
                .find(|b| b.nome.local == a.nome.local && b.nome.uri == a.nome.uri);
            let Some(b) = repetido else { continue };
            return match &b.nome.uri {
                Some(u) => self.erro(format!(
                    "Attribute \"{}\" bound to namespace \"{u}\" was already specified for element \"{elemento}\".",
                    b.nome.local
                )),
                None => self.erro(format!(
                    "Attribute \"{}\" was already specified for element \"{elemento}\".",
                    b.nome.bruto
                )),
            };
        }
        Ok(())
    }

    /// chamado com o `</` já consumido
    fn fim_elemento(&mut self) -> R {
        let esperado = self.pilha.last().cloned().unwrap_or_default();
        if !self.pular_texto(&esperado) {
            return self.erro(format!(
                "The element type \"{esperado}\" must be terminated by the matching end-tag \"</{esperado}>\"."
            ));
        }
        self.pular_espacos();
        if !self.pular('>') {
            return self.erro(format!(
                "The end-tag for element type \"{esperado}\" must end with a '>' delimiter."
            ));
        }
        self.em_marcacao = false;
        self.entregar_texto();
        let pos = self.pos();
        self.m.fim(pos)?;
        self.pilha.pop();
        self.ns.fechar();
        Ok(())
    }

    fn epilogo(&mut self) -> R {
        loop {
            self.pular_espacos();
            match self.espiar() {
                None => return Ok(()),
                Some('<') => {
                    self.avancar();
                    self.em_marcacao = true;
                    self.marcacao_epilogo()?;
                    self.em_marcacao = false;
                }
                Some(_) => return self.erro("Content is not allowed in trailing section."),
            }
        }
    }

    fn marcacao_epilogo(&mut self) -> R {
        if self.pular('?') {
            return self.pi();
        }
        if self.pular('!') {
            if !self.pular_texto("--") {
                return self.erro("Comment must start with \"<!--\".");
            }
            return self.comentario();
        }
        self.pular('/');
        self.erro("The markup in the document following the root element must be well-formed.")
    }
}

#[derive(Clone, Copy)]
enum EstadoDecl {
    Versao,
    Codificacao,
    Autonomo,
    Fim,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Eventos(Vec<String>);

    impl Manipulador for Eventos {
        fn inicio(&mut self, e: &Elemento, _: &Namespaces, pos: Posicao) -> Result<(), ErroXml> {
            let uri = e.nome.uri.as_deref().unwrap_or("-");
            self.0.push(format!(
                "<{}|{uri}@{}:{}",
                e.nome.local, pos.linha, pos.coluna
            ));
            Ok(())
        }
        fn texto(&mut self, t: &str) {
            self.0.push(format!("t:{t}"));
        }
        fn fim(&mut self, pos: Posicao) -> Result<(), ErroXml> {
            self.0.push(format!(">@{}:{}", pos.linha, pos.coluna));
            Ok(())
        }
    }

    fn erro(xml: &str) -> String {
        match analisar(xml, &mut Eventos::default()) {
            Ok(()) => "ok".into(),
            Err(e) => e.to_string(),
        }
    }

    #[test]
    fn eventos_com_posicoes_e_namespaces() {
        let mut ev = Eventos::default();
        let r = analisar(
            "<a xmlns='urn:x'>\r\n <b>1&amp;<![CDATA[2]]></b><c/></a>",
            &mut ev,
        );
        assert_eq!(r, Ok(()));
        assert_eq!(
            ev.0,
            [
                "<a|urn:x@1:18",
                "t:\n ",
                "<b|urn:x@2:5",
                "t:1&2",
                ">@2:28",
                "<c|urn:x@2:32",
                ">@2:32",
                ">@2:36"
            ]
        );
    }

    #[test]
    fn mensagens_de_boa_formacao() {
        assert_eq!(
            erro("   "),
            "Erro na linha 1, coluna 4: Premature end of file."
        );
        assert_eq!(
            erro("abc<a/>"),
            "Erro na linha 1, coluna 1: Content is not allowed in prolog."
        );
        assert_eq!(
            erro("<a>\n"),
            "Erro na linha 2, coluna 1: XML document structures must start and end within the same entity."
        );
        assert_eq!(
            erro("<a><b></a>"),
            "Erro na linha 1, coluna 9: The element type \"b\" must be terminated by the matching end-tag \"</b>\"."
        );
        assert_eq!(
            erro("<a x='1' x='2'/>"),
            "Erro na linha 1, coluna 17: Attribute \"x\" was already specified for element \"a\"."
        );
        assert_eq!(
            erro("<a>&#0;</a>"),
            "Erro na linha 1, coluna 8: Character reference \"&#0\" is an invalid XML character."
        );
        assert_eq!(
            erro("<?xml version=\"2.0\"?><a/>"),
            "Erro na linha 1, coluna 20: XML version \"2.0\" is not supported, only XML 1.0 is supported."
        );
        assert_eq!(
            erro("<a/>x"),
            "Erro na linha 1, coluna 5: Content is not allowed in trailing section."
        );
        assert_eq!(
            erro("<p:a/>"),
            "Erro na linha 1, coluna 7: The prefix \"p\" for element \"p:a\" is not bound."
        );
    }
}
