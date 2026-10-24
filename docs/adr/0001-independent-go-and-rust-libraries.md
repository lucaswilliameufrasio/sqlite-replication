# ADR 0001: bibliotecas Go e Rust independentes

- **Status:** Aceita (conforme decisão D-01/D-02 do plano)
- **Data:** 2026-10-23

## Contexto

O produto oferece duas bibliotecas embarcadas para aplicações Go e Rust. Um
runtime compartilhado ou FFI acoplaria builds, ciclos de vida e falhas.

## Decisão

Manter dois motores nativos independentes em um monorepo. Compartilhar somente
especificações, fixtures e casos de conformance; não compartilhar código de
runtime nem importar Litestream/LTX como dependência de distribuição.

## Alternativas rejeitadas

- FFI entre Go e Rust: acopla ABI, lifecycle e diagnóstico.
- Um único motor/sidecar: contraria a integração embutida e a independência dos
  pacotes.

## Consequências

Cada linguagem mantém codec, SQLite adapter, storage e testes próprios. Uma
alteração no contrato exige atualização de spec, fixtures e ambas as suites.

## Reconsiderar quando

Se o custo de manutenção tornar impossível manter conformance sem compartilhar
runtime, revisar semântica e custos de distribuição antes de mudar a decisão.
