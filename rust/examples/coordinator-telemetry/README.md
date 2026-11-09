# Métricas opcionais de Flush em Rust

Este workspace de exemplo separado usa `opentelemetry` 0.33.1 e `prometheus`
0.14.0; os testes usam `opentelemetry_sdk` 0.33.1. As crates foram adicionadas
com `cargo add`, e `Cargo.lock` fixa o grafo do exemplo. Elas não foram
adicionadas às dependências do workspace principal. A aplicação fornece um
`prometheus::Registry` e um `opentelemetry::metrics::Meter` e é responsável pelo
ciclo de vida e exportação. Nenhum servidor HTTP é iniciado pelo exemplo.

```rust,ignore
let observer = Arc::new(FlushMetrics::new(&registry, &meter)?);
let coordinator = Coordinator::new(config)?.with_metrics(observer);
```

O callback produz histogramas em segundos para os dois backends com um único
atributo `outcome`. Adapte-o para um backend se ambos terminarem no mesmo
destino, evitando contagem duplicada. `returned_ok` significa apenas que a
future retornou `Ok`: pode incluir cancelamento observado e não garante que a
admissão reabriu. `returned_error` não distingue cancelamento de outras falhas;
uma future descartada não emite amostra. Esses rótulos não são semanticamente
equivalentes aos resultados do coordinator Go. O callback deve ser rápido,
thread-safe e não entrar em panic; não crie labels de paths, IDs, credenciais ou
mensagens de erro. O callback de métricas não cria correlação com traces.

Execute `cargo test --locked` e `cargo clippy --locked --all-targets -- -D
warnings` neste diretório. O teste executa o coordinator com um snapshot SQLite
real e adapter sintético, coleta Prometheus e OpenTelemetry em memória e
verifica uma amostra `returned_ok` e uma `returned_error` em ambos. O exporter
OpenTelemetry em memória é apenas um utilitário de teste.

Este workspace permanece separado do workspace Rust principal. O alvo
`make test-telemetry` na raiz verifica os exemplos Go e Rust, e integra o
`make test`; `make test-native` continua sem esse alvo. Seu
[inventário separado](../../../docs/sbom/2026-11-05/telemetry-examples/README.md)
é preliminar; suas dependências e notices exigem revisão antes de distribuição.
Não valida o gate de admissão de produção, o caso de cancelamento em voo, a
integração de rede ou serviços S3/R2, e não resolve a regressão conhecida.

## Trace explícito

`traced_flush(&coordinator, &tracer, &parent)` cria um span filho de um contexto
OpenTelemetry explícito e registra somente `returned_ok` ou `returned_error` e,
em caso de erro, uma descrição fixa. Não registra a mensagem do erro. A API do
coordinator Rust não aceita contexto de trace: este wrapper **não propaga** o
span para o adapter nem instrumenta os flushes internos de `Run`. A aplicação
controla tracer, sampler e exporter; o teste usa exporter em memória e verifica
pai, resultados, atributos e ausência de eventos. Um teste adicional interrompe
a future por descarte via `select!` e por `abort` da task enquanto uma captura
sintética está bloqueada. Com o SDK 0.33.1 e exporter de teste, o span terminou
com status `Unset`, sem atributo `outcome` nem evento, antes de a captura ser
liberada; `Unset` **não significa sucesso**. A captura `spawn_blocking`
continuou até o teste liberá-la. O teste verificou admissão fechada nessa
sequência, não a serialização de flushes posteriores nem a corrida de reabertura.
Não foi testada captura SQLite real nesse caso, nem correlação interna,
exportação externa ou outros exporters/SDKs.
