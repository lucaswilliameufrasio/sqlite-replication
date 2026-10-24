# ADR 0007: escritor único e ausência de failover distribuído

- **Status:** Aceita para v1 (D-04)
- **Data:** 2026-10-23

## Contexto

Storage imutável e checksums evitam sobrescrever uma chave, mas não elegem um
writer nem impedem que dois hosts avancem cadeias concorrentes.

## Decisão

v1 é disaster recovery de um único escritor lógico por banco/repositório. Exigir
exclusividade local do replicador e identidade estável. Não alegar multiwriter,
failover automático, lease distribuído, fencing ou RPO zero. Detectar e falhar
quando conflito é observável; documentar que impossibilidade de detectar um
writer remoto não significa exclusividade garantida.

## Alternativas rejeitadas

- Inferir lease a partir de nome/lock local: locks locais não coordenam hosts.
- Usar `If-None-Match` como eleição: protege objeto, não a cadeia inteira.
- Resolver split-brain escolhendo a cadeia mais recente: pode descartar commits
  válidos.

## Consequências e evidência

Conflito de chave divergente é erro crítico. Operação multi-host exige uma
revisão de arquitetura com lease/fencing confiável e testes de partição antes de
qualquer nova promessa.

## Reversão / reconsideração

Revisar somente com mecanismo de ownership distribuído comprovado e política de
reconciliação que não escolha silenciosamente entre históricos divergentes.
