# Migração da toolchain Rust

O workspace passou a declarar Rust **1.94.1** como versão mínima, em vez de
1.85, devido às dependências atualizadas do AWS SDK. A linha de base usada no
desenvolvimento e no CI é **Rust/Cargo 1.99.1**, fixada em `.mise.toml`. A MSRV
declarada ainda não foi validada por uma execução do gate com Rust 1.94.1.

## Verificação parcial da MSRV — 2026-11-05

Em Linux x86-64, Rust `1.94.1 (e408947bf 2026-03-25)` e Cargo
`1.94.1 (29ea6fb6a 2026-03-24)` foram instalados em `RUSTUP_HOME` isolado,
sem alterar `.mise.toml` nem a toolchain padrão do usuário. A execução ocorreu
em container rootless sem rede, com checkout e cache de dependências somente
leitura, diretório de build separado e lockfile existente:

```sh
cargo test --offline --locked --workspace --all-features --no-run
cargo test --offline --locked --workspace --all-features --no-fail-fast
```

A compilação dos testes do workspace principal **passou**. Na execução, 83
testes passaram, oito foram ignorados e um falhou: a regressão preexistente
`cancellation_inside_admission_callback_never_reopens_writes`, com
`admission reopened after cancellation was signaled`. `--no-fail-fast`
permitiu executar os demais alvos; entre os ignorados estão os testes opt-in de
Floci e drills que exigem infraestrutura própria. A falha é a mesma já
observada com 1.99.1, não evidência de incompatibilidade específica de MSRV.

O workspace separado `rust/examples/coordinator-telemetry` também foi compilado
e testado offline com Rust/Cargo 1.94.1 e seu próprio lockfile, em container
rootless sem rede e com diretório de build separado:

```sh
cargo test --offline --locked --no-run
cargo test --offline --locked
```

Os quatro testes de integração de métricas, traces e descarte de future passaram;
nenhum foi ignorado. Eles usam adapters sintéticos e não homologam adapters de
produção ou exportação remota. Isso comprova resultados parciais para o checkout
e lockfiles examinados, não aprovação da MSRV: o gate integral e testes opt-in
permanecem pendentes. A regressão e a proposta de contrato de admissão seguem
abertas.

### Floci opt-in com 1.94.1

Em container Floci 2.3.0 descartável, rootless e sem rede externa, usando o
mesmo target de build isolado e `--offline --locked`, foram executados:

- `cargo test -p sqlite-replication-storage-s3 tests::floci_adapter_conformance -- --ignored --exact`:
  **passou**. Sem as variáveis de fixture compartilhada, não executou o trecho
  de interoperabilidade Go/Rust ou o teste de restart do smoke completo.
- `cargo test -p sqlite-replication-core recovery::tests::floci_s3_recovery_restore_compaction_and_gc -- --ignored --exact`:
  **falhou** ao final, ao excluir o bucket isolado: Floci retornou
  `BucketNotEmpty` (HTTP 409). As verificações anteriores, inclusive restores
  após GC, foram alcançadas, mas o teste inteiro não passou. Investigação
  posterior reproduziu o erro e identificou três objetos em
  `public-rust-sync-test/`: a limpeza percorria apenas `recovery-test/`.
  A limpeza do teste foi corrigida para ambos os prefixos no bucket isolado;
  o mesmo teste passou duas vezes com IDs distintos e Rust 1.94.1. Isso era uma
  falha de limpeza do teste, não uma incompatibilidade demonstrada da toolchain.
- `cargo test -p sqlite-replication-core --test remote_cancellation cancellation_during_successful_remote_sync_drains_and_confirms -- --ignored --exact`:
  **passou**, com proxy de teste e restore remoto no Floci descartável.

O container de teste foi encerrado após a execução. Isso não é `make test-floci`
completo, não cobre o smoke entre linguagens nem os drills ENOSPC/FUSE/QEMU e
não estabelece compatibilidade universal S3/R2. O gate e a MSRV continuam sem
aprovação integral; a regressão de admissão permanece independente desta falha.

### Gate após correção da limpeza

Em 2026-11-05, `make test` foi executado com Go 1.27.3 e Rust 1.99.1 em
container rootless offline, com `GOFLAGS=-count=1`: formatação, `go vet`, Clippy
e testes Go sem cache passaram. Os unitários Rust do core passaram (20 passaram,
4 ignorados), mas o alvo `test-rust` interrompeu o Make na regressão conhecida
`cancellation_inside_admission_callback_never_reopens_writes` (7 passaram,
1 falhou nesse binário). Portanto, o gate completo não passou e não alcançou
fuzz, conformance ou Floci; o container offline não foi configurado para
executar o Docker necessário ao alvo Floci.

Separadamente, no mesmo tipo de isolamento, `make test-fuzz test-conformance`
passou: três campanhas curtas de fuzz Go e matriz de conformance com 16 casos e
96 restores. Isso não substitui `make test` completo nem os testes opt-in de
infraestrutura ou a validação de adapters de produção.

### Smoke Floci completo após correção

