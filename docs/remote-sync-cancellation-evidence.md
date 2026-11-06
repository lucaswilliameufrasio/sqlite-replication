# Cancelamento durante falha de sync remoto

Em 2026-11-05, o teste de integração
`cancellation_during_failed_remote_sync_keeps_local_snapshot_and_no_ack` foi
executado três vezes com sucesso em Linux x86-64, container Docker rootless sem
rede externa, usando Rust 1.99.1 e cliente S3 real contra um servidor HTTP
local controlado. O login shell era fish e o executor dos comandos, Bash.

O teste pausa a primeira requisição S3, sinaliza cancelamento e confirma que o
worker ainda não terminou após 100 ms. Libera a resposta HTTP 403 e verifica que
o worker retorna erro, a admissão permaneceu fechada nessa sequência, o arquivo
de acknowledgement remoto não existe e o snapshot local restaura o valor
confirmado. O cliente de teste desabilita retries e configura timeout de 10 s;
esses limites não são garantia para outras configurações da aplicação.

## Sync bem-sucedido após cancelamento

O teste opt-in `cancellation_during_successful_remote_sync_drains_and_confirms`
foi executado três vezes contra Floci 2.3.0 em um namespace rootless sem rede
externa. Um proxy local suspende a primeira conexão S3; o teste sinaliza o
cancelamento, verifica que o worker segue em execução e que ainda não existe
acknowledgement, e então libera o proxy. O worker termina com sucesso, a
admissão não foi reaberta nessa sequência e o acknowledgement registra TXID 1,
prefixo e identificador do destino. Um restore a partir do armazenamento Floci
recupera o valor confirmado. O teste remove os objetos e o bucket únicos após o
sucesso; falhas podem deixar recursos, portanto execute-o apenas com Floci
descartável isolado em `127.0.0.1:4566`:

```sh
cargo test --offline -p sqlite-replication-core --test remote_cancellation \
  cancellation_during_successful_remote_sync_drains_and_confirms -- --ignored --exact
```

Esses cenários não testam recuperação após resposta perdida, cancelamento
imediato da operação ou provedores reais. Floci não comprova compatibilidade
universal S3/R2. Não resolvem a corrida entre sinalização e reabertura da
admissão documentada na ADR 0011: não considerar M5 ou o gate completo aprovados.
