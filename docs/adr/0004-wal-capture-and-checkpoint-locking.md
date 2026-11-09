# ADR 0004: captura WAL e política de checkpoint

- **Status:** protocolo operacional aprovado para Linux x86-64 no escopo restrito; expansão de matriz não aprovada
- **Data:** 2026-10-23

## Contexto

Ler o arquivo `-wal` em paralelo com SQLite não impede checkpoint, reset e reuse.
Um reader pin é necessário ao desenho, mas sozinho não prova que commits não
serão perdidos nas transições de lock.

## Decisão

Implementar parser WAL próprio conforme `spec/WAL.md`; coordenar checkpoint pelo
adapter SQLite, não fazer tail/cópia simples. Não conectar captura incremental a
API pública fora da matriz aprovada; a sequência de locks, injeção determinística
e falhas exigidas em `spec/CHECKPOINT_PROTOCOL.md` passaram para o escopo Linux
x86-64 delimitado em 2026-10-27. Não extrapolar essa aprovação a outras
plataformas ou configurações.
Falha de spool, reader pin, `SQLITE_BUSY`, conexão externa ou versão/driver não
homologado bloqueia checkpoint e degrada/falha explicitamente.

## Alternativas rejeitadas

- Copiar `-wal` periodicamente sem proteção: reset concorrente pode criar lacuna.
- Tratar read transaction/pin como prova completa: não demonstra protocolo de
  liberação, checkpoint e nova fixação.
- Considerar ausência de erro em `sqlite3_wal_checkpoint_v2` prova de que todo
  WAL elegível foi backfilled.

## Consequências e evidência

M1 continua snapshot-only até a implementação da superfície pública aprovada.
Implementações internas Go/Rust agora cobrem WAL→LTX, spool durável, watermark,
recuperação e leases SQLite; testes locais cobrem locks, reset/reconciliação e
crashes de processo. A matriz e suas lacunas atuais estão em
`spec/CHECKPOINT_PROTOCOL.md`. CP-01 está aprovado somente na combinação
homologada; conectar a API e completar gates M5 continuam trabalho pendente. O
motor não solicita checkpoint destrutivo durante a captura.

## Reversão / reconsideração

Se o adapter SQLite não oferecer locks/conexões comprováveis para a matriz
suportada, remover a captura incremental daquele adapter e manter apenas
snapshot/restore. Não relaxar o gate; qualquer mudança de driver/VFS/OS exige
nova prova.
