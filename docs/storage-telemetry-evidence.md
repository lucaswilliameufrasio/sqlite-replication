# Evidência parcial de telemetria do adapter S3

Em 2026-11-05, `TestStorageMetricsAcrossLocalHTTPBoundary` executou o adapter
Go e o cliente AWS reais contra um servidor HTTP local controlado, em container
rootless sem rede externa. O teste passou em três execuções (`go test -count=3
-run '^TestStorageMetricsAcrossLocalHTTPBoundary$' ./storages3`); também passou
`go test -race -count=1 ./storages3`. O shell de login era fish e os comandos
foram executados em Bash; o servidor usou apenas credenciais sintéticas.

Foram verificados: GET de cinco bytes com observação `get`/sucesso/5 bytes;
HEAD com HTTP 403 e observação `head`/erro permanente; GET pré-cancelado e
observação `get`/cancelamento. O erro HEAD não incluiu o nome sintético do
bucket nem a chave sintética. As observações contêm apenas operação, duração,
bytes e classe de erro; o teste verifica os valores nas sequências executadas.
Isso não prova ausência de informações sensíveis em todos os logs, traces,
erros ou integrações possíveis.

## Adapter Rust

O teste `metrics_report_success_and_forbidden_across_local_http` exercitou o
cliente AWS e adapter S3 Rust reais contra servidor HTTP local, em container
rootless sem rede externa e com credenciais sintéticas. Em três execuções,
registrou `get` com cinco bytes e sucesso, e `head` com HTTP 403, zero bytes e
erro permanente; a mensagem de erro não incluiu bucket ou chave sintéticos.
`cargo clippy --offline -p sqlite-replication-storage-s3 --test metrics_http --
-D warnings` passou. `cargo test --offline -p sqlite-replication-storage-s3`
passou com nove testes unitários e o novo teste de integração; o teste Floci
opt-in permaneceu ignorado. Isso não testa cancelamento ou leitura posterior do
corpo com erro, nem todos os caminhos de storage ou vazamentos possíveis.

## Coordinator Go

`CoordinatorConfig.Metrics` é opcional e observa `Flush` com duração e uma das
três categorias fixas (`success`, `failure`, `canceled`), sem erros, paths ou
identificadores. A duração inclui espera pelo lock; o callback síncrono roda
após liberar o lock, deve ser rápido, seguro para chamadas concorrentes e não
entrar em panic. Cancelamento tem precedência se combinado com outro erro. A
adição de um campo público à configuração pode afetar literais Go sem nomes de
campos. Não foram adicionadas dependências nem servidor HTTP.

O teste `TestCoordinatorFlushMetricsOutcomesAndReentrancy` passou três vezes e
`go test -race -count=1 ./replicator` passou. Cobre sucesso, falha, cancelamento
prévio e callback que invoca outro `Flush`, com snapshot SQLite real e adapter
de teste. Não valida OTEL, Prometheus, traces, Rust, ou adapters de escrita de
produção. Após a mudança, `make test-native` passou por formatação, `go vet`,
Clippy e testes Go, mas falhou no teste Rust conhecido
`cancellation_inside_admission_callback_never_reopens_writes` com `admission
reopened after cancellation was signaled`; etapas posteriores não rodaram. O
comando literal `make test` não foi executado.

## Coordinator Rust

`Coordinator::with_metrics` instala opcionalmente um observador síncrono de
`Flush`, chamado depois de liberar o lock, com duração incluindo a espera e
resultado limitado a `ReturnedOk` ou `ReturnedError`. `ReturnedOk` descreve
apenas o retorno da future: pode ocorrer após o worker observar cancelamento e
não significa que a admissão reabriu. Uma future descartada antes do retorno
não produz observação. O callback deve ser rápido, seguro para concorrência e
não entrar em panic; não recebe erros, caminhos ou identificadores.

