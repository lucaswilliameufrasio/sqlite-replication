# ADR 0012: segurança, observabilidade e gates de release

- **Status:** Guardrails aceitos; detalhes de release ainda pendentes
- **Data:** 2026-10-23

## Contexto

O motor é embutido em processos de aplicação; não deve iniciar listener HTTP,
expor credenciais ou confundir estado local/remoto. Pacotes públicos ainda não
estão prontos.

## Decisão

- Expor observabilidade por interfaces/registradores injetados (OpenTelemetry e
  métricas Prometheus conforme plano), sem servidor HTTP obrigatório.
- Não registrar secrets, tokens, SQL ou conteúdo de aplicação; usar categorias
  de erro sem labels arbitrárias de alta cardinalidade.
- Validar runtime SQLite e combinação driver antes de operar; versão de produção
  mínima é >=3.51.3 conforme política do projeto.
- Tratar Floci como evidência local, nunca como prova universal de S3.
- Não anunciar uma capacidade sem teste, evidência e gate correspondentes;
  CP-01 é pré-requisito para qualquer promessa de streaming WAL.
- Apache-2.0 é licença proposta pelo plano; revisar notices/licenças de
  dependências e fixtures antes da publicação.

## Alternativas rejeitadas

- Listener ou framework HTTP dentro da biblioteca: compete com lifecycle da
  aplicação.
- Secrets em logs de diagnóstico: expõe credenciais.
- Confiar em runtime SQLite/driver não validado: enfraquece locks/checkpoint.
- Tratar teste local como homologação externa ou benchmark não executado como
  resultado.

## Consequências e pendências

Status/health, métricas, traces, threat model, SECURITY.md, SBOM e licença de
distribuição precisam ser finalizados antes do release. Go/Rust mantêm
registradores idiomáticos e não compartilham runtime. M5/M6 continuam
incompletos.

## Reversão / reconsideração

Revisar após threat model e revisão de supply chain; qualquer export de
telemetria deve preservar o princípio de dados mínimos e redaction.
