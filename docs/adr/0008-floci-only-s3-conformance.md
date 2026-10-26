# ADR 0008: Floci como único S3 de conformance

- **Status:** Aceita para desenvolvimento e CI (D-09)
- **Data:** 2026-10-23

## Contexto

Testes automatizados precisam validar SDK/adapters sem contas, credenciais ou
recursos cloud reais. Um emulador local também não prova compatibilidade
completa de todos os provedores S3-compatible.

## Decisão

Usar Floci como único serviço S3 em testes de integração. Fixar release e digest
OCI, usar `persistent` com volume e credenciais sintéticas. A referência atual
é `floci/floci:2.3.0`, index digest
`sha256:e0b292df4dc9559b7b1bf4f51bdda4bb8e269276236438cd03cae661260ea8a9`,
registrado em `infra/floci-image.lock`. Não usar `latest`, MinIO/RustFS ou cloud
real como requisito de CI.

## Alternativas rejeitadas

- AWS S3/R2 obrigatórios: exigem credenciais/custos e tornam CI não isolada.
- Emuladores adicionais: aumentam matriz sem alterar o contrato definido.
- Tag sem digest: pode resolver para bytes diferentes em outro dia.

## Consequências e evidência

`infra/floci-smoke.sh` isola e limpa projeto/volume e executa conformance dos
adapters Go/Rust, inclusive PUT imutável/multipart, reconciliação, SIGKILL,
restart do volume e restores cruzados. Registrar comportamento não suportado
pelo Floci como limitação, não como teste aprovado. Resultado não homologa
provedor externo.

## Reversão / reconsideração

Atualizar tag/digest somente por alteração revisada do lock, verificando release
canônica, digest do registry e smoke completo. Adicionar outro backend de teste
exige decisão explícita de produto.
