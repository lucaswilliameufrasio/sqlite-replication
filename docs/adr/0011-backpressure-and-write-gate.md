# ADR 0011: limites de trabalho e política de write-gate

- **Status:** decisões de contrato aceitas; coordenador explícito implementado em Go/Rust; homologação de adapters, stress/cancelamento e telemetria pendentes
- **Data:** 2026-10-23

> A decisão posterior [ADR 0015](0015-flush-barrier.md) substitui a semântica
> de alvo amostrado de `Flush` descrita abaixo. Este documento preserva o
> histórico; a barreira aprovada ainda precisa ser implementada e homologada.

## Contexto

Storage lento/indisponível pode crescer WAL e spool sem limite. Concurrency de
worker, itens em espera e bytes duráveis são limites diferentes. Uma fila em
memória não torna trabalho durável.

## Proposta

### Política de admissão decidida pelo mantenedor — 2026-10-27

- O modo **estrito é o padrão** da futura API integrada.
- Modo degradado permanece opt-in explícito e precisa sinalizar que o RPO pode
  ficar ilimitado; não promete bloquear escritores externos ou conexões fora do
  adapter controlado.
- Inicialização estrita falha se o adapter/callback controlado pela aplicação
  não puder impor a política de admissão.

Essa escolha fixa somente a política de modo; não aprova ainda as assinaturas,
os pontos de reserva/liberação de capacidade, os limites numéricos ou a
semântica de aceitação/cancelamento descritos abaixo.

### Base de contrato aprovada para implementação — 2026-10-27

O mantenedor aprovou esta base para implementação. Isso não congela os detalhes
listados como pendentes abaixo. Captura WAL pode ser integrada somente no escopo
Linux x86-64 aprovado em CP-01, após os gates de implementação e API; não
anunciar cobertura mais ampla.

- `Status` expõe separadamente o progresso que o adapter consegue observar,
  aquele que já está protegido em spool durável e o último TXID confirmado no
  remoto. Valores indisponíveis devem ser representados explicitamente como
  desconhecidos/ausentes, nunca inferidos como zero ou sucesso. Backlog staged,
  queued e in-flight não se confundem.
- `Health` é um retrato tipado dos componentes locais/remotos e de falhas
  categorizadas; não inicia listener HTTP, não implica frescor/RPO aceitável e
  não transforma um remoto indisponível em falha de liveness do processo. A
  aplicação decide como mapear o resultado para readiness/liveness.
- `Flush` captura como alvo o maior TXID já observado pelo adapter quando a
  chamada começa. No modo local, sucesso só ocorre após esse alvo estar em spool
  durável; no modo remoto, sucesso só ocorre após confirmação remota desse alvo.
  Um deadline/cancelamento encerra a espera e retorna erro, mas não desfaz
  trabalho já durável nem deve sugerir que uma publicação em andamento foi
  revertida; `Status`/reconciliação informam o resultado posterior.
- A observabilidade é injetada pela aplicação, sem servidor embutido, labels de
  alta cardinalidade ou dados de aplicação/credenciais. OpenTelemetry e métricas
  Prometheus seguem os guardrails da ADR 0012; os nomes/convenções e adapters
  concretos ainda precisam de revisão.

Pontos ainda necessários antes de declarar a API estável: testes adicionais da
semântica de `Flush` sob concorrência/cancelamento, compatibilidade de drivers e
adapters reais que conseguem observar e impor admissão, e telemetria injetável.
Mudanças necessárias encontradas durante testes devem voltar para revisão do
contrato.

Em 2026-10-27, o mantenedor confirmou a semântica proposta de alvo de `Flush`,
confirmação local/remota e timeout/cancelamento. A API e suas implementações
continuam pendentes de testes; a confirmação não substitui os gates de M5.

### Fechamento de decisões de integração — 2026-10-28

O mantenedor decidiu:

