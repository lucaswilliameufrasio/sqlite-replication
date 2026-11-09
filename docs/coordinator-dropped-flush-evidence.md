# Sobreposição após descarte de Flush Rust

**Risco aberto; teste de caracterização, não garantia de segurança.** Em
2026-11-05, `second_flush_can_capture_while_dropped_flush_capture_is_running`
reproduziu três vezes a seguinte sequência com o coordinator Rust real,
snapshot SQLite real e adapter de captura sintético:

1. O primeiro `Flush` inicia `capture()` em `spawn_blocking` e a captura aguarda
   uma barreira controlada pelo teste.
2. A task que aguardava o primeiro `Flush` é abortada; a captura bloqueante
   permanece aguardando a barreira.
3. Um segundo `Flush` alcança uma segunda chamada de `capture()` **antes** de
   a barreira ser liberada. Nenhuma das duas capturas terminou antes da
   liberação; após liberadas, ambas retornam erro sintético.
4. Nessa sequência, as duas transições de admissão registradas são `false` e o
   snapshot local inicial é restaurável com o valor esperado.

O resultado é compatível com a posse do lock na future de `Flush`: descartá-la
solta o guard, mas não interrompe o trabalho `spawn_blocking` já iniciado. Não
foi demonstrada corrupção nem execução simultânea do adapter real SQLite; o
teste mostra que o coordinator, sozinho, não mantém a exclusão sobre a captura
bloqueante após o descarte. A existência de write-gates ou locks específicos do
adapter precisa ser avaliada separadamente. O cancelamento em voo, manutenção
de durabilidade e sincronização de admissão continuam pendentes.

## Lease SQLite real

Um segundo teste, em `rust/crates/core/src/capture_overlap_test.rs`, usa o
coordinator real e a implementação interna de captura SQLite real, com hook de
teste que pausa a primeira captura **após** adquirir a lease. O banco permanece
aberto em WAL e contém uma alteração posterior ao snapshot. Após abortar a
primeira future, o segundo `Flush` chega à captura mas retorna `SQLITE_BUSY`
enquanto a primeira ainda detém `BEGIN IMMEDIATE` (`busy_timeout=0`). Depois da
liberação, a primeira captura publica o TXID 2 e o restore desse TXID recupera
a alteração. As duas transições de admissão observadas foram `false`. O teste
passou três repetições; `cargo fmt --check` e Clippy dos testes do core passaram.

O hook exercita diretamente a captura interna para poder pausar dentro da lease:
**não passa pelo `WriteGate` público da aplicação**. O gate é uma trait
fornecida pelo chamador; a inspeção encontrou implementações de teste e de
conformance, não um gate de produção homologado. Assim, a lease impediu a
sobreposição da região protegida para o mesmo banco e esse cenário Linux x86-64,
mas não prova o comportamento de gates reais, o que acontece antes de adquirir
a lease, reabertura da admissão, retries ou recuperação em todos os pontos de
falha. Não transforma a sobreposição de chamadas de captura em garantia segura.

Após acrescentar o teste da lease real, `make test-native` voltou a passar
formatação, `go vet`, Clippy e os testes Go em cache; os unitários Rust do core
incluíram o novo teste com sucesso (20 passaram, 4 opt-in ignorados). O gate
parou depois na regressão conhecida de admissão; as etapas restantes não
rodaram e `make test` literal não foi executado.

O teste passou três vezes em container Docker rootless sem rede externa com
`cargo test --offline -p sqlite-replication-core --test coordinator_dropped_flush`;
`cargo fmt --check` e Clippy do teste com `-D warnings` passaram. Ele afirma o
comportamento atual perigoso para tornar a descoberta reproduzível: se a
implementação for corrigida para impedir a sobreposição, o teste deve ser
atualizado para afirmar o novo contrato seguro. Não marcar o gate nem M5 como
aprovados por esse teste passar. Não altera a proposta de contrato de admissão
ou o teste de regressão já existente.

Após adicionar o teste, `make test-native` passou formatação, `go vet`, Clippy
e testes Go em cache, mas parou no teste de regressão de admissão conhecido,
com `admission reopened after cancellation was signaled`. O teste de
caracterização acima foi executado separadamente; o gate interrompido não
alcançou as etapas seguintes. `make test` literal não foi executado.
