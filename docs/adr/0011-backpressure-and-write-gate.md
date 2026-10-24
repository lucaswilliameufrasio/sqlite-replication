# ADR 0011: limites de trabalho e política de write-gate

- **Status:** Proposta para M2/M3; API e defaults não congelados
- **Data:** 2026-10-23

## Contexto

Storage lento/indisponível pode crescer WAL e spool sem limite. Concurrency de
worker, itens em espera e bytes duráveis são limites diferentes. Uma fila em
memória não torna trabalho durável.

## Proposta

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
