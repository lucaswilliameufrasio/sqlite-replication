# Runbook de disaster recovery

## Escopo operacional atual

Este runbook cobre o restore offline do snapshot LTX inicial para um caminho
novo em um repositório filesystem; o restore incremental público também reconstrói
uma cadeia local para um TXID exato ou instante UTC inclusivo. Go `replicator.CaptureOnce` e Rust
`capture_once` também expõem um ciclo síncrono de captura para spool local,
somente em Linux x86-64 com os runtimes SQLite testados e um único escritor
lógico coordenado. O chamador precisa pausar todos os escritores pelo write-gate
e manter o spool local; a função de captura não faz upload, checkpoint nem
restore e não expõe `Status`/`Health`/`Flush`. A garantia exclui perda física de
energia/cache. ARM64 e outras plataformas/configurações ficam fora até drills
CP-01 específicos.

Restore por TXID/instante é público para repositório filesystem local e via S3
compatível (testado apenas com Floci).
Go: `replicator.RestoreToTarget(ctx, root, destination, replicator.RestoreTarget{TXID: n})`
ou use `TimestampMillis` em vez de `TXID`. Rust:
`restore_to_target(root, destination, RestoreTarget::Txid(n))` ou
`AtOrBeforeTimestampMillis(ms)`. Para S3, Go `replicator.RestoreFromS3(...)` e
Rust `restore_from_s3(...).await` aceitam store/prefix e o mesmo alvo. O caminho
S3 foi exercitado somente com Floci; nenhum outro provedor foi homologado. O
destino precisa ser novo e é verificado antes da promoção. Compactação e GC continuam
implementados privadamente para desenvolvimento M4 em filesystem e S3 nos
engines Go/Rust, sem APIs/CLI operacionais nem procedimentos aprovados para
repositórios ativos. O
restore S3 baixa o repositório inteiro para staging filesystem temporário; o
uso de espaço local é proporcional ao repositório e provedores
externos não foram homologados.

## Sync explícito para S3

Go `replicator.SyncToS3(ctx, repositoryRoot, store, prefix, remoteID)` e Rust
`sync_to_s3(store, repositoryRoot, prefix, remoteID).await` validam e publicam
todos os LTX locais imutáveis e o manifesto por último. `remoteID` deve ser
estável, não secreto e escolhido pela aplicação para identificar a configuração
remota; credenciais/endpoints não são gravados no ACK. O ACK local só avança
depois da confirmação de todos os objetos. Retry aceita objetos idênticos e
rejeita conteúdo divergente. Essa operação não remove spool nem faz GC; não
executar sync concorrente para o mesmo repositório. Compatibilidade foi testada
somente com Floci, sem homologar AWS S3, R2 ou outros provedores.

## Restore via biblioteca

1. Interrompa novas escritas e preserve uma cópia somente-leitura do repositório
   e dos logs relevantes. Não execute GC nem remova objetos.
2. Escolha um caminho de destino que não exista, sem `-wal` ou `-shm` preexistente.
   O restore público não sobrescreve banco nem sidecar.
3. Chame a API pública de restore offline na ferramenta/aplicação operacional:
   Go `RestoreSnapshot(ctx, repositoryRoot, destinationPath)` ou Rust
   `restore_if_missing(repository_root, destination)` para o snapshot inicial;
   para um ponto incremental, use `RestoreToTarget`/`restore_to_target`. S3 pode
   ser restaurado via `RestoreFromS3`/`restore_from_s3`, que baixa o repositório
   inteiro para staging temporário local. Não use o exemplo em
   `docs/examples/snapshot-restore.md` como comando de desastre: ele primeiro
   publica um snapshot a partir de uma base-fonte e serve apenas como smoke test.
4. Verifique o banco restaurado com `quick_check` e `integrity_check`, valide
   contagens/hashes esperados e a consistência da aplicação antes de direcionar
   tráfego. Mantenha a cópia do repositório original até a validação terminar.
5. Reabra escrita em um único host/escritor lógico. Registre a identidade do
   repositório e a evidência da recuperação.

## Operações M4 ainda não liberadas

- Não há CLI para restore por TXID/timestamp nem ferramenta pública para listar
  restore points do S3.
- Não há CLI de restore autônoma; a recuperação M1 exige chamar a API da
  biblioteca a partir de uma ferramenta/aplicação.
- Não rode compactação ou GC por código interno/testes em repositórios ativos.
- Quando a integração pública for autorizada, primeiro gere e revise plano
  dry-run em cópia isolada; declare explicitamente cada ponto de restore que a
  retenção promete. Só então revalide e execute GC, mantendo backup recuperável.
- A retenção não possui defaults: idade, número de snapshots e horizonte fino
  devem ser definidos pelo operador após requisitos e medições, não inferidos.
