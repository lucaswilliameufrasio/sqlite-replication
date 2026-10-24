# Testes locais

## Ferramentas

- Go 1.27.1 ou posterior compatível com `go.mod`.
- Rust/Cargo 1.98.1 ou posterior compatível com `Cargo.toml`.
- M1 usa filesystem local e SQLite embutido; não requer credenciais nem S3.
- `make test-floci` e `make test` requerem Docker Compose e Python 3. O smoke
  usa somente `floci/floci:2.3.0` pinado por digest, credenciais fictícias e
  S3 local; nunca chama AWS S3 ou Cloudflare R2.
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
  de teste; compilação de ambas as toolchains é requisito)
- `make test-floci` (Floci persistente: cria bucket/objeto local, para e inicia
  o mesmo serviço mantendo volume, confere o objeto, e remove os recursos
  isolados da execução)
- `make test-cp01-enospc` (drill opt-in de ENOSPC em tmpfs limitado, publicação
  parcial, watermark e recuperação Go/Rust; exige as ferramentas acima)
- `make test-native` (format, lint, suites Go/Rust e conformance, sem o smoke
  Floci que requer Docker; usado nos runners nativos Linux/macOS arm64)
- `make test` (format, lint, suites Go/Rust, conformance e Floci; não executa o
  drill opt-in `make test-cp01-enospc`; valida o emulador, não um adapter S3 da
  biblioteca, que ainda não existe)

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
