# Calculadora de Tributos

## Descrição do Projeto

API REST que realiza o cálculo da CBS, do IBS e do Imposto Seletivo (Reforma Tributária do Consumo — LC 214/2025) nas operações do regime geral, consulta as tabelas oficiais usadas no cálculo e gera e valida os grupos de tributação dos documentos fiscais eletrônicos.

---

## Público-Alvo

- Empresas de tecnologia
- Desenvolvedores
- Contribuintes em geral

---

## Funcionalidades

Todos os caminhos começam com `http://localhost:8080/api/calculadora`. A lista completa, com esquemas e exemplos, está no Swagger (`http://localhost:8080/api/swagger-ui.html`).

| Funcionalidade | Endpoint |
|---|---|
| Cálculo de tributos (a operação inteira falha no primeiro erro) | `POST /regime-geral` |
| Cálculo com observabilidade (erros por item, sem interromper os demais) | `POST /observabilidade/regime-geral` |
| Descrição de um código de erro da observabilidade | `GET /observabilidade/erros?codigo=REG-011` |
| Base de cálculo unificada da NF-e/NFC-e | `POST /base-calculo/base-calculo-unificada` |
| Base de cálculo unificada da NFS-e | `POST /base-calculo/base-calculo-unificada-nfse` |
| Geração do XML a partir do resultado do cálculo | `POST /xml/generate?tipo=nfe` |
| Validação de XML (grupo ou nota) contra o XSD | `POST /xml/validate?tipo=nfe&subtipo=grupo` |
| CST e classificações tributárias (cClassTrib) | `GET /dados-abertos/situacoes-tributarias/cbs-ibs`, `GET /dados-abertos/classificacoes-tributarias/...` |
| Fundamentações legais | `GET /dados-abertos/fundamentacoes-legais` |
| NCM e NBS | `GET /dados-abertos/ncm`, `GET /dados-abertos/nbs` |
| Alíquotas de referência (União, UF e município) | `GET /dados-abertos/aliquota-uniao`, `/aliquota-uf`, `/aliquota-municipio` |

Exemplos de utilização da API estão em [docs/exemplos.md](docs/exemplos.md).

---

## Requisitos

### Requisitos do Sistema

- **Rust**: toolchain estável (edition 2024), instalada via [rustup](https://rustup.rs)
- **Git**: para clonar o repositório
- **Task**: opcional, para os atalhos do `Taskfile.yml` ([taskfile.dev](https://taskfile.dev))
- **Sistema Operacional**: Windows, macOS ou Linux

### Organização

| Crate | Conteúdo |
|---|---|
| `core` (`rtc-core`) | Cálculo, regras de validação, consultas ao banco, geração e validação de XML |
| `server` (`rtc-server`) | API HTTP, Swagger, CORS, métricas e logs |
| `banco` (`rtc-banco`) | Gerador do banco SQLite a partir das migrações em `flyway/sql` |

O SQLite é compilado junto (`rusqlite` com `bundled`); não é preciso instalar nada além do Rust.

---

## Instruções de Instalação

### 1. Clonar o Repositório

```bash
git clone https://github.com/ReceitaFederal/calculadora-rtc-regime-regular.git
cd calculadora-rtc-regime-regular
```

### 2. Gerar o Banco de Dados (obrigatório)

O servidor lê o banco `./calculadora/db/calculadora-pro.db`, relativo ao diretório de execução, em modo somente leitura. O gerador aplica as migrações de `flyway/sql` com as mesmas regras do Flyway (baseline, versões, checksums e callbacks).

> **Atenção:** a aplicação só funciona se o banco estiver gerado.

```bash
task banco
```

Ou diretamente:

```bash
cargo run --release -p rtc-banco --bin rtc-gerar-banco -- --sql flyway/sql --banco calculadora/db/calculadora-pro.db
```

### 3. Executar os Testes

```bash
task test
```

Ou `cargo test --workspace`. Os testes geram o próprio banco temporário, então não dependem do passo 2.

O teste `server/tests/paridade.rs` sobe o servidor e reenvia mais de 1800 requisições gravadas da versão Java, comparando status, cabeçalhos e corpo das respostas. Para rodar só ele:

```bash
task paridade
```

### 4. Executar a Aplicação

```bash
task run
```

Ou `cargo run --release -p rtc-server`. A aplicação estará disponível em `http://localhost:8080/api`; health e métricas (formato Actuator) ficam na porta `9101`.

Variáveis de ambiente:

| Variável | Padrão | Descrição |
|---|---|---|
| `SERVER_PORT` | `8080` | Porta da API |
| `MANAGEMENT_SERVER_PORT` | `9101` | Porta de métricas e health |
| `APPLICATION_DB_FILENAME` | `calculadora-pro.db` | Nome do arquivo do banco em `./calculadora/db` |
| `LOGGING_LEVEL_ROOT` | `INFO` | Nível mínimo de log |
| `VERSAO_OFFLINE_URL` | serviço piloto | URL consultada para comparar a versão local com a publicada |

### 5. Gerar o Binário

```bash
task build
```

Os binários `rtc-server` e `rtc-gerar-banco` ficam em `bin/`. Execute o servidor a partir de um diretório que contenha `./calculadora/db`.

### 6. Docker

```bash
docker build -t calculadora-rtc-regime-regular .
docker run -p 8080:8080 calculadora-rtc-regime-regular
```

A imagem já sai com o banco gerado.

---

## Referências

- [Code of conduct](https://www.contributor-covenant.org)
- [MIT license](https://choosealicense.com/licenses/mit/)
- [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/)
- [Semantic Versioning](https://semver.org/lang/pt-BR/)

## Licença

Este projeto é distribuído sob a licença especificada no arquivo [LICENSE](LICENSE).