- `Flush` será oferecido por um coordenador explícito, sem worker/listener
  obrigatório. O adapter injetado reporta o maior TXID observado e executa
  captura durável; o coordenador captura até alcançar o alvo amostrado no começo
  da chamada e, no modo remoto, sincroniza até esse mesmo alvo. Chamadas Flush
  concorrentes no mesmo coordenador são serializadas. Cancelamento/deadline só
  interrompem a espera/operação, nunca revertem publicação já durável.
- `Health` retorna estados separados por componente, sem status global que a
  biblioteca use como liveness/readiness. Falha remota é componente remoto
  degradado; erro local irrecuperável requer intervenção. Ausência de probe é
  `unknown`, não saudável. Não se infere degradação apenas de atraso/RPO sem
  limiar configurado.
- Limites de spool, WAL, idade máxima sem proteção e uploads em voo serão
  configuração obrigatória da API integrada, com validação estrita e sem
  defaults numéricos implícitos. Os valores da configuração devem ser usados
  para aplicar admission/backpressure, não somente reportados.
- O adapter também deve fornecer um admission controller obrigatório no modo
  estrito. O coordenador fecha a admissão e a mantém fechada enquanto a
  capacidade/segurança estiver excedida ou a captura/sync falhar; reabre somente
  após verificar que os limites foram novamente satisfeitos. Inicialização
  estrita falha sem esse contrato de controle. A política e os limites
  configurados são entregues ao callback de admissão, que deve reservar
  capacidade atomicamente antes de aceitar novas escritas e manter a admissão
  fechada se falhar ao habilitá-la.
- O trabalho contínuo será iniciado explicitamente pela aplicação via `Run`,
  sem intervalo de polling fixo. O adapter fornece notificações coalescíveis de
  commits/estado de capacidade; o worker observa o TXID mais recente e processa
  até esse alvo. O admission controller continua responsável por impedir
  atomicamente novas escritas quando não há capacidade, inclusive antes de um
  limite rígido ser ultrapassado. A aplicação deve iniciar fechada e só abrir
  admissão após o primeiro `Flush`/reconciliação bem-sucedido.
- Enquanto houver TXID observado além do último ponto local durável,
  `max_unprotected_age` é medido conservadoramente desde o timestamp do último
  LTX/snapshot durável. Isso pode bloquear antes da idade real do commit, mas não
  zera o prazo após restart nem exige timestamp de commit do adapter.

Permanecem ajustes de nomes/campos públicos e categorias de erro, além dos
adapters de métricas/traces da ADR 0012. Implementação não deve declarar o gate
M5 fechado até validação dos adapters, testes de concorrência, cancelamento,
saturação e execução dos gates correspondentes.

Definir limites separados para `max_spool_bytes`, `max_wal_bytes`, idade máxima
sem proteção, workers e bytes em voo. Rejeitar ou aplicar backpressure antes de
aceitar trabalho fora dos limites; work que precise sobreviver a crash só é
reconhecido após spool durável. Expor backlog queued/staged separadamente de
operações in-flight, erros, retries, saturação e latência com labels de baixa
cardinalidade.

Modo estrito exige callback de admissão de escrita controlado pela aplicação e
falha na inicialização se o adapter não conseguir impor a política. Modo
degradado é opt-in, informa risco/RPO possivelmente ilimitado e não promete
bloquear conexões externas. Limites iniciais de 1 GiB de spool, 64 MiB de WAL,
70/85% de alerta e concurrency 2 são hipóteses do plano, não defaults
estabelecidos.

## Alternativas rejeitadas

- Queue ilimitada ou task-per-upload: backlog/memória podem crescer sem bound.
- Concurrency limitada sem limitar espera/bytes: deixa outra fila sem bound.
- Write-gate implícito em conexão que o motor não controla: não cobre escritores
  externos/pools diferentes.
- Declarar queue volátil como durável: crash perde trabalho aceito.

## Consequências e evidência pendente

