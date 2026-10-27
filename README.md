# SQLite Replication

Independent Go and Rust SQLite replication libraries

**Status:** M0 permanece parcial; M1 está implementado e verificado em Go e Rust
pelo gate local `make test`, incluindo 16 casos/96 restores de snapshots LTX v2
(até 1 milhão de linhas e BLOBs de 16 MiB), fixtures independentes de deltas,
compactação e uma transação de 4 MiB. M3 está implementado e verificado
localmente: adapters S3 Go/Rust, métricas, reconciliação read-only e testes de
falha/retry/cancelamento com restore cruzado após crash/restart do Floci. M4
possui restore, compactação, retenção e GC filesystem/S3 implementados e
verificados localmente, ainda privados e sem fechamento operacional completo.
Isso não homologa AWS S3, R2 ou outro provedor. O fuzzing Go do codec LTX começou;
M5/M6 continuam incompletos. Parsers WAL são isolados e não habilitam captura
incremental. CP-01 segue pendente e bloqueia exposição/qualquer promessa de
segurança WAL.

## Documentos
- [Plano de desenvolvimento](docs/PLANO_DESENVOLVIMENTO.md)
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
