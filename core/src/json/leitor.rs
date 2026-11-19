//! tokenizer com o mesmo comportamento (e mensagens) do `UTF8StreamJsonParser` do Jackson
//! na configuração padrão do Spring

use super::ErroJson;

const MAX_TOKEN_ERRO: usize = 256;
const LISTA_VALORES: &str =
    "(JSON String, Number, Array, Object or token 'null', 'true' or 'false')";
const BOM: &[u8] = &[0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    InicioObjeto,
    FimObjeto,
    InicioLista,
    FimLista,
    Campo(String),
    Texto(String),
    Inteiro(String),
    Decimal(String),
    Verdadeiro,
    Falso,
    Nulo,
}

impl Token {
    /// nome do `JsonToken` usado nas mensagens do Jackson
    pub fn nome_jackson(&self) -> &'static str {
        match self {
            Self::InicioObjeto => "START_OBJECT",
            Self::FimObjeto => "END_OBJECT",
            Self::InicioLista => "START_ARRAY",
            Self::FimLista => "END_ARRAY",
            Self::Campo(_) => "FIELD_NAME",
            Self::Texto(_) => "VALUE_STRING",
            Self::Inteiro(_) => "VALUE_NUMBER_INT",
            Self::Decimal(_) => "VALUE_NUMBER_FLOAT",
            Self::Verdadeiro => "VALUE_TRUE",
            Self::Falso => "VALUE_FALSE",
            Self::Nulo => "VALUE_NULL",
        }
    }

    pub fn texto(&self) -> String {
        match self {
            Self::InicioObjeto => "{".into(),
            Self::FimObjeto => "}".into(),
            Self::InicioLista => "[".into(),
            Self::FimLista => "]".into(),
            Self::Campo(t) | Self::Texto(t) | Self::Inteiro(t) | Self::Decimal(t) => t.clone(),
            Self::Verdadeiro => "true".into(),
            Self::Falso => "false".into(),
            Self::Nulo => "null".into(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TipoContexto {
    Raiz,
    Objeto,
    Lista,
}

#[derive(Debug, Clone, Copy)]
struct Contexto {
    tipo: TipoContexto,
    indice: i64,
    linha: usize,
    coluna: usize,
    /// dentro de objeto: o próximo token é um valor (o nome já foi lido)
    espera_valor: bool,
}

impl Contexto {
    fn novo(tipo: TipoContexto, linha: usize, coluna: usize) -> Self {
        Self {
            tipo,
            indice: -1,
            linha,
            coluna,
            espera_valor: false,
        }
    }

    fn descricao(&self) -> &'static str {
        match self.tipo {
            TipoContexto::Raiz => "root",
            TipoContexto::Objeto => "Object",
            TipoContexto::Lista => "Array",
        }
    }

    fn espera_virgula(&mut self) -> bool {
        self.indice += 1;
        self.tipo != TipoContexto::Raiz && self.indice > 0
    }

    fn local(&self) -> String {
        format!(
            "[Source: REDACTED (`StreamReadFeature.INCLUDE_SOURCE_IN_LOCATION` disabled); line: {}, column: {}]",
            self.linha, self.coluna
        )
    }
}

pub struct Leitor<'a> {
    dados: &'a [u8],
    pos: usize,
    linha: usize,
    inicio_linha: usize,
    pilha: Vec<Contexto>,
    atual: Option<Token>,
    nome_atual: Option<String>,
}

type R<T> = Result<T, ErroJson>;

fn erro<T>(msg: impl Into<String>) -> R<T> {
    Err(ErroJson::new(msg))
}

fn descricao_char(c: u32) -> String {
    let ch = char::from_u32(c).unwrap_or(char::REPLACEMENT_CHARACTER);
    if c < 0x20 || (0x7F..=0x9F).contains(&c) {
        return format!("(CTRL-CHAR, code {c})");
    }
    if c > 255 {
        return format!("'{ch}' (code {c} / 0x{c:x})");
    }
    format!("'{ch}' (code {c})")
}

fn char_inesperado<T>(c: u32, comentario: &str) -> R<T> {
    erro(format!(
        "Unexpected character ({}): {comentario}",
        descricao_char(c)
    ))
}

fn char_numerico_inesperado<T>(c: u32, comentario: &str) -> R<T> {
    erro(format!(
        "Unexpected character ({}) in numeric value: {comentario}",
        descricao_char(c)
    ))
}

/// onde a string está: muda as mensagens de erro do Jackson
#[derive(Clone, Copy)]
enum Lugar {
    Nome,
    Valor,
}

impl Lugar {
    fn eof(self) -> String {
        match self {
            Self::Nome => "Unexpected end-of-input: was expecting closing '\"' for name".into(),
            Self::Valor => "Unexpected end-of-input in VALUE_STRING".into(),
        }
    }

    fn descricao(self) -> &'static str {
        match self {
            Self::Nome => "name",
            Self::Valor => "string value",
        }
    }
}