Mapear produtor → admissão → captura → spool → storage → ACK continua necessário
para cada adapter suportado. Testar fila cheia, storage lento, cancelamento,
erros, recuperação de quotas e isolamento por banco. A implementação atual tem
testes locais de contrato, mas ainda não demonstra saturação/stress com adapters
de produção.

## Reversão / reconsideração

Não remover limites nem mudar comportamento estrito/degradado sem novo contrato
e testes de saturação/cancelamento. Ajustar números após workload/benchmark
mensurado; limites de segurança não podem ser desativados silenciosamente.

## Proposta para revisão: cancelamento e admissão Rust — 2026-11-05

**Status: direção aprovada pelo usuário em 2026-11-06; implementação e validação
integrais pendentes.** O contrato atual entrega um
`watch::Sender` à aplicação, independente de `CoordinatorAdapter::set_write_admission`.
O teste `cancellation_inside_admission_callback_never_reopens_writes` demonstra
que o callback pode sinalizar cancelamento e depois abrir a admissão: o
coordenador não controla as instruções dentro desse callback. Consultar `watch`
antes ou depois não torna essas ações atômicas; segurar um borrow do `watch`
durante o callback pode bloquear indefinidamente um envio reentrante.

Uma solução estrita precisa que a aplicação coopere no ponto efetivo de aceitação
de cada escrita: o sinal de cancelamento e a decisão de aceitar a escrita devem
compartilhar um estado sincronizado, monotônico e controlado por uma única API.
Abrir o gate externo não basta se o escritor puder ignorar esse estado. A API de
cancelamento teria de mudar ou exigir que o adapter e todos os produtores de
escrita adotem o mesmo protocolo; um `watch::Sender` independente não pode
garantir a ordem atômica. O protocolo deve definir o ponto de linearização de
cancelamento versus escrita, falhar fechado, evitar callback de usuário sob lock
reentrante e não alegar interrupção de `spawn_blocking`. O teste atual é também um
adapter deliberadamente não cooperativo: nenhuma implementação do coordenador
pode impedir a instrução de abertura executada por esse callback depois de ele
próprio cancelar, se continuar invocando esse callback sob o contrato atual.

Alternativa sem mudança de API: fechar admissão novamente após observar
cancelamento, aceitando uma janela em que escritas podem passar. Isso não cumpre
o requisito estrito e não deve ser apresentado como solução equivalente. Requer
revisão do contrato antes de alterar a API ou suas garantias; testes devem cobrir
concorrência real, reentrada, timeout, encerramento e os adapters efetivos.

### Implementação em andamento

O usuário aprovou o protocolo estrito e a preparação de distribuição do
workspace Rust completo, sem autorizar publicação. `AdmissionGate` introduz um
estado compartilhado inicialmente fechado, cancelamento monotônico e um ponto
de aceitação sincronizado. Permissões aceitas antes do cancelamento não são
revogadas. Ele não bloqueia escritores externos ou não cooperativos. O
coordenador agora expõe o gate e uma entrada
principal `run`, exercitada por um adapter de teste cooperativo que
cancela dentro do callback. A regressão `cancellation_inside_admission_callback_never_reopens_writes`
foi adaptada para verificar aceitação real pelo gate compartilhado. A antiga
condição impossível para callback não cooperativo permanece explicitamente
caracterizada no teste `legacy_independent_cancellation_can_reopen_adapter_admission`:
a API legada `run_legacy` ainda aceita cancelamento independente e pode reabrir o
gate externo após cancelamento. Não há adapter de produção homologado nem
garantia de que escritores reais usem o gate. Portanto a integração e validação
permanecem incompletas; testes verdes da API cooperativa não tornam a API
legada estrita, nem demonstram proteção de escritores externos.

`run` recebe eventos; a aplicação cancela por `admission_gate().cancel()` e os
escritores cooperativos devem chamar `try_accept()` no ponto de admissão. Ao
descartar o future de `run`, um guard cancela o gate compartilhado, mas não
interrompe `spawn_blocking`, revoga escritas já aceitas ou garante fechamento do
gate externo do adapter. A API anterior mudou de nome e assinatura; migração
dos consumidores e validação de limites reais continuam necessárias.

