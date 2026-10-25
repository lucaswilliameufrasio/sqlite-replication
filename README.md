# SQLite Replication

Independent Go and Rust SQLite replication libraries

**Status:** M1 implementado e verificado em Go e Rust pelo gate local `make
test`, incluindo 16 casos/96 restores de snapshots LTX v2 (até 1 milhão de
linhas e BLOBs de 16 MiB) contra o oráculo fixado e smoke Floci persistente.
O gate local de M0 passou, mas a matriz ampla de casos segue
parcial. Parsers WAL são isolados e não habilitam captura incremental. O
projeto ainda não oferece adapter S3 da biblioteca, retenção ou compactação.
CP-01 continua bloqueando qualquer promessa de segurança WAL.

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