fn eh_parte_identificador(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

impl<'a> Leitor<'a> {
    pub fn novo(dados: &'a [u8]) -> Self {
        let inicio = if dados.starts_with(BOM) { BOM.len() } else { 0 };
        Self {
            dados,
            pos: inicio,
            linha: 1,
            inicio_linha: inicio,
            pilha: vec![Contexto::novo(TipoContexto::Raiz, 1, 0)],
            atual: None,
            nome_atual: None,
        }
    }

    pub fn atual(&self) -> Option<&Token> {
        self.atual.as_ref()
    }

    pub fn nome_atual(&self) -> Option<&str> {
        self.nome_atual.as_deref()
    }

    fn ctx(&mut self) -> &mut Contexto {
        let ultimo = self.pilha.len() - 1;
        &mut self.pilha[ultimo]
    }

    fn espiar(&self) -> Option<u8> {
        self.dados.get(self.pos).copied()
    }

    /// byte cru como o Jackson reporta na maioria das mensagens (sem decodificar UTF-8)
    fn byte_cru(&self) -> u32 {
        self.espiar().map_or(0, u32::from)
    }

    fn coluna(&self) -> usize {
        self.pos - self.inicio_linha
    }

    fn pular_espacos(&mut self) -> R<Option<u8>> {
        while let Some(b) = self.espiar() {
            match b {
                b' ' | b'\t' => self.pos += 1,
                b'\n' => {
                    self.pos += 1;
                    self.nova_linha();
                }
                b'\r' => {
                    self.pos += 1;
                    if self.espiar() == Some(b'\n') {
                        self.pos += 1;
                    }
                    self.nova_linha();
                }
                b'/' => {
                    return char_inesperado(
                        u32::from(b),
                        "maybe a (non-standard) comment? (not recognized as one since Feature 'ALLOW_COMMENTS' not enabled for parser)",
                    );
                }
                _ if b < 0x20 => {
                    return erro(format!(
                        "Illegal character ((CTRL-CHAR, code {b})): only regular white space (\\r, \\n, \\t) is allowed between tokens"
                    ));
                }
                _ => return Ok(Some(b)),
            }
        }
        Ok(None)
    }

    fn nova_linha(&mut self) {
        self.linha += 1;
        self.inicio_linha = self.pos;
    }

    fn eof_entre_entradas<T>(&self) -> R<T> {
        let desc = self.pilha.last().map_or("root", Contexto::descricao);
        erro(format!(
            "Unexpected end-of-input within/between {desc} entries"
        ))
    }

    fn eof_sem_fechamento<T>(&self) -> R<T> {
        match self.pilha.last() {
            Some(c) if c.tipo != TipoContexto::Raiz => {
                let marcador = if c.tipo == TipoContexto::Lista {
                    "Array"
                } else {
                    "Object"
                };
                erro(format!(
                    "Unexpected end-of-input: expected close marker for {marcador} (start marker at {})",
                    c.local()
                ))
            }
            _ => erro("No content to map due to end-of-input"),
        }
    }

    /// `JsonParser.nextToken()`; `None` no fim da entrada da raiz
    pub fn proximo(&mut self) -> R<Option<Token>> {
        let t = self.proximo_interno()?;
        self.atual = t.clone();
        Ok(t)
    }

    fn proximo_interno(&mut self) -> R<Option<Token>> {
        if self.ctx().espera_valor {
            self.ctx().espera_valor = false;
            return self.ler_valor_de_campo().map(Some);
        }
        let Some(mut b) = self.pular_espacos()? else {
            if self.pilha.len() > 1 {
                return self.eof_sem_fechamento();
            }
            return Ok(None);
        };
        if b == b']' || b == b'}' {
            self.pos += 1;
            return self.fechar(b).map(Some);
        }
        if self.ctx().espera_virgula() {
            if b != b',' {
                let desc = self.ctx().descricao();
                return char_inesperado(
                    self.byte_cru(),
                    &format!("was expecting comma to separate {desc} entries"),
                );
            }
            self.pos += 1;
            b = match self.pular_espacos()? {
                Some(b) => b,
                None => return self.eof_entre_entradas(),
            };
        }
        if self.ctx().tipo == TipoContexto::Objeto {
            return self.ler_nome_campo(b).map(Some);
        }
        self.ler_valor(b).map(Some)
    }

    fn fechar(&mut self, b: u8) -> R<Token> {
        let Some(c) = self.pilha.last().copied() else {
            return char_inesperado(u32::from(b), "expected a value");
        };
        match (b, c.tipo) {
            (b']', TipoContexto::Lista) | (b'}', TipoContexto::Objeto) => {
                self.pilha.pop();
                Ok(if b == b']' {
                    Token::FimLista
                } else {
                    Token::FimObjeto
                })
            }
            (_, TipoContexto::Lista | TipoContexto::Objeto) => {
                let (esperado, nome) = if c.tipo == TipoContexto::Lista {
                    (']', "Array")
                } else {
                    ('}', "Object")
                };
                erro(format!(
                    "Unexpected close marker '{}': expected '{esperado}' (for {nome} starting at {})",
                    char::from(b),
                    c.local()
                ))
            }
            (_, TipoContexto::Raiz) => char_inesperado(u32::from(b), "expected a value"),
        }
    }

    fn ler_nome_campo(&mut self, b: u8) -> R<Token> {
        if b != b'"' {
            return char_inesperado(
                self.char_para_erro(),
                "was expecting double-quote to start field name",
            );
        }
        self.pos += 1;
        let nome = self.ler_string(Lugar::Nome)?;
        match self.pular_espacos()? {
            Some(b':') => self.pos += 1,
            Some(_) => {
                return char_inesperado(
                    self.byte_cru(),
                    "was expecting a colon to separate field name and value",
                );
            }
            None => return self.eof_entre_entradas(),
        }
        self.ctx().espera_valor = true;
        self.nome_atual = Some(nome.clone());
        Ok(Token::Campo(nome))
    }

    fn ler_valor_de_campo(&mut self) -> R<Token> {
        match self.pular_espacos()? {
            Some(b @ (b']' | b'}')) => char_inesperado(u32::from(b), "expected a value"),
            Some(b) => self.ler_valor(b),
            None => self.eof_entre_entradas(),
        }
    }

    fn ler_valor(&mut self, b: u8) -> R<Token> {
        let (linha, coluna) = (self.linha, self.coluna() + 1);
        match b {
            b'"' => {
                self.pos += 1;
                self.ler_string(Lugar::Valor).map(Token::Texto)
            }
            b'{' => {
                self.pos += 1;
                self.pilha
                    .push(Contexto::novo(TipoContexto::Objeto, linha, coluna));
                Ok(Token::InicioObjeto)
            }
            b'[' => {
                self.pos += 1;
                self.pilha
                    .push(Contexto::novo(TipoContexto::Lista, linha, coluna));
                Ok(Token::InicioLista)
            }
            b't' => self.literal("true", Token::Verdadeiro),
            b'f' => self.literal("false", Token::Falso),
            b'n' => self.literal("null", Token::Nulo),
            b'-' | b'0'..=b'9' => self.ler_numero(),
            b'+' => {
                self.pos += 1;
                match self.espiar() {
                    Some(b'I') => self.infinito(false),
                    _ => char_numerico_inesperado(
                        u32::from(b'+'),
                        "JSON spec does not allow numbers to have plus signs: enable `JsonReadFeature.ALLOW_LEADING_PLUS_SIGN_FOR_NUMBERS` to allow",
                    ),
                }
            }
            b']' | b'}' => char_inesperado(u32::from(b), "expected a value"),
            b',' if self.pilha.len() > 1 => char_inesperado(u32::from(b), "expected a value"),
            b'N' => {
                self.casar_literal("NaN")?;
                erro(
                    "Non-standard token 'NaN': enable `JsonReadFeature.ALLOW_NON_NUMERIC_NUMBERS` to allow",
                )
            }
            b'I' => {
                self.casar_literal("Infinity")?;
                erro(
                    "Non-standard token 'Infinity': enable `JsonReadFeature.ALLOW_NON_NUMERIC_NUMBERS` to allow",
                )
            }
            _ => self.valor_inesperado(b),
        }
    }

    /// `_handleUnexpectedValue`: o Jackson olha o byte cru como se fosse Latin-1
    fn valor_inesperado(&mut self, b: u8) -> R<Token> {
        let ch = char::from(b);
        if !(ch.is_alphabetic() || ch == '_' || ch == '$') {
            return char_inesperado(
                u32::from(b),
                &format!("expected a valid value {LISTA_VALORES}"),
            );
        }
        self.pos += 1;
        match self.espiar() {
            Some(c) if b >= 0x80 && c & 0xC0 == 0x80 => {
                erro(format!("Invalid UTF-8 start byte 0x{c:x}"))
            }
            _ => self.token_invalido(ch.to_string()),
        }
    }

    fn infinito(&mut self, negativo: bool) -> R<Token> {
        let alvo = if negativo { "-Infinity" } else { "+Infinity" };
        self.pos += 1;
        if self.espiar() == Some(b'n') {
            let resto = &alvo[3..];
            self.pos += 1;
            self.casar_resto(&alvo[..3], resto)?;
            return erro(format!(
                "Non-standard token '{alvo}': enable `JsonReadFeature.ALLOW_NON_NUMERIC_NUMBERS` to allow"
            ));
        }
        let c = self.byte_cru();
        let msg = if negativo {
            "expected digit (0-9) to follow minus sign, for valid numeric value"
        } else {
            "expected digit (0-9) for valid numeric value"
        };
        char_numerico_inesperado(c, msg)
    }

    fn casar_literal(&mut self, alvo: &str) -> R<()> {
        self.pos += 1;
        self.casar_resto(&alvo[..1], &alvo[1..])
    }

    /// `_matchToken`: casado = prefixo já consumido
    fn casar_resto(&mut self, casado: &str, resto: &str) -> R<()> {
        let mut lido = String::from(casado);
        for esperado in resto.bytes() {
            if self.espiar() != Some(esperado) {
                return self.token_invalido(lido);
            }
            self.pos += 1;
            lido.push(char::from(esperado));
        }
        if let Some(b) = self.espiar()
            && b >= b'0'
            && b != b']'
            && b != b'}'
        {
            let c = char::from_u32(self.char_para_erro()).unwrap_or(char::REPLACEMENT_CHARACTER);
            if eh_parte_identificador(c) {
                return self.token_invalido(lido);
            }
        }
        Ok(())
    }

    fn literal(&mut self, alvo: &str, token: Token) -> R<Token> {
        self.casar_literal(alvo)?;
        Ok(token)
    }

    fn token_invalido<T>(&mut self, prefixo: String) -> R<T> {
        let mut sb = prefixo;
        while sb.chars().count() < MAX_TOKEN_ERRO && self.pos < self.dados.len() {
            let c = char::from_u32(self.char_para_erro()).unwrap_or(char::REPLACEMENT_CHARACTER);
            self.avancar_char();
            if !eh_parte_identificador(c) {
                break;
            }
            sb.push(c);
        }
        if sb.chars().count() == MAX_TOKEN_ERRO {
            sb.push_str("...");
        }
        erro(format!(
            "Unrecognized token '{sb}': was expecting {LISTA_VALORES}"
        ))
    }

    /// `_decodeCharForError`: decodifica o caractere UTF-8 na posição atual, sem consumir
    fn char_para_erro(&self) -> u32 {
        let resto = &self.dados[self.pos.min(self.dados.len())..];
        let fim = resto.len().min(4);
        for n in 1..=fim {
            if let Ok(s) = std::str::from_utf8(&resto[..n])
                && let Some(c) = s.chars().next()
            {
                return u32::from(c);
            }
        }
        resto.first().map_or(0, |b| u32::from(*b))
    }

    fn avancar_char(&mut self) {
        let c = self.char_para_erro();
        let tam = char::from_u32(c).map_or(1, char::len_utf8);
        self.pos = (self.pos + tam).min(self.dados.len());
    }

    fn ler_numero(&mut self) -> R<Token> {
        let inicio = self.pos;
        let negativo = self.espiar() == Some(b'-');
        if negativo {
            self.pos += 1;
            match self.espiar() {
                Some(b'0'..=b'9') => {}
                Some(b'I') => return self.infinito(true),
                Some(_) => {
                    return char_numerico_inesperado(
                        self.byte_cru(),
                        "expected digit (0-9) to follow minus sign, for valid numeric value",
                    );
                }
                None => return erro("Unexpected end-of-input in a Number value"),
            }
        }
        if self.espiar() == Some(b'0') && matches!(self.dados.get(self.pos + 1), Some(b'0'..=b'9'))
        {
            return erro("Invalid numeric value: Leading zeroes not allowed");
        }
        self.digitos();
        let mut decimal = false;
        if self.espiar() == Some(b'.') {
            decimal = true;
            self.pos += 1;
            if self.digitos() == 0 {
                return self.sem_digito("Decimal point not followed by a digit");
            }
        }
        if matches!(self.espiar(), Some(b'e' | b'E')) {
            decimal = true;
            self.pos += 1;
            if matches!(self.espiar(), Some(b'+' | b'-')) {
                self.pos += 1;
            }
            if self.digitos() == 0 {
                return self.sem_digito("Exponent indicator not followed by a digit");
            }
        }
        let texto = String::from_utf8_lossy(&self.dados[inicio..self.pos]).into_owned();
        Ok(if decimal {
            Token::Decimal(texto)
        } else {
            Token::Inteiro(texto)
        })
    }

    fn sem_digito<T>(&self, msg: &str) -> R<T> {
        match self.espiar() {
            Some(_) => char_numerico_inesperado(self.byte_cru(), msg),
            None => erro("Unexpected end-of-input in a Number value"),
        }
    }

    fn digitos(&mut self) -> usize {
        let inicio = self.pos;
        while matches!(self.espiar(), Some(b'0'..=b'9')) {
            self.pos += 1;
        }
        self.pos - inicio
    }

    fn ler_string(&mut self, lugar: Lugar) -> R<String> {
        let mut bytes = Vec::new();
        loop {
            let Some(b) = self.espiar() else {
                return erro(lugar.eof());
            };
            self.pos += 1;
            match b {
                b'"' => break,
                b'\\' => self.ler_escape(&mut bytes, lugar)?,
                _ if b < 0x20 => {
                    return erro(format!(
                        "Illegal unquoted character ((CTRL-CHAR, code {b})): has to be escaped using backslash to be included in {}",
                        lugar.descricao()
                    ));
                }
                _ if b < 0x80 => bytes.push(b),
                _ => self.ler_multibyte(b, &mut bytes, lugar)?,
            }
        }
        String::from_utf8(bytes).map_err(|_| ErroJson::new("Invalid UTF-8"))
    }

    fn ler_multibyte(&mut self, b: u8, saida: &mut Vec<u8>, lugar: Lugar) -> R<()> {
        let extras = match b {
            0xC0..=0xDF => 1,
            0xE0..=0xEF => 2,
            0xF0..=0xF7 => 3,
            _ => return erro(format!("Invalid UTF-8 start byte 0x{b:x}")),
        };
        saida.push(b);
        for _ in 0..extras {
            let Some(c) = self.espiar() else {
                return erro(lugar.eof());
            };
            if c & 0xC0 != 0x80 {
                return erro(format!("Invalid UTF-8 middle byte 0x{c:x}"));
            }
            self.pos += 1;
            saida.push(c);
        }
        Ok(())
    }

    fn ler_escape(&mut self, saida: &mut Vec<u8>, lugar: Lugar) -> R<()> {
        let Some(b) = self.espiar() else {
            return erro(lugar.eof());
        };
        self.pos += 1;
        let c = match b {
            b'"' => '"',
            b'\\' => '\\',
            b'/' => '/',
            b'b' => '\u{8}',
            b'f' => '\u{c}',
            b'n' => '\n',
            b'r' => '\r',
            b't' => '\t',
            b'u' => return self.ler_unicode(saida, lugar),
            _ => {
                self.pos -= 1;
                let c = self.char_para_erro();
                return erro(format!(
                    "Unrecognized character escape {}",
                    descricao_char(c)
                ));
            }
        };
        let mut buf = [0u8; 4];
        saida.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        Ok(())
    }

    fn hex4(&mut self, lugar: Lugar) -> R<u32> {
        let mut v = 0u32;
        for _ in 0..4 {
            let Some(b) = self.espiar() else {
                return erro(lugar.eof());
            };
            let Some(d) = char::from(b).to_digit(16) else {
                return char_inesperado(
                    u32::from(b),
                    "expected a hex-digit for character escape sequence",
                );
            };
            self.pos += 1;
            v = (v << 4) | d;
        }
        Ok(v)
    }

    /// strings Java são UTF-16: surrogates soltos viram U+FFFD na conversão para Rust
    fn ler_unicode(&mut self, saida: &mut Vec<u8>, lugar: Lugar) -> R<()> {
        let alto = self.hex4(lugar)?;
        let c = if (0xD800..0xDC00).contains(&alto) && self.dados[self.pos..].starts_with(b"\\u") {
            let volta = self.pos;
            self.pos += 2;
            let baixo = self.hex4(lugar)?;
            if (0xDC00..0xE000).contains(&baixo) {
                char::from_u32(0x10000 + ((alto - 0xD800) << 10) + (baixo - 0xDC00))
            } else {
                self.pos = volta;
                None
            }
        } else {
            char::from_u32(alto)
        };
        let c = c.unwrap_or(char::REPLACEMENT_CHARACTER);
        let mut buf = [0u8; 4];
        saida.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        Ok(())
    }

    pub fn pular_filhos(&mut self) -> R<()> {
        if !matches!(self.atual, Some(Token::InicioObjeto | Token::InicioLista)) {
            return Ok(());
        }
        let mut nivel = 1usize;
        while nivel > 0 {
            match self.proximo()? {
                Some(Token::InicioObjeto | Token::InicioLista) => nivel += 1,
                Some(Token::FimObjeto | Token::FimLista) => nivel -= 1,
                Some(_) => {}
                None => return self.eof_sem_fechamento(),
            }
        }
        Ok(())
    }
}
