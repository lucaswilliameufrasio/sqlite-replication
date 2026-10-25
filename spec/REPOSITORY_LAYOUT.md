# Layout e identidade do repositório

## Identidade

`<root>/_meta/repository.json` contém JSON versionado com `schema_version`,
`format_version`, `repository_id` e `database_id`. UUIDs são gerados uma vez e
não são inferidos de caminho, nome de arquivo ou conteúdo. O primeiro perfil
define `format_version: 2`; LTX v2/v3 nunca é inferido de `LTX1`.

No schema 1, ambas as identidades usam UUID v4 com variante RFC 4122 e forma
hifenizada de 36 caracteres (hexadecimal pode ser maiúsculo ou minúsculo).
UUID nulo, outra versão/variante, forma compacta, URN ou chaves externas são
rejeitados igualmente pelos dois motores. Metadados inválidos não são
reinicializados silenciosamente nem substituídos por identidades novas.

O metadata ajuda validação, mas não é índice mutável autoritativo: recuperação
enumera objetos imutáveis e valida suas posições, identidade e cadeia. Ausência,
versão não suportada, campo desconhecido ou JSON excedente deve falhar
explicitamente.

## Filesystem

```text
<root>/
  _meta/repository.json
  .staging/                 # temporários locais; não recuperáveis
  ltx/<level decimal>/<min_txid 16 hex>-<max_txid 16 hex>.ltx
```

M1 publica `<root>/ltx/0/0000000000000001-0000000000000001.ltx`. Staging fica
no mesmo filesystem do destino para permitir publicação atômica sem overwrite.
O writer sincroniza os bytes, instala uma chave imutável sem substituir a
existente e sincroniza diretório(s). Retry só é idempotente se os bytes
completos forem iguais; conteúdo divergente na mesma chave é erro crítico.

## S3-compatible

O perfil remoto mantém namespace próprio:

```text
<prefix>/_meta/repository.json
<prefix>/<level hexadecimal com 4 dígitos>/<min_txid>-<max_txid>.ltx
```

O layout S3 não copia o diretório `ltx/` do filesystem. Objetos são imutáveis;
usar precondição create-only quando suportada e confirmar existência/conteúdo
por checksum independente de ETag. Multipart, listagem paginada e semântica do
provedor são definidos por `STORAGE_CONTRACT.md`.

## Posição e geração

TXIDs são `uint64`, com intervalos inclusivos. Uma cadeia incremental liga
`min_txid` ao `max_txid` anterior sem lacuna e verifica checksum pre/post,
`database_id` e `epoch`. Rebootstrap intencional cria uma nova geração explícita;
não mascara lacunas, forks nem reset inesperado do WAL. Checkpoint sequence,
WAL salt e offsets são metadados de captura, não substitutos do TXID da cadeia.

## Nomes e caminhos

Segmentos externos devem ser normalizados/validados antes de compor chaves. O
adapter não aceita separadores, traversal, chaves absolutas ou nomes que
escapem do prefixo configurado. Não seguir symlinks de staging como se fossem
objetos confiáveis. Um backend não deve usar listagem eventual como prova única
de que um upload foi confirmado.
