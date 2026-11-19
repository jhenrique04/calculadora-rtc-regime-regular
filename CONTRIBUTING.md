# Guia de Contribuição

Obrigado pelo interesse em contribuir com a **Calculadora de Tributos**!
Este guia descreve como você pode participar do projeto de forma organizada e efetiva.

---

## 1. Formas de Contribuição

Este projeto é mantido pela Receita Federal do Brasil (RFB) e pelo SERPRO.
**Pull Requests externos não são aceitos diretamente.** Contribuições externas devem ser
encaminhadas exclusivamente pelos canais oficiais listados abaixo:

| Canal                         | Tipo de Contribuição                                                                  |
|-------------------------------|--------------------------------------------------------------------------------------|
| **Issues do repositório**     | Relato de bugs, sugestões de melhoria, dúvidas técnicas                              |
| **Discussions do repositório**| Debate de propostas      |

> **Nota:** Sugestões e correções recebidas por Issues ou Discussions serão avaliadas pela
> equipe definida pela área de negócio da RFB responsável pelo projeto e, se aprovadas,
> incorporadas por meio dos processos internos de desenvolvimento.

---

## 2. Relatos sobre o código

### Bug Report

Ao relatar um bug, inclua:

- Versão da aplicação utilizada (campo `versao` no retorno da API ou `server/Cargo.toml`)
- Endpoint afetado e payload de entrada utilizado
- Comportamento observado vs. comportamento esperado
- Logs de erro relevantes
- Versão do Rust e sistema operacional

### Sugestão de Melhoria

Ao sugerir uma melhoria, inclua:

- Descrição clara do problema ou necessidade
- Referência normativa aplicável (artigo de lei, nota técnica, portaria, etc.)
- Proposta de solução ou comportamento esperado
- Impacto esperado sobre outros endpoints ou funcionalidades

---

## 3. Pull Requests

**Este repositório não aceita Pull Requests abertos pelo público em geral no GitHub.**

Eventuais Pull Requests abertos por usuários externos serão encerrados sem mesclagem.

---

## 4. Padrões de Código

- **Linguagem dos identificadores:** Português sem acentuação (ex: `calcular_aliquota`, `buscar_ncm`)
- **Estilo:** convenções padrão Rust (snake_case para funções/variáveis, PascalCase para tipos), formatado com `cargo fmt`
- **Análise estática:** `cargo clippy --workspace --all-targets -- -D warnings` sem alertas; sem `unwrap`/`expect` fora de testes
- **Logs:** usar o logger do servidor com níveis adequados (`DEBUG`, `INFO`, `WARN`, `ERROR`). Proibido `println!` no servidor
- **Testes:** testes unitários obrigatórios; mudanças de comportamento da API devem manter o teste de paridade (`task paridade`) passando

---

## 5. Código de Conduta

Ao participar deste projeto, você concorda em seguir nosso [Código de Conduta](CODE_OF_CONDUCT.md).