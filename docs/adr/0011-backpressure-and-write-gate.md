# ADR 0011: limites de trabalho e política de write-gate

- **Status:** modo estrito por padrão decidido; contrato da API, limites e defaults operacionais ainda em proposta
- **Data:** 2026-10-23

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

### Proposta de contrato para revisão — `Status` / `Health` / `Flush`

Esta seção é uma proposta para revisão, não autorização para implementar ou
anunciar capacidades incrementais. A API pública que dependa de captura WAL
continua bloqueada por CP-01.

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

Pontos para revisão antes de congelar: o que `Health` considera degradação,
semântica exata do alvo `Flush` para transações concorrentes, nomes e tipos das
posições de `Status`, e compatibilidade de drivers/adapters que conseguem
observar e impor admissão.

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

Antes de implementar, mapear produtor → espera → worker → storage → ACK,
definir semântica de aceitação, cancelamento e liberação de capacidade. Testar
fila cheia, storage lento, cancelamento, erros, recuperação de quotas e
isolamento por banco. Nenhuma dessas funções existe em M1.

## Reversão / reconsideração

Não remover limites nem mudar comportamento estrito/degradado sem novo contrato
e testes de saturação/cancelamento. Ajustar números após workload/benchmark
mensurado; limites de segurança não podem ser desativados silenciosamente.