`cargo test --offline -p sqlite-replication-core --test coordinator_metrics`
passou três vezes com snapshot real, adapter de teste, sucesso, erro e callback
que aguarda outro flush; Clippy do mesmo teste passou com `-D warnings`. Após
essa mudança, `make test-native` passou formatação, `go vet`, Clippy e os testes
Go (em cache), mas voltou a falhar na regressão Rust de admissão descrita acima;
as etapas posteriores não rodaram. `make test` literal não foi executado.

## Exemplo OpenTelemetry e Prometheus Go

O módulo isolado `go/examples/coordinator-telemetry` demonstra histogramas dos
dois SDKs sem iniciar servidor, com um registry Prometheus e um leitor
OpenTelemetry em memória. O teste executa `Flush` real com snapshot SQLite e
adapter sintético e verifica as séries de sucesso, falha e cancelamento nos
dois registradores. No checkout, `go vet ./...`, `go test -race -count=3 ./...`
e `go mod tidy -diff` passaram no diretório do exemplo, offline, em container
rootless. As dependências novas são do módulo de exemplo e não constam dos
SBOMs anteriores; ainda exigem revisão antes de distribuição. A parte de métricas
não deve exportar os dois histogramas ao mesmo destino sem tratar duplicidade.

## Exemplo OpenTelemetry e Prometheus Rust

O workspace isolado `rust/examples/coordinator-telemetry` usa `opentelemetry`
0.33.1, `prometheus` 0.14.0 e, no teste, `opentelemetry_sdk` 0.33.1. As crates
foram adicionadas com `cargo add`; não alteram o workspace principal. O teste
utiliza snapshot SQLite real, adapter sintético e registradores em memória, e
verifica uma observação `returned_ok` e uma `returned_error` em ambos os
backends. `cargo fmt --check`, `cargo clippy --offline --locked --all-targets --
-D warnings` e três execuções de `cargo test --offline --locked` passaram em
container rootless sem rede externa. Este workspace e as dependências novas não
fazem parte do gate ou SBOMs anteriores. Não foram testados cancelamento
em voo, exportação de rede ou adapters de produção; `returned_ok` não equivale
ao resultado `success` de Go.

## Exemplos de trace explícito

Os dois exemplos agora possuem wrappers explícitos de `Flush` com spans
OpenTelemetry e testes de exportação em memória, sem dados de erro ou caminhos
em atributos. Go propaga o contexto do span para o callback de observação do
adapter e verifica sucesso, falha e cancelamento com `go test -race -count=3`.
Rust verifica pai e resultados `returned_ok`/`returned_error` em três execuções
de `cargo test --offline --locked` e passou Clippy com `-D warnings`. O
coordinator Rust não recebe contexto e o wrapper não o propaga aos adapters.
Nenhum dos wrappers intercepta `Run`; exportação externa, spans internos e
privacidade de outros componentes não foram validados.

Os testes Rust `trace_drop` exercitaram descarte via `select!` e `abort` da task
enquanto uma captura sintética executada com `spawn_blocking` aguardava liberação.
Com SDK 0.33.1 e exporter em memória, o span foi finalizado com status `Unset`,
sem atributo de resultado, antes da captura bloqueante terminar. A admissão ficou
fechada na sequência testada e a captura continuou até ser liberada; isso não
prova serialização de futuras capturas, captura SQLite real ou solução da
corrida de admissão. `cargo fmt --check`, Clippy com `-D warnings` e três
execuções `cargo test --offline --locked` passaram no workspace do exemplo,
em container rootless sem rede externa.

Um teste posterior reproduziu sobreposição de duas capturas sintéticas após
descartar a primeira future; detalhes em `coordinator-dropped-flush-evidence.md`.

Os testes acima cobrem apenas os cenários descritos. M5 ainda exige adapters de
admissão de produção e instrumentação dos demais fluxos, incluindo traces e
exportação OpenTelemetry/Prometheus. Não homologam
fornecedores S3/R2 nem resolvem a corrida de cancelamento/admissão.
