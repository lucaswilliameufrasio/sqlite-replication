# ADR 0002: perfil LTX v2 e versionamento explícito

- **Status:** Aceita para o perfil inicial (D-06/D-07)
- **Data:** 2026-10-23

## Contexto

LTX v2 e v3 usam o mesmo magic `LTX1`, mas têm layouts de frame distintos.
Inferir versão pelo magic pode interpretar dados incorretamente.

## Decisão

O primeiro perfil implementa LTX v2 conforme `spec/LTX_V2.md`. A versão é
configuração/metadado explícito fora do arquivo; o decoder não tenta adivinhar.
Referência v2 fixada em `superfly/ltx v0.4.0`, apenas para conformance. LTX v3
fica isolado e não bloqueia v1.

## Alternativas rejeitadas

- Inferir v2/v3 de `LTX1`: impossível de forma segura.
- Usar Litestream v0.5.x como oráculo v2: suas releases dependem do perfil v3.
- Importar a biblioteca de referência no runtime: viola independência.

## Consequências

Os repositórios devem declarar `format_version`; os testes mantêm fixtures
distintas e testam rejeição explícita de layouts incompatíveis.

O gate de conformance também exercita snapshots de bancos vazios, 100 mil/1
milhão de linhas, BLOBs, rollback e VACUUM via `conformance/snapshot_matrix.py`.
A identidade schema 1 tem o mesmo contrato UUID v4 hifenizado/variante RFC 4122
em Go e Rust (ver `spec/REPOSITORY_LAYOUT.md`). Manifestos inválidos devem ser
rejeitados antes da promoção, sem reinicialização silenciosa. Essa validação
corrige uma divergência entre a desserialização permissiva de UUID em Rust e a
validação Go; não muda os UUIDs gerados por versões anteriores do projeto.

## Reconsiderar quando

Adicionar leitura/escrita v3 somente após uma nova matriz de compatibilidade e
plano explícito de migração/versionamento.
