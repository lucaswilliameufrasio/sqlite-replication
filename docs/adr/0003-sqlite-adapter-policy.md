# ADR 0003: adapters SQLite do primeiro vertical

- **Status:** Aceita para M1; suporte de WAL permanece aberto
- **Data:** 2026-10-23

## Contexto

Snapshots consistentes precisam usar SQLite Online Backup API, respeitando o
runtime/locking usado pela aplicação. A política de produção do plano requer
SQLite >= 3.51.3.

## Decisão

Go começa com `modernc.org/sqlite` e Online Backup API através da conexão
dedicada. Rust começa com `rusqlite` e feature `backup`. Ambos verificam a
versão real do runtime antes de snapshot/restore. Esta decisão homologa apenas
o caminho de snapshot M1, não a captura WAL nem combinações arbitrárias de
drivers/pools.

## Alternativas rejeitadas

- Copiar o arquivo SQLite vivo: não é consistente em WAL.
- Go `mattn/go-sqlite3` ou Rust `sqlx` como adapter inicial: exigiriam prova
  própria de locking e lifecycle antes de suporte.

## Consequências

Drivers não homologados falham explicitamente. CP-01 e testes de conexões,
locks e checkpoint são pré-requisitos separados para streaming incremental.

## Reconsiderar quando

Adicionar driver/OS somente com matriz de versão SQLite, locking, checkpoint,
conexões externas e testes de falha aprovada.
