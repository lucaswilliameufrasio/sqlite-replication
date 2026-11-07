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
prometido após o processo cliente ser encerrado abruptamente entre exclusões de
GC, mas não homologa AWS S3, R2 ou outros provedores.
Cadência é avaliada por intervalo TXID positivo e invocação explícita; scheduler
automático e defaults continuam deliberadamente ausentes.

Pendente para concluir o gate operacional: fault-injection nas demais fronteiras
de durabilidade, coordenação do GC com o writer único, homologação CP-01 e
confirmação de semântica nos provedores de produção. Até lá, M4 não é API pública
nem promessa de produto.

### Auditoria da janela de coordenação — 2026-11-06

No checkout `8b2d9b3`, os executores privados filesystem Go
(`go/internal/recovery/gc.go`, `ApplyGCPlan`) e Rust
(`rust/crates/core/src/recovery.rs`, `apply_gc_plan`) reconstroem e comparam o
plano antes de excluir candidatos, mas não mantêm um lock compartilhado com o
writer entre essa comparação e as exclusões. Os executores S3 correspondentes
(`ApplyS3GCPlan` e `apply_s3_gc_plan`) também reconstroem o plano e, em seguida,
emitem deletes em lotes separados; o inventário é obtido por listagem e download,
não como uma transação com os deletes.

Rejeitar um plano já stale ou adulterado antes de começar a exclusão não prova
segurança contra publicação ou mudança de política após essa verificação. Uma
coordenação apenas em memória também não protege outro processo ou host, e um
lock local não coordena por si só o prefixo remoto. Enquanto não houver um
protocolo compartilhado e verificado com todos os participantes pertinentes,
GC deve permanecer privado e não deve ser executado concorrentemente com writer,
compactador, outro GC ou alterações dos pontos prometidos. Esta é uma limitação
identificada por inspeção, não uma demonstração de corrupção reproduzida nem uma
homologação de solução; o teste `TestGCPlanIsDryRunExplicitAndRevalidated`,
naquela revisão, cobria alteração anterior à aplicação, não essa janela.

**Caracterização posterior:** o teste Go acima agora também intercala, por um
callback interno de teste, a publicação real em filesystem de um snapshot
entre a comparação e as exclusões. O executor aceita a publicação e remove o
candidato do plano anterior. Isso reproduz a janela em diretório temporário,
mas não demonstra perda de restore point, não exercita processos concorrentes
e não corrige ou homologa a coordenação local ou remota.

**Integração parcial experimental Go:** captura e aplicação filesystem do GC
adquirem a mesma trava consultiva não bloqueante antes do ciclo e a mantêm até
seu término. Um processo que não utiliza a trava não é impedido de publicar;
naquela etapa snapshot, compactação, restore, Rust e S3 não participavam. O GC permanece
privado e a corrida não é considerada corrigida. A implementação atende Linux
e macOS, mas nesta etapa apenas Linux foi executado; demais plataformas falham
fechadas. A existência permanente do arquivo de trava é necessária para que
participantes cooperativos compartilhem o mesmo inode: não removê-lo durante
operações. A prova entre processos até agora cobre apenas a primitiva isolada,
não os ciclos de captura e GC completos em processos distintos.

**Etapa Go posterior:** o restore incremental e a compactação filesystem também
adquirem a trava pelo ciclo completo; a compactação reutiliza seu plano ao
restaurar internamente, sem adquirir a trava novamente. O snapshot inicial,
restore inicial, Rust, operações remotas e participantes não cooperativos
continuam fora da coordenação. Testes Go locais verificam contenção e a suíte
de recovery; isso não fecha o gate de concorrência nem protege S3.

**Etapa Go seguinte:** snapshot e restore iniciais também adquirem a trava;
`EnsureRepository` no snapshot ainda precede a aquisição. Os testes atuais
exercitam contenção no mesmo processo e o ciclo geral, mas não comprovam a
exclusão mútua dos ciclos completos em processos distintos. Rust, publicações
diretas e storage remoto permanecem fora; GC não é considerado seguro para
concorrência enquanto esses limites persistirem.

**Estado posterior Go/Rust (`bf93969`):** captura, snapshot e restore iniciais,
restore incremental, compactação e aplicação de GC filesystem utilizam a mesma
trava consultiva local nas duas implementações. Nos snapshots iniciais, a
inicialização de metadados ainda precede sua aquisição. Testes opt-in em Linux
x86-64 confirmaram contenção da primitiva entre processos nos dois sentidos e
reaquisição Rust após liberação Go; não exercitaram ciclos completos de captura
e GC concorrentes. A caracterização Go acima chama o executor interno sem trava
e usa publicação direta: ela demonstra o limite de participantes que contornam
o protocolo, não uma falha comprovada dos ciclos cooperativos atuais.

Publicação direta no storage, alteração externa do repositório ou do arquivo de
trava, mudanças de política e outros participantes não cooperativos permanecem
fora da proteção. O arquivo de trava não deve ser removido durante operações.
S3 não é coordenado por essa trava; Floci não prova semântica universal nem
coordenação remota. Não se declara homologação de GC concorrente, CP-01 ou
distribuição com base nesses testes.

## Reversão / reconsideração

Não apagar objetos durante rollout do GC. Começar em dry-run; só habilitar
remoção após validação em cópia de repositório e plano de rollback.
