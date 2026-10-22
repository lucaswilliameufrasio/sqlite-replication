# Prompt para execução — SQLite Replication (Go + Rust)

Leia **integralmente** `PLANO_DESENVOLVIMENTO.md` e trate-o como especificação de produto, arquitetura, qualidade e critérios de aceite para construir duas bibliotecas open source de replicação SQLite embutidas em aplicações Go e Rust.

## Objetivo

Implemente os motores **inteiramente independentes**, sem importar Litestream, `superfly/ltx`, `literstream`, wrappers de replicação ou código de outro port na distribuição. São permitidas bibliotecas de uso geral (drivers SQLite, SDK S3, criptografia, compressão, tracing). Nunca use FFI entre os motores nem processos externos/sidecars para fazer a replicação. CLI/oráculo Litestream é opcional **somente** em testes de conformance.

## Princípios de execução

1. Comece imediatamente pelo marco **M0**, fazendo o menor vertical útil **M1**: snapshot SQLite consistente → LTX v2 → filesystem → restore verificado **em Go e Rust**.
2. Estabeleça contrato/fixtures compartilhados; cada linguagem implementa seu codec, state machine, spool e adaptadores de maneira própria.
3. Utilize **Floci** como único serviço S3 em testes de integração/CI. Utilize versão/digest pinados, sem MinIO, RustFS, AWS S3 ou R2 reais obrigatórios.
4. Garanta a mesma precisão de campos, layouts, TXIDs, checksums e decisões de versão em ambas as linguagens. **Não** deduza v2/v3 pelo magic `LTX1`.
5. Não faça benchmarking/research como etapa isolada e bloqueadora. Faça estudos técnicos estritamente necessários para implementar o próximo vertical; inicie benchmarks assim que o fluxo funcional permitir.
6. A fase **M2** tem gate obrigatório **CP-01**: prove, com testes concorrentes/crash, segurança da captura WAL/checkpoint. Não declare streaming funcional com meros snapshots ou parsing superficial de `-wal`.
7. Exija versão SQLite do runtime corrigida para o WAL-reset bug (>=3.51.3 para produção) e bloqueie drivers/conexões não homologadas que invalidem locks/checkpoints.
8. Nunca alegue que S3 simulado prova compatibilidade completa com provedores externos, nem que o sistema tem failover distribuído/multiwriter ou RPO zero.
9. Mantenha integração de application lifecycle sem listener HTTP, processo auxiliar ou dependência de cloud real. Não exponha segredos em logs.
10. Não anuncie etapas concluídas sem executar os testes pertinentes e registrar evidências observadas. Documente testes não executados e motivos.

## Fluxo de trabalho por milestone

- Leia specs e ADRs correspondentes ao incremento.
- Implemente o menor vertical possível, incluindo documentação e exemplos.
- Adicione testes unitários, conformance, integração Floci/SQLite e testes de falha adequados.
- Execute os comandos reais no ambiente disponível (`go test ./...`, `go vet ./...`, `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test --all-features`, suites adicionais).
- Registre resultados, limites, falhas e benchmark baseline.
- Atualize ADRs, manuais, checklist de aceite e status de riscos.
- Só prossiga para a próxima fase conforme dependências/gates do plano.

## Formato esperado de relatório em cada fase

1. **Implementado:** caminhos de arquivos e funcionalidades efetivamente criadas.
2. **Verificado:** testes/comandos executados, resultado observável e evidências.
3. **Pendente:** tarefas de curto prazo e incompatibilidades conhecidas.
4. **Decisões:** ADRs incluídas/revisadas, alternativas e tradeoffs.
5. **Próximo vertical:** menor funcionalidade útil que mantém a cadeia correta.

Não substitua implementação por um novo documento de pesquisa. Se o ambiente impedir um teste, informe a limitação explicitamente; não o marque como aprovado.
