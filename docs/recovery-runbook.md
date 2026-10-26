# Runbook de disaster recovery

## Escopo operacional atual

Este runbook cobre somente o vertical público M1: restore offline do snapshot
LTX inicial para um caminho novo em um repositório filesystem. Restore por TXID/
instante, compactação e GC foram implementados privadamente para desenvolvimento
M4, mas não são APIs/CLI operacionais e não estão integrados ao S3. CP-01 segue
pendente; não habilite captura WAL nem anuncie recuperação incremental.

## Restore disponível (M1)

1. Interrompa novas escritas e preserve uma cópia somente-leitura do repositório
   e dos logs relevantes. Não execute GC nem remova objetos.
2. Escolha um caminho de destino que não exista, sem `-wal` ou `-shm` preexistente.
   O restore público não sobrescreve banco nem sidecar.
3. Chame a API pública de restore offline na ferramenta/aplicação operacional:
   Go `RestoreSnapshot(ctx, repositoryRoot, destinationPath)` ou Rust
   `restore_if_missing(repository_root, destination)`, respeitando a assinatura da versão
   instalada. O repositório deve estar acessível localmente; o adapter S3 ainda
   não está ligado ao restore. Não use o exemplo em
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
