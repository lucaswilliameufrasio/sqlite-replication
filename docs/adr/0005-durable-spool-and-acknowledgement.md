# ADR 0005: spool durável e confirmação remota

- **Status:** Implementada para sync explícito local→S3 compatível; integração verificada somente no Floci
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

Testar crashes em cada fronteira stage/fsync/publish/PUT/ack e reconciliação de
resposta perdida continua necessário. A implementação precisa limitar bytes e
concorrência do spool e manter commits locais, staged e remotos como posições
distintas; os limites operacionais seguem pendentes em ADR 0011.

### Contrato do primeiro sync remoto local

O sync de spool é uma operação explícita, serializada pelo chamador, que enumera
e valida a cadeia LTX completa antes de publicar. Publica cada LTX com chave
imutável e o manifesto por último para evitar anunciar um repositório incompleto;
retry idempotente valida bytes/checksum e conflito é erro.
Não remove spool nem faz GC. Só depois de todas as confirmações, grava e
sincroniza `_meta/remote-ack.json` com versão, `remote_id`, prefixo, TXID e
checksum de banco. `remote_id` é um identificador estável, não secreto,
fornecido pela aplicação para distinguir endpoints/buckets; não inferi-lo apenas
do nome do bucket nem registrar endpoints/credenciais. Status só pode atribuir o
ACK ao destino cujo `remote_id` e prefixo correspondam. Um ACK ausente, inválido
ou de outro destino é desconhecido, não zero nem confirmação atual.

O sync não constitui compatibilidade universal S3/R2: a evidência de integração
continua sendo somente Floci. Uma operação por repositório por vez é exigida;
captura posterior pode avançar o spool enquanto o sync transmite, mas o sync só
confirma o TXID cujo conjunto foi validado e integralmente enviado.

## Reversão / reconsideração

Uma alteração do protocolo de acknowledgement exige migração versionada e teste
de replay idempotente antes de descartar spool anterior.
