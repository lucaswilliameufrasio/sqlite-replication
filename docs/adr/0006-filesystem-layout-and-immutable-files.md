# ADR 0006: layout filesystem e publicação imutável

- **Status:** Aceita para M1 (D-08/D-10)
- **Data:** 2026-10-23

## Contexto

O primeiro vertical precisa publicar um snapshot local sem expor arquivo
parcial e sem sobrescrever uma réplica já existente.

## Decisão

Usar `<root>/ltx/<level decimal>/<min>-<max>.ltx`; M1 escreve nível `0`.
Gerar staging no mesmo filesystem, sincronizar conteúdo, publicar sem
substituição e sincronizar o diretório. Se a chave já existir, somente aceitar
retry quando os bytes forem idênticos.

## Alternativas rejeitadas

- Escrever diretamente na chave final: crash pode deixar um arquivo parcial.
- Sobrescrever em retry: pode destruir histórico imutável divergente.
- Copiar layout filesystem para S3: layouts dos backends não são equivalentes.

## Consequências

M1 depende de semântica local de hard-link/rename e `fsync` do filesystem. O
adapter S3 terá contrato e testes próprios em M3.

## Reconsiderar quando

Se um filesystem suportado não oferecer publicação atômica sem overwrite,
falhar no startup ou introduzir protocolo específico com prova de durabilidade.
