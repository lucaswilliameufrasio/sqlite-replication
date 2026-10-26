# Testes locais

## Ferramentas

- Go 1.27.1 ou posterior compatível com `go.mod`.
- Rust/Cargo 1.98.1 ou posterior compatível com `Cargo.toml`.
- M1 usa filesystem local e SQLite embutido; não requer credenciais nem S3.
- O conformance requer Python 3 com módulo padrão `sqlite3`. Python cria e
  verifica somente fixtures offline; snapshots e restores das bibliotecas usam
  os runtimes SQLite Go/Rust homologados, não o runtime do Python.
- O runner de conformance precisa continuar compatível com Bash 3.2, ainda
  disponível por padrão em runners macOS; não usar `mapfile`/`readarray`.
- `make test-floci` e `make test` requerem Docker Compose, Python 3, Go/Cargo e
  dependências baixadas pelos SDKs. O alvo usa somente `floci/floci:2.3.0`
  pinado por digest, credenciais sintéticas e S3 local; nunca chama AWS S3 ou
  Cloudflare R2.
- O smoke escolhe o contexto Docker `rootless` quando existe e confirma a
  opção `name=rootless` do daemon. Cada execução usa nome de projeto e volume
  próprios e remove apenas esses recursos no teardown.
- `make test-cp01-enospc` é um drill opt-in para Linux amd64: requer Docker com
  daemon rootless verificado, a imagem Debian pinada por digest já disponível
  localmente, Go, Cargo, Python 3 e `/tmp/opencode`. Não baixa imagens nem usa
  rede; roda os testes em container com `/cp01` limitado a 8 MiB de tmpfs e
  `/tmp` a 32 MiB.
  O ENOSPC resultante é real para esse tmpfs, mas não simula falha física de
  `fsync`, perda de energia ou comportamento de outros filesystems.

## Comandos

- `make fmt-check`
- `make vet-go`
- `make clippy-rust`
- `make test-go`
- `make test-rust`
- `make test-conformance` (Go↔Rust e `superfly/ltx v0.4.0` como oráculo apenas
  de teste; compilação de ambas as toolchains é requisito; inclui a matriz de
  snapshots descrita abaixo, com um milhão de linhas e BLOB de 16 MiB)
- `make test-floci` (testa adapters S3 Go/Rust contra Floci: PUT imutável/retry/
  conflito, zero bytes, multipart, metadata, checksum, HEAD/GET range, paginação,
  reconciliação (missing/divergent/unexpected) e delete batch; publica fixture
  LTX em Go, mata Floci com SIGKILL, reinicia e valida restore Go↔Rust a partir
  dos objetos de cada linguagem. Suites Go/Rust também usam servidor HTTP local
  para falhas 503, retry limitado/exaurido, timeout, cancelamento e resposta
  perdida após PUT;
  recursos Floci são isolados e removidos)
- `make test-cp01-enospc` (drill opt-in de ENOSPC em tmpfs limitado, publicação
  parcial, watermark e recuperação Go/Rust; exige as ferramentas acima)
- `make test-native` (format, lint, suites Go/Rust e conformance, sem o smoke
  Floci que requer Docker; usado nos runners nativos Linux/macOS arm64)
- `make test` (format, lint, suites Go/Rust, conformance e adapters/Floci; não
  executa o drill opt-in `make test-cp01-enospc` nem os drills CP-01)

## Matriz nativa arm64 no GitHub Actions

Push e pull request executam somente o job Linux amd64 padrão. Os jobs Linux
arm64 e macOS arm64 só podem ser iniciados por `workflow_dispatch`, marcando
`run_native_architecture_matrix`; a opção vem desmarcada e, sem ela, nenhum
runner é iniciado pelo despacho manual. O job padrão também não roda em
`workflow_dispatch`. Em repositórios privados, minutos de runners hospedados
podem ser cobrados; marque a opção somente quando quiser executar ambos os
jobs. O despacho manual precisa usar um workflow presente no branch padrão do
repositório. Essa matriz executa `make test-native`, não o drill Linux amd64
`make test-cp01-enospc`, e seus resultados não aprovam CP-01 por si sós.

Os testes devem usar diretórios temporários exclusivos e removê-los ao terminar.
Não executar testes contra bancos de desenvolvimento ou credenciais externas.
O gate CP-01 permanece obrigatório antes de declarar captura WAL/checkpoint
incremental segura.

## Matriz de snapshots

`conformance/snapshot_matrix.py` é chamado pelo runner do gate; não é dependência
de runtime das bibliotecas. Compila-se um executável de cada linguagem e do
oráculo fixado, sem recompilação por caso. A matriz executa 16 casos:

O executável Rust da matriz usa `cargo build --release`; as suites unitárias e
integrações Rust continuam no perfil debug do `cargo test`. Uma primeira
tentativa em release ainda excedeu 300 s: o CRC-64 ISO Rust processava cada bit
em um loop de oito iterações por byte. A implementação agora usa uma tabela
CRC-64 ISO calculada em compilação, com vetor padrão `123456789` e conformance
cruzado. A matriz final passou mantendo o milhão de linhas, asserts completos e
timeout original de 300 s. Isso corrige o bloqueio funcional do gate; não é
comparação de throughput entre Go e Rust nem evidência dos benchmarks de M6.

- banco sem tabelas e tabela com zero linhas, cada um com páginas de
  512/1024/4096/8192/65536 bytes;
- 1, 100 mil e 1 milhão de linhas (páginas de 4096);
- BLOBs de 1 e 16 MiB, com todos os valores possíveis de byte;
- rollback de alterações, remoções e DDL;
- redução física por `VACUUM`, seguida de reinserção/reutilização de IDs.

Cada caso passa por seis rotas: Go→Rust, Go→oráculo, Rust→Go, Rust→oráculo e
oráculo→Go/Rust. Os restores usam os adapters reais. Cada imagem passa por
`quick_check`, `integrity_check`, `foreign_key_check` e SHA-256 lógico de todo o
schema e de cada valor ordenado por chave primária. O hash distingue tipos,
limites dos valores, NULL, BLOB e texto, incluindo Unicode e NUL. Também compara
page size, encoding, application ID e user version. Não basta somar IDs ou
comparar contagem/tamanho das linhas. Não se exige igualdade dos bytes físicos
SQLite, que podem mudar legitimamente no Online Backup.

O caso de uma linha também exercita 16 manifestos inválidos em ambos os motores
(32 restores rejeitados, sem promoção nem alteração dos metadados), além de duas
restaurações com UUIDs válidos em maiúsculas. Os testes Python do próprio oráculo
comprovam detecção de conteúdo alterado de mesmo tamanho, índice ausente,
distinção de tipos e ausência de criação de arquivo durante verificação.

As fixtures são isoladas e descartadas por caso. O hash percorre linhas em
streaming; um valor BLOB é materializado pelo driver Python, limitado aos 16 MiB
da fixture. A matriz é conformance funcional, não benchmark, teste de limites de
memória nem prova CP-01. Milhão de linhas/BLOB/rollback/VACUUM são exercitados
como snapshots offline, não como captura WAL concorrente. A parte incremental
do runner continua limitada às duas cadeias internas já descritas em CP-01.
