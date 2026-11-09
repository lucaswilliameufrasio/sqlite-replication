# Contrato dos backends de storage

## Interface lógica

Os motores Go e Rust implementam adapters nativos para as operações equivalentes
a:

- `PutImmutable(key, stream, metadata)`;
- `Get(key, optional byte range)`;
- `Head(key)`;
- `List(prefix, cursor)`;
- `DeleteBatch(keys)`;
- `Reconcile(prefix, expected immutable objects)`;
- `Capabilities()`.

I/O aceita cancelamento/timeout onde a linguagem permite. Streams são
processados incrementalmente e com buffers limitados; erros incluem operação,
chave redigida quando necessário e classificação retryable/permanent, nunca
credenciais ou payload SQL.

## Put imutável

1. Calcular tamanho e hash do conteúdo enquanto lê, sem confiar em ETag como
   MD5.
2. Enviar com condição create-only (`If-None-Match: *`) quando suportada.
3. Se a chave já existe, comparar tamanho e checksum do conteúdo existente.
4. Aceitar retry apenas quando o conteúdo é idêntico; divergência é conflito
   fatal de cadeia e nunca é sobrescrita.
5. Avançar watermark remoto somente depois de confirmar o objeto e persistir
   localmente o acknowledgement correspondente.

O contrato não presume atomicidade de multipart não finalizado, ordenação de
listagem ou leitura-after-write em todo provedor. `Capabilities()` precisa
explicitar precondições e limites. Reconciliação consome todos os cursores e
classifica objetos esperados ausentes, divergentes e remotos inesperados. Deve
recalcular o SHA-256 dos bytes recebidos; ETag e metadata SHA-256 são apenas
pistas, não prova.
Reconciliação é somente leitura: não exclui objetos, não os sobrescreve e não
avança watermark/acknowledgement. O chamador decide se um objeto ausente pode ser
reenviado a partir do spool local durável.

As capabilities reportadas pelos adapters indicam as operações/precondições que
eles usam e não constituem descoberta automática da implementação do endpoint.
O deployment deve confirmar que o provedor honra PUT condicional e
`If-None-Match` em multipart completion; se não honrar, o adapter não pode
degradar para overwrite. O limite de delete batch é 1000 chaves e o tamanho
máximo reportado de objeto S3 é 5 TiB. O adapter escreve o input em arquivo
temporário local antes do upload, usa buffers limitados e não tenta limpar
multipart incompleto após queda abrupta do processo; deployments devem definir
uma política de lifecycle no bucket para uploads multipart órfãos. O comportamento
do Floci é evidência de conformance local, não homologação de AWS S3, R2 ou
qualquer outro serviço.

## Filesystem local

Staging é criado sob o mesmo filesystem do destino. Bytes são escritos e
sincronizados antes da publicação. Publicação falha se a chave existe com bytes
diferentes; retry idêntico é idempotente. Sincronizar o diretório depois de
instalação/remoção. Filesystem ou VFS sem as primitivas requeridas falha
explicitamente; não degradar para cópia direta.

## Listagem e paginação

Consumidores seguem todos os cursores e deduplicam chaves. Não presumem ordenação
estável, nem que página vazia implica fim sem cursor terminal. Objetos são
validados individualmente antes de entrar em plano de restore/GC.

## Falhas e testes

Adapters testam zero bytes, objeto grande e multipart, cancelamento, timeout,
retry limitado pelo SDK, resposta inesperada, conflito de chave, checksum
divergente, 403/404/409/412/429/5xx, perda da resposta depois do PUT, paginação,
reconciliação e interrupção. Falhas de storage não removem o único caminho
recuperável nem avançam o acknowledgement. A política de retries precisa ser
configurada no SDK; evitar um segundo loop de retry no adapter para não
multiplicar tentativas.
