# ADR 0010: snapshots, compactação e retenção

- **Status:** Aceita para implementação M4; habilitação de restore incremental e GC segue bloqueada por CP-01
- **Data:** 2026-10-23

## Contexto

Histórico integral aumenta custo de restore e uso de storage. Compactação e GC
podem apagar o único caminho para um ponto de recuperação.

## Decisão

Snapshots imutáveis formam bases alternativas. Compactação cria e valida um
snapshot integral para uma fronteira TXID antes que qualquer origem possa ser
considerada para coleta. Restore por timestamp escolhe o maior TXID disponível
cujo timestamp LTX seja menor ou igual ao instante UTC solicitado; restore por
TXID é inclusivo e só pode selecionar fronteiras representadas integralmente.

GC usa o grafo de segmentos necessários aos restore points preservados, retorna
um plano explicável e começa em `dry_run`. Exclusão requer chamada explícita,
revalidação do plano contra o inventário atual e prova de que há snapshots
verificados suficientes para cada ponto prometido. Não há defaults implícitos
de idade, número de snapshots ou horizonte fino: a política é obrigatória e
fornecida pelo operador. Os 7 dias/dois snapshots continuam hipótese de
planejamento, não uma garantia de produto.

CP-01 permanece pré-requisito para habilitar publicamente restore incremental,
compactação/GC sobre cadeias incrementais ou qualquer promessa de segurança
WAL. Desenvolvimento e testes isolados de M4 não aprovam M2 nem expõem captura.

## Alternativas rejeitadas

- Apagar por idade ou prefixo sem conhecer a cadeia: pode remover bases ainda
  necessárias.
- Tratar compactação como checkpoint SQLite: são operações diferentes.
- Coletar automaticamente por idade/configuração padrão: pode contradizer
  restore points ainda prometidos pelo operador.

## Consequências e evidência

Política deve ser explícita em cada planejamento; plano antigo nunca pode ser
aplicado sem revalidação. O executor não roda automaticamente. Implementações
privadas filesystem Go/Rust e testes locais cobrem restore por TXID/instante,
cadeia SQLite real, snapshot compactado round-trip, dry-run, retenção dos
segmentos prometidos, coleta explícita e rejeição de inventário stale.

Os adapters S3 Go/Rust são exercitados privadamente via staging em filesystem
temporário, reutilizando a validação de cadeia existente. Isso consome espaço
local proporcional ao repositório baixado; não cria cache persistente nem API
pública. O Floci local cobre snapshot + delta, restore por TXID/timestamp,
compactação concorrente do mesmo ponto, revalidação de GC e preservação do ponto
prometido após interrupção parcial simulada de GC, mas não homologa AWS S3, R2
ou outros provedores.
Cadência é avaliada por intervalo TXID positivo e invocação explícita; scheduler
automático e defaults continuam deliberadamente ausentes.

Pendente para concluir o gate operacional: crash/fault-injection, coordenação do
GC com o writer único, homologação CP-01 e confirmação de semântica nos provedores
de produção. Até lá, M4 não é API pública nem promessa de produto.

## Reversão / reconsideração

Não apagar objetos durante rollout do GC. Começar em dry-run; só habilitar
remoção após validação em cópia de repositório e plano de rollback.