O coordenador rejeita uma segunda chamada concorrente de `run` ou `run_legacy`
com `AlreadyExists`, sem cancelar a chamada ativa. Antes dessa proteção, uma
segunda chamada que recebia encerramento dos eventos podia cancelar o gate
compartilhado da primeira; o teste de regressão reproduziu a falha e passou
depois da correção. O guard cancela o gate ao sair ou descartar o future; ele
não sincroniza `run` com outros coordenadores que usem o mesmo banco ou com
escritores externos e não substitui reserva de capacidade.
Após essa alteração, `make test` passou com Go 1.27.3, Rust 1.99.1 e
`GOFLAGS=-count=1`, incluindo 12 testes do coordenador, fuzz smoke,
conformance e Floci com restart. Os drills CP-01 não foram repetidos nessa
execução, e o resultado não homologa adapters de produção.

O `AcceptedWrite` agora contabiliza operações aceitas; após fechar o gate e o
adapter, `flush` aguarda a liberação de todos os permits antes da captura. O
teste `flush_waits_for_previously_accepted_writer` falhou antes da mudança e
passou depois, verificando que novas aceitações são recusadas enquanto o flush
aguarda. Cada escritor cooperativo precisa manter seu permit até terminar a
escrita; esquecê-lo pode bloquear a espera indefinidamente, e soltá-lo cedo
quebra a proteção. Isso não drena processos externos, não cancela trabalho já
aceito, não implementa timeout de flush independente nem reserva de bytes/quotas. O TXID alvo continua
sendo observado no início do flush; o teste não muda essa semântica.
Após essa mudança, `make test` passou com Go 1.27.3, Rust 1.99.1 e
`GOFLAGS=-count=1`: 13 testes do coordenador, fuzz smoke, conformance cruzada
e Floci com restart passaram. Os testes opt-in CP-01 não foram repetidos; o
gate não comprova reserva de capacidade nem homologa adapters de produção.

Durante `run`, cancelamento enquanto o flush aguarda permits interrompe a
espera e mantém a admissão fechada, sem revogar o trabalho aceito; o teste
`cancellation_during_drain_leaves_admission_closed_without_waiting_for_writer`
falhou por timeout antes da correção e passou depois. `flush` independente não
possui cancelamento próprio e pode aguardar indefinidamente. O teste de estado
envenenado verifica que aceitação, cancelamento e drenagem retornam erro em vez
de abrir silenciosamente; não demonstra recuperação automática do mutex.
Após essas alterações, `make test` passou com Go 1.27.3, Rust 1.99.1 e
`GOFLAGS=-count=1`, incluindo 14 testes do coordenador, fuzz smoke, conformance
e Floci com restart. Os drills CP-01 não foram repetidos nessa execução.

Em teste local `cooperative_sqlite_writers_reject_after_cancellation`, uma
escrita SQLite de controle foi aceita antes do cancelamento; oito threads
esperaram uma barreira liberada após `cancel()` e todas foram recusadas pelo
mesmo gate. A consulta ao banco confirmou uma linha anterior e nenhuma
posterior. O teste usa adapter de teste, SQLite local e produtores cooperativos;
não cobre corrida simultânea de aceitação/cancelamento nem valida limites de
capacidade, drenagem de operações já aceitas ou integrações de produção.

O teste unitário `acceptance_racing_cancellation_obeys_linearization` também
disputa `try_accept()` e `cancel()` entre oito threads por 200 rodadas e verifica
que novas aceitações e reabertura falham depois que `cancel()` retorna. Ele não
classifica a ordem de cada aceitação concorrente nem prova exaustivamente todos
os escalonamentos; aceitação que lineariza antes do cancelamento permanece
permitida. Os três testes unitários `admission::tests` passaram com Cargo
1.99.1 offline nessa execução.
