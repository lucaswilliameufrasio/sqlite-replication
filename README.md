# SQLite Replication

Rust SQLite replication library. The independent Go implementation lives in
[`sqlite-replication-go`](https://github.com/lucaswilliameufrasio/sqlite-replication-go).
Cross-language verification checks out the Go implementation separately; see
`docs/TESTING.md` for the pinned revision and local setup.

**Status:** M0 permanece parcial; M1 está implementado e verificado em Go e Rust
pelo gate local `make test`, incluindo 16 casos/96 restores de snapshots LTX v2
(até 1 milhão de linhas e BLOBs de 16 MiB), fixtures independentes de deltas,
compactação e uma transação de 4 MiB. M3 está implementado e verificado
localmente: adapters S3 Go/Rust, métricas, reconciliação read-only e testes de
falha/retry/cancelamento com restore cruzado após crash/restart do Floci. M4
possui restore, compactação, retenção e GC filesystem/S3 implementados e
verificados localmente, ainda privados e sem fechamento operacional completo.
Isso não homologa AWS S3, R2 ou outro provedor. M6 está em andamento: o CI roda
fuzz-smokes Go para os decoders LTX e parser WAL; testes Rust verificam
truncamentos/mutações dos decoders LTX e do parser WAL com vetores
independentes. CP-01 foi aprovado somente para Linux x86-64, nas combinações de
SQLite testadas e com um escritor lógico coordenado; ARM64 e perda física de
energia/cache ficam fora da garantia. Go `replicator.CaptureOnce` e Rust
`capture_once` agora expõem captura síncrona de um ciclo para spool local nesse
escopo, exigindo write-gate explícito. Não solicitam checkpoint nem fazem upload
remoto; restores incrementais filesystem e S3 estão disponíveis por TXID ou
timestamp. Sync explícito local→S3 com ACK durável por destino está implementado
em Go/Rust, com publicação do manifesto por último e sem apagar spool. O fluxo
integrado Go/Rust e `make test` passaram no Floci desta revisão.
`Status`/`Health`/`Flush` estão parcialmente disponíveis via coordenador
explícito Go/Rust; o worker `Run` exige notificações do adapter e admission
control estrito. Métricas/traces, validação de adapters de produção e exemplos
completos seguem pendentes. M5 e o restante de M6 permanecem incompletos.

## Documentos
- [Development plan](docs/development-plan.md)
- [Migração da toolchain Rust](docs/migration-rust-toolchain.md)
- [SBOM e triagem de licenças](docs/sbom/2026-11-05/README.md)
- [Prompt de execução](docs/PROMPT_EXECUCAO.md)
- [Contrato LTX v2](spec/LTX_V2.md)
- [Como testar](docs/TESTING.md)
- [Runner de conformance](conformance/run-cross-language.sh)
- [Matriz de snapshots e validação de metadados](conformance/snapshot_matrix.py)
- [Contrato de captura WAL](spec/WAL.md)
- [Gate CP-01](spec/CHECKPOINT_PROTOCOL.md)
- [Infraestrutura Floci](infra/compose.yaml)
- [Prova local de persistência Floci](infra/floci-smoke.sh)
- [Exemplo de snapshot e restore](docs/examples/snapshot-restore.md)