Em 2026-11-05, `make test-floci` passou com Go 1.27.3 e Rust 1.99.1,
dependências de linguagem em modo offline e ambiente de aplicação reduzido. O
harness usou Docker rootless, projeto Compose e volume exclusivos, endpoint
publicado somente em loopback e imagem Floci 2.3.0 fixada por digest. O Compose
executou `--pull always`, portanto não foi uma execução inteiramente offline.
O teste verificou adapters e recuperação Go/Rust, intercâmbio entre linguagens,
sync/restore públicos e persistência após SIGKILL e reinício do Floci. O harness
reportou sucesso e remoção do container, volume e rede próprios.

Isso valida somente o ambiente e cenários do smoke: não prova compatibilidade
universal S3/R2, segurança de gates de escrita fornecidos pela aplicação ou
durabilidade contra perda física de energia/cache. Não é uma execução completa
do gate `make test`.

Em uma execução posterior de `make test-floci` com Go 1.27.3 e a toolchain
isolada Rust/Cargo 1.94.1, o smoke completo também **passou**, incluindo as
etapas de interoperabilidade entre Go e Rust e a persistência após SIGKILL e
reinício do Floci. O harness reportou a remoção do container, rede e volume
exclusivos. O target de compilação Rust era separado; Cargo e Go foram
configurados offline, mas o Compose fez pull da imagem por digest. Não se
alteraram os pins do projeto. O gate integral e a regressão de admissão seguem
pendentes; esse smoke não homologa adapters de admissão de produção nem amplia
as garantias de provedor ou durabilidade física.

### Drills CP-01 independentes

Em 2026-11-05, com Go 1.27.3 e Rust 1.99.1, `make
test-cp01-fsync-interrupt` passou para Go e Rust: os processos de publicação
foram interrompidos enquanto `fsync` estava bloqueado no FUSE de teste, e a
recuperação/retry foi verificada. `make test-cp01-qemu-poweroff` também passou:
após publicação, verificação e sync no guest, o processo QEMU foi encerrado
abruptamente e ambos os repositórios foram recuperados em nova inicialização,
com ext4 sobre virtio-blk e KVM. Foram usados somente os diretórios temporários
e processos criados pelos próprios scripts, com ambiente reduzido e dependências
de linguagem offline. Esses drills não comprovam perda física de energia/cache.

O drill `make test-cp01-enospc` inicialmente não foi executado porque sua imagem
Debian fixada não estava em cache e o próprio script não faz pull. Em execução
posterior, a imagem foi obtida explicitamente no Docker rootless pelo digest
`sha256:62bd423aeee912639ca8ee681c2b762cb3e25137e0a92b20be455898a9f034ad`;
`make test-cp01-enospc` passou em Go e Rust 1.99.1. O harness executou os testes
em container sem rede, filesystem somente leitura e tmpfs de 8 MiB, induzindo
ENOSPC real e verificando recuperação da captura. O download da imagem usou
rede; a execução do teste não. Não foram alterados privilégios ou configuração
do host. A imagem permanece no cache Docker rootless.

O mesmo `make test-cp01-enospc` foi executado posteriormente com Rust/Cargo
1.94.1 isolado e Go 1.27.3: os testes de captura Go e Rust passaram novamente,
com o mesmo tmpfs limitado de 8 MiB e container rootless sem rede. O target
Rust era separado e as dependências de linguagem foram usadas offline. Isso
valida esse drill sob 1.94.1.

Em execução posterior, `make test-cp01-fsync-interrupt` e `make
test-cp01-qemu-poweroff` também passaram com Go 1.27.3 e Rust/Cargo 1.94.1.
No FUSE, ambos os testes recuperaram após interrupção de publicação durante o
`fsync` bloqueado. No QEMU/KVM, após o guest sincronizar e validar as capturas,
o processo QEMU foi encerrado e a recuperação em nova inicialização passou em
ext4 sobre virtio-blk. Os scripts usaram processos e diretórios temporários
próprios; as dependências de linguagem foram usadas offline. A execução QEMU
usa seu próprio target temporário; o FUSE usou target separado para a MSRV.

O gate integral e a validação de adapters de produção permanecem abertos;
esses resultados não comprovam perda física de energia ou cache.

Antes de atualizar a dependência desta biblioteca, instale uma toolchain
compatível, atualize o lockfile do projeto consumidor e execute os testes da
aplicação. Compiladores anteriores a 1.94.1 deixam de ser suportados. Projetos
que usam `mise` podem usar os pins do repositório como referência.

O adapter S3 usa o cliente HTTPS padrão moderno do AWS SDK, Tokio, HTTP 1.x e
SigV4a. A feature `rustls` do SDK, que ativa também o conector legado, não é
selecionada diretamente por este workspace. Features são unificadas pelo Cargo:
outra dependência ou o próprio consumidor ainda podem reativá-la. Audite a
árvore e o lockfile finais da aplicação; um resultado limpo neste repositório
não garante que toda árvore consumidora esteja livre de advisories. A presença
de certificados raiz válidos no ambiente continua necessária para HTTPS.

O teste Floci é local e não homologa AWS S3, Cloudflare R2 ou outros provedores.
As garantias de captura WAL permanecem restritas às combinações Linux x86-64,
Go/Rust/SQLite verificadas pelos drills CP-01; esta migração não amplia esse
escopo nem comprova proteção contra perda física de energia ou cache.
