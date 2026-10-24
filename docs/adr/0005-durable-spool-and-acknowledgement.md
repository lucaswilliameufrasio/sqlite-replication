# ADR 0005: spool durável e confirmação remota

- **Status:** Aceita para a arquitetura incremental futura; não implementada em M1
- **Data:** 2026-10-23

## Contexto

Upload pode falhar, ficar lento ou perder a resposta depois de o servidor já ter
armazenado o objeto. Memória/process-local queue não protege trabalho após crash
e status local não equivale a confirmação remota.

## Decisão

Serializar LTX completo em staging, validar e sincronizar arquivo antes de
publicá-lo no spool imutável. Persistir metadados/watermark local; fazer upload
idempotente por chave imutável e checksum. Só avançar `remote_confirmed_txid`
depois da confirmação verificável e da persistência local do acknowledgement.
Após restart, reconciliar spool e objetos remotos antes de repetir/limpar dados.

## Alternativas rejeitadas

- Fila apenas em memória: perde trabalho no crash.
- Avançar watermark no início do upload: afirma recuperação sem confirmação.
- Retry com overwrite: pode destruir conteúdo divergente sob chave imutável.
- ETag como hash universal: multipart e provedores podem usar semânticas
  diferentes.

## Consequências e evidência

No futuro, testar crashes em cada fronteira stage/fsync/publish/PUT/ack e
reconciliação de resposta perdida. A implementação precisa limitar bytes e
concorrência do spool e manter commits locais, staged e remotos como posições
distintas. M1 ainda não implementa fila nem watermark remoto.

## Reversão / reconsideração

Uma alteração do protocolo de acknowledgement exige migração versionada e teste
de replay idempotente antes de descartar spool anterior.
