# Adapters de admissão: levantamento e lacunas

> **Nota posterior:** os relatos abaixo preservam etapas históricas. A ADR 0015
> substituiu o alvo previamente observado pela captura após fechamento e
> drenagem; os testes citados de alvo desconhecido foram alterados para o novo
> contrato. O gate compartilhado Go foi adicionado em `851fcbd`. Essas mudanças
> não homologam adapters de produção nem quotas físicas.

Inspeção estática inicial do checkout em 2026-11-05; os testes dessa data são
evidências históricas. Em 2026-11-06 o usuário aprovou a direção da ADR 0011 e
foi introduzido o gate compartilhado Rust; os resultados posteriores constam
na seção de atualização abaixo. Isto não é homologação de adapter de produção.

## Atualização de 2026-11-06: integração disponível

`zero_observed_txid_is_rejected_and_keeps_admission_closed` reproduziu um
adapter de teste que informava TXID zero: antes da mudança, `status` o aceitava.
O coordenador agora rejeita zero nas observações usadas por `status` e `flush`,
reporta saúde degradada da categoria adapter e mantém a admissão fechada na
falha de flush. O teste não valida a autenticidade de TXIDs positivos nem
resolve a observação concorrente ou a homologação do adapter.

`target_becoming_unknown_after_drain_keeps_admission_closed` cobre a consulta
intermediária adicionada em `9295dba`: um adapter de teste informa alvo conhecido
na entrada e desconhecido após a drenagem vazia. O flush retorna erro, o gate
rejeita ingresso e o adapter recebe apenas o fechamento. Não há escritor ativo,
checkpoint concorrente ou integração de produção nesse ensaio; a fonte de
TXID e a janela depois da última consulta continuam sem solução homologada.

`unknown_observed_target_reports_degraded_health` demonstra que o status local
era `Healthy` quando o adapter de teste retornava `None` e os limites ainda não
haviam expirado. Agora `health()` reporta `Degraded` com categoria `Adapter`;
o teste verifica também que `flush` falha e o gate continua fechado. Quando
há simultaneamente limite excedido e alvo desconhecido, a categoria `Adapter`
tem prioridade no único campo de erro local. Isso não cria uma fonte confiável
de TXID, não sincroniza observação com checkpoints e não homologa o adapter.

`target_becoming_unknown_before_reopen_keeps_admission_closed` usa um adapter
de teste que informa TXID 1 inicialmente e passa a informar `None` na última
consulta de status. Antes da mudança, `flush` retornou sucesso e reabriu; agora
retorna erro e mantém a admissão fechada. O coordenador também rejeita `None`
na consulta intermediária após drenagem. Isso não cria uma fonte confiável de
TXID, não sincroniza mudança posterior à última consulta com reabertura e não
homologa adapter, quotas ou tratamento de checkpoints.

`stale_target_can_make_flush_skip_checkpointed_sqlite_changes` usa um adapter
deliberadamente incorreto que informa o TXID durável antigo após dois commits
SQLite e checkpoint. O `flush` retorna sucesso sem capturar; restaurar o TXID 1
produz o snapshot sem as duas linhas. O teste caracteriza o risco de um alvo
subestimado, não demonstra falha de um adapter real nem corrige a interface.
O contrato de `observe_txid` agora alerta para esse risco e exige `None` quando
o alvo não for confiável; um protocolo concreto de observação permanece pendente.

`checkpointed_commits_can_collapse_into_one_reconciliation_txid` caracteriza
uma restrição para obter o alvo do coordenador: dois commits SQLite separados
após o snapshot, seguidos por `wal_checkpoint(TRUNCATE)` bem-sucedido antes da
captura, resultaram em um único delta de reconciliação, TXID 2; o restore contém
as duas linhas. A tradução normal atribui TXIDs por commit de WAL, mas a
reconciliação de imagem pode consolidá-los. Portanto contar commits observados
no WAL não determina, por si só, o TXID futuro da captura; mudanças de geração,
checkpoint e concorrência exigem protocolo adicional. Este teste não implementa
um adapter nem estabelece uma solução para a observação do alvo.

`cooperative_capture_waits_for_real_ltx_measurement` agora exercita a API
pública `capture_once_with_budget`, não apenas o wrapper interno. Com a medição
de LTX pausada sob o mutex, o gate de teste sinaliza aquisição e a captura não
termina durante a espera observada; após liberar a medição, o teste verifica a
publicação, invalidação, reconciliação e restore em TXID 2. O gate não bloqueia
escritores reais; esta observação de escalonamento não prova quotas, exclusão de
outros processos nem homologação de adapter.

`failed_gate_acquire_invalidates_budget_even_with_active_writer` reproduziu uma
falha de aquisição do gate com reserva ativa: antes da mudança, outra reserva
era aceita. A captura cooperativa agora tenta invalidar o orçamento nesse erro,
mesmo com reserva ativa, e preserva o erro de aquisição caso a invalidação também
falhe. A liquidação da reserva anterior não restaura ocupação desconhecida. O
gate é simulado e não houve captura; não prova quotas nem revoga trabalho aceito.
Uma reconciliação independente posterior ainda exige coordenação da aplicação.

O teste `budgeted_public_capture_drains_application_writer_before_capture` foi
ampliado para um segundo ciclo: após reabrir o gate de aplicação de teste, o
orçamento desconhecido ainda rejeita a reserva; após reconciliação lógica, uma
segunda escrita SQLite é capturada e restaurada em TXID 3. O gate, os custos e a
medição continuam de teste; não são adapter homologado nem limites de recursos.

`failed_capture_invalidates_release_callback_reconciliation` reproduziu uma
falha de captura seguida de callback de liberação que reconciliava o orçamento:
antes da mudança, novas reservas eram aceitas. A captura cooperativa agora
invalida após erro de captura **ou** de liberação, inclusive se uma reserva ativa
impediu a mutação; a regressão passou. Ela não revoga escritas já aceitas nem
impede nova reconciliação por outro chamador, e não homologa adapter ou quotas.

`failed_gate_release_invalidates_callback_reconciliation` reproduziu um caso
em que o callback de liberação do gate de teste reconciliava o orçamento e
depois retornava erro; antes da mudança uma reserva ainda era aceita. Agora a
API invalida o orçamento após o erro de liberação e reporta qualquer erro da
invalidação junto com o de liberação. O teste passa com captura e restore reais.
O gate é simulado: a invalidação não revoga escritas já aceitas nem impede uma
reconciliação independente posterior; não substitui coordenação da aplicação,
adapter homologado ou quotas conservadoras.

`budgeted_public_capture_drains_application_writer_before_capture` usa um gate
de aplicação **de teste** que fecha a admissão e drena um escritor já admitido.
Enquanto o gate está fechado, outro ingresso é recusado; o escritor confirma uma
linha SQLite e liquida sua reserva antes de liberar o gate. A captura pública
então publica LTX, invalida o orçamento e o restore verifica as duas linhas;
uma reconciliação lógica permite nova reserva. A sincronização e custos são
de teste: não é adapter de produção, prova de todos os produtores, limite de
WAL/spool/staging ou garantia contra processos externos.

`public_budgeted_capture_release_error_preserves_durable_data_and_unknown_occupancy`
injeta falha de liberação do `WriteGate` após captura pública cooperativa real:
o erro informa que a captura foi durável, o orçamento recusa reservas por
ocupação desconhecida e o restore em TXID 2 recupera o commit. Uma reconciliação
com valor lógico de teste permite novamente reservar. O gate que falha é
simulado e não compartilha a admissão: isso não prova que seja seguro retomar
uma aplicação após erro do gate nem que custos ou quotas sejam conservadores.

`public_capture_rejects_active_producer_then_recovers` mantém uma reserva ativa
em outra thread após um commit SQLite. A captura pública cooperativa retorna
`WouldBlock`, libera o gate de teste e não permite restaurar TXID 2; após a
liquidação, o retry captura e restaura o commit e invalida a ocupação lógica.
O `WriteGate` do ensaio apenas contabiliza liberações e não bloqueia escritores;
os custos são artificiais. Não demonstra quotas nem adapter de produção.

`capture_once_with_budget` é uma entrada experimental da captura pública: obtém
o `WriteGate` antes de `with_occupancy_mutation` e o libera após a saída deste,
inclusive quando a captura retorna erro. O teste de integração
`cooperative_public_capture_keeps_gate_callbacks_outside_budget_lock` usa
callbacks de gate de teste que invalidam o mesmo orçamento, captura SQLite/LTX
real e restauração no TXID 2; também verifica retorno de erro para origem ausente.
O gate do teste não controla produtores reais. A API não é adapter homologado,
não fornece limites conservadores nem protege mutações que contornem o orçamento;
`WriteGate` e escritores continuam responsabilidade da aplicação. O teste não
estabelece recuperação em panic ou todos os escalonamentos concorrentes.

Na integração com a API pública, `capture_once` adquire e libera o `WriteGate`
dentro da chamada; se ela for envolvida por `with_occupancy_mutation`, os dois
callbacks ocorrem enquanto o mutex do orçamento está retido. Um callback que
reentre no mesmo orçamento ou aguarde um produtor bloqueado nele pode causar
deadlock. A liberação do `WriteGate` também não reconcilia o orçamento nem
impede escritas que o contornem. A documentação da API agora explicita essa
ordem; não é um teste de deadlock, adapter de produção ou quota homologada.

`fresh_budget_requires_measurement_after_coordinator_recreation` descarta e
recria coordenador, adapter e orçamento no mesmo processo sobre o mesmo banco e
repositório. A nova instância recusa reservas antes de medir e após falha de
medição; após retry, uma escrita cooperativa é capturada e restaurada em TXID 2.
O adapter e TXID são de teste; o processo e a conexão SQLite permanecem ativos.
Não é drill de restart/crash, nem comprovação de limites de WAL, spool ou staging.

O cenário `failed_reconciliation_keeps_admission_closed_after_real_capture`
também injeta falha na primeira medição do adapter de teste: o primeiro flush
falha sem aceitar reservas; após retry bem-sucedido, um produtor confirma uma
alteração SQLite e a captura e a recuperação são exercitadas. Isto testa uma
instância nova, não um restart de processo ou recuperação de estado persistido;
os valores usados não comprovam quotas reais.

`legacy_receiver_cancellation_does_not_atomically_close_gate` força outra ordem:
após o flush abrir o gate, o callback de métricas envia `true` ao receiver
legado e tenta aceitar uma escrita antes de retornar ao worker. A aceitação
ocorre; depois o worker observa o receiver e cancela o gate. Trata-se de uma
caracterização determinística da janela, não de uma correção nem de prova de
segurança para o adapter. Encaminhar o receiver por uma tarefa assíncrona não
sincroniza `send` com `try_accept`: para a garantia estrita, produtores e
controlador devem compartilhar o `AdmissionGate`, chamar `cancel()` diretamente
e usar `run`; a operação legada continua sem essa garantia.

Na regressão de cancelamento legado, o callback de métricas observa o gate
após o flush e antes da próxima iteração do worker. Com cancelamento dentro do
callback de abertura do adapter, o teste observou aceitação antes da mudança;
após uma segunda verificação do receiver, observou rejeição. O adapter ainda
recebe `true` temporariamente, e cancelamentos posteriores à segunda verificação
podem competir com a abertura: o receiver independente não fornece garantia
atômica. Para garantia estrita, usar o gate compartilhado e `run`.

Na investigação da API legada, observar `try_accept` apenas no callback de
fechamento após o cancelamento não detectou a janela de reabertura: nesse
instante o coordenador já havia cancelado o gate. Uma verificação experimental
nesse ponto passou mesmo com a implementação legada inalterada; portanto não
serve como teste de regressão para a corrida entre o callback de abertura e a
observação do receiver. O teste existente continua caracterizando a transição
do adapter para `true`, sem demonstrar aceitação de um produtor nessa janela.
Não houve correção da corrida; a garantia estrita continua exigindo `run` e
cancelamento pelo gate compartilhado.

`failed_reconciliation_keeps_admission_closed_after_real_capture` usa um
adapter exclusivamente de teste que reconcilia antes de reabrir o gate.
Após um commit cooperativo e captura SQLite/LTX real, uma falha de medição
injetada impede a reabertura; o restore no TXID 2 confirma o valor capturado,
e um retry da medição permite nova reserva. O TXID e o custo da reserva são
controlados pelo teste. O comprimento observado do WAL é apenas um retrato,
não um limite conservador de crescimento; spool e staging não são medidos.
Não é adapter homologado nem proteção contra produtores externos.

`coordinator_capture_panic_closes_gate_and_poisons_budget` injeta panic após o
snapshot em captura real executada por adapter de teste via `Coordinator::flush`.
O flush retorna erro, o gate permanece fechado e o orçamento envenenado recusa
reservas e reconciliação; o commit SQLite prévio continua legível na origem.
O teste não verifica restore nem recuperação posterior da instância envenenada;
o TXID e os custos são artificiais e não provam limites físicos ou de spool.

`cooperative_capture_panic_poison_fails_closed` injeta um panic após o snapshot
em captura real envolvida por `with_occupancy_mutation`: a instância cujo mutex
foi envenenado rejeita reserva, reconciliação e nova mutação. O teste captura o
panic apenas para verificar essas respostas; não demonstra recuperação automática
da instância nem segurança para operações que contornem o orçamento.

`cooperative_capture_waits_for_real_ltx_measurement` mantém uma leitura real dos
comprimentos LTX dentro de `reconcile_with` enquanto outra thread solicita uma
captura SQLite/LTX por `with_occupancy_mutation`. No ensaio a captura não
terminou durante a espera observada; após liberar a medição publicou LTX e
invalidou a ocupação até nova medição. O teste não prova todos os escalonamentos,
nem coordena captura que contorne esse mesmo orçamento. Comprimentos lógicos
observados não são limites de crescimento ou quotas físicas.

Atualização posterior do teste `cooperative_budget_flush_real_capture_failure_and_retry`:
o adapter exclusivamente de teste agora usa `with_occupancy_mutation` ao chamar
a captura SQLite/LTX real. A reserva é liquidada com custo lógico artificial;
uma falha de captura e outra de medição são injetadas. O retry invalida um
baseline lógico reinstalado, exigindo nova reconciliação com o WAL observado;
o restore verifica o commit. Não é adapter de produção: TXID é controlado pelo
teste, não existe proteção de processos externos nem custos conservadores de
WAL, spool ou staging comprovados. O coordenador por si só não aplica esse wrapper.

`ConservativeBudget::with_occupancy_mutation` fornece exclusão cooperativa com
`reconcile_with` pela mesma instância: recusa a operação se há reservas ativas,
invalida a ocupação antes do callback e a mantém desconhecida após sucesso ou
erro. Testes direcionados exercitam captura SQLite/LTX com falha e retry e
caracterizam exclusão concorrente entre callbacks. Captura direta, coordinator,
outras instâncias e escritores externos não são automaticamente integrados;
o chamador deve encaminhar todas as mutações pelo protocolo. O callback mantém
mutex síncrono, pode bloquear e não pode reentrar. O teste usa unidades lógicas,
sem comprovar limite de WAL, spool ou staging nem adapter homologado.

`characterize_capture_can_stale_reconciliation_measurement` força a captura
real entre leitura e instalação no callback de `reconcile_with`: no ensaio, os
arquivos LTX passaram de 351 para 541 bytes, enquanto orçamento lógico de 352
instalou 351 e aceitou mais uma unidade. O teste impõe essa ordem no próprio
callback, não demonstra corrida observada em produção. Ele mostra que o mutex
do orçamento não exclui a captura: mesmo medindo dentro do callback, o chamador
precisa coordenar a captura durante toda a janela. Nenhuma quota foi corrigida.

Atualização do teste `cooperative_budget_flush_real_capture_failure_and_retry`:
uma falha de medição injetada após falha de captura mantém reservas recusadas;
após retry da captura, `reconcile_with` lê `Coordinator::status().wal_bytes`
dentro do callback, e a restauração verifica o commit. A reserva inicial usa
custo artificial; o comprimento observado é um retrato lógico do WAL, não um
limite futuro, e não inclui spool ou staging. O adapter e TXID são de teste;
captura e medição são sequenciais, não exclusão geral de terceiros.

`cooperative_budget_flush_real_capture_failure_and_retry` combina
`Coordinator::flush`, reserva compartilhada e commit SQLite: o flush aguarda
a liquidação desconhecida, uma falha injetada após snapshot interrompe a
captura, e uma tentativa posterior captura o WAL e restaura o valor confirmado
em TXID 2. A captura não reconcilia o orçamento automaticamente; o teste
reconcilia manualmente com unidades lógicas. O adapter e seu TXID observado são
controlados pelo teste, não são adapter homologado; não há medição conservadora
de WAL, spool ou staging nem proteção de escritores externos demonstrada.

`flush_drains_abandoned_budget_reservation_but_does_not_reconcile_it` usa o
coordenador com adapter de teste e uma reserva lógica compartilhada: o flush
fecha a admissão e aguarda a reserva; ao abandoná-la, o flush termina e o gate
reabre, mas o orçamento continua desconhecido e recusa reservas até uma
reconciliação manual. O adapter não realiza captura LTX real nesse ensaio;
o teste não homologa o ciclo completo nem estabelece limites de disco.

`characterize_capture_does_not_exclude_budget_reservations` faz um commit
SQLite sob reserva lógica e pausa uma captura real após o snapshot; outra thread
consegue reservar e liquidar no mesmo orçamento durante essa pausa. Trata-se da
captura direta com gate aberto, não de `Coordinator::flush` nem de uma escrita
SQLite simultânea: o teste demonstra ausência de integração automática entre
essa captura e o orçamento, não uma falha de exclusão do lease SQLite. A
aplicação precisa coordenar captura, admissão e medição; não há quota de disco
comprovada por esse cenário.

O teste `active_reservation_prevents_measurement_without_invalidating` verifica
uma distinção para o futuro adapter: `reconcile_with` com reserva ativa devolve
`WouldBlock` sem chamar o callback nem invalidar o valor anterior. A liquidação
conhecida ainda é contabilizada; se houver perda independente de confiança na
ocupação, o chamador precisa usar `reconcile(None)` e reconciliar após drenagem.
O teste usa unidades lógicas, não exercita captura concorrente nem demonstra
limites reais de WAL, spool ou staging.

Atualização posterior à caracterização abaixo: `reconcile(Some(valor))` agora
recusa valores pré-calculados; `reconcile(None)` continua invalidando o estado.
`reconcile_with` executa a medição e instala o resultado sob o mutex da mesma
instância, recusando reconciliação com reservas ativas. O teste de regressão
confere rejeição do valor antigo; testes de concorrência e erro verificam
exclusão e estado desconhecido. O callback deve realmente medir dentro da
chamada: a API não consegue impedir que ele devolva um valor antigo. Também não
exclui captura, escritores externos ou outras instâncias de orçamento; um
callback lento bloqueia chamadas e reentrada pode causar deadlock. Não há
garantia de quota de disco nem adapter de produção homologado.

Antes da alteração que rejeita valores pré-calculados, o teste histórico
`characterize_stale_measurement_can_understate_occupancy` demonstrou que uma
medição anterior à escrita podia ser instalada como se fosse atual. Na API
atual, `reconcile(Some(...))` retorna `InvalidInput`; a regressão atual é
`precomputed_stale_measurement_is_rejected`. Isso não prova que um callback
de `reconcile_with` obtenha uma medição atual ou inclua todas as mutações:
capturas e produtores que contornem o protocolo ainda precisam ser excluídos.

O teste `conservative_budget_reconciles_after_real_capture` executa captura
SQLite/LTX real após liquidação de custo desconhecido. A captura publica LTX,
mas não reconcilia automaticamente o orçamento: novas reservas seguem recusadas
até a aplicação chamar `reconcile`. No ensaio, os comprimentos lógicos somados
dos arquivos LTX passaram de 351 para 541 bytes. A reconciliação do teste é
manual e exclusiva, sem concorrência, e exclui WAL, staging, blocos físicos e
outros produtores. Não é integração automática, adapter de produção ou quota.

`ConservativeBudget::reconcile(None)` agora invalida a ocupação mesmo quando
há reservas em voo. Antes, retornava `WouldBlock` sem invalidar e podia deixar
novas reservas serem aceitas após o chamador detectar ocupação desconhecida.
O teste `unknown_occupancy_invalidates_even_with_active_reservations` falhou
antes e passou depois: liquidação subsequente não restaura o valor desconhecido;
somente reconciliação após drenagem permite novas reservas. A invalidação depende
de o chamador detectá-la e chamá-la; não interrompe trabalho aceito nem mede
WAL, spool ou staging, e não torna o controlador uma quota de disco.

O teste `conservative_budget_reconciles_observed_wal_after_unknown_growth`
executa commit SQLite e, após liquidação com crescimento desconhecido, confirma
que reservas falham até reconciliar com `Status::wal_bytes`: no ensaio o WAL
passou de 0 a 4.152 bytes e o limite lógico era 65.536. A medição ocorreu sem
outros produtores no teste. O valor é um retrato do comprimento lógico do WAL,
não inclui spool ou staging, não garante espaço físico nem estabelece teto de
crescimento para a próxima transação; o orçamento de 16.384 é apenas teste,
não limite verificado pelo SQLite. Não é adapter homologado ou prova de quota.

O teste unitário `concurrent_reservations_cancel_and_reconcile_without_overcommit`
mantém simultaneamente 12 tentativas em um orçamento lógico de 5 unidades,
com 2 já ocupadas: exatamente 3 reservas entram, reconciliação enquanto ativas
é recusada e o cancelamento impede novas aceitações. Após liquidação, o teste
confere 5 unidades contabilizadas e nenhuma reserva ativa; reconciliar depois
não reverte o cancelamento. Os três testes de `conservative_budget::tests`
passaram com Cargo 1.99.1 offline. Não é teste exaustivo de escalonamentos.

Na inspeção de `coordinator.rs`, `Status` calcula `spool_bytes` a partir de
`repository_root/ltx` e `wal_bytes` a partir do arquivo WAL, sem um campo para
`.staging`. `ConservativeBudget::reconcile_with` executa a medição fornecida
pelo chamador sob o mutex do orçamento, mas não verifica exclusão de produtores
e mutações que contornem a mesma instância. Logo essas medições isoladas não são
um snapshot atômico WAL/spool/staging nem provam crescimento máximo futuro ou
blocos físicos. Ligar `Status` diretamente ao orçamento como se comprovasse
uma quota seria inseguro; permanece necessária integração específica com
exclusão, limites conservadores e recuperação após restart.

O teste `conservative_budget_sqlite_unknown_settlement_requires_reconciliation`
executa um commit SQLite e passa crescimento desconhecido à liquidação. A
liquidação retorna erro, o commit permanece e novas reservas são recusadas até
reconciliação; também verifica que reconciliação com reserva ativa é recusada.
Os valores fornecidos pelo teste são unidades lógicas artificiais, não medições
conservadoras de WAL, spool ou staging. Não prova segurança contra escritores
externos, medição confiável ou recuperação após falhas reais de filesystem.

A busca por implementações de `CoordinatorAdapter` no checkout Rust encontrou
somente testes de `core` e testes do exemplo de telemetria; não foi encontrado
um adapter de produção com pool de conexões ou ponto central de escrita a ser
integrado. Os exemplos de telemetria usam adapters de teste; a existência de
`Connection::open` em seus testes não demonstra controle de todos os escritores
de uma aplicação. Não se deve trocar um callback no-op por uma alegação de
reserva de capacidade ou proteção de conexões externas.

O `Coordinator::run` atual usa o `AdmissionGate` compartilhado para
cancelamento e aceitação de escritores cooperativos; `run_legacy` mantém o
receiver independente e sua corrida é caracterizada em teste. O teste com
SQLite real inseriu uma linha antes do cancelamento e recusou oito escritores
que tentaram admissão depois; outro teste disputou cancelamento e admissão em
200 rodadas, sem demonstrar todos os escalonamentos. O flush agora aguarda
permits cooperativos já aceitos após fechar a admissão, mas depende de cada
escritor mantê-los até o término e não drena escritores externos; quotas e
reserva continuam dependentes do adapter.
O gate completo `make test` passou em 2026-11-06, incluindo Floci e conformance;
`go test -count=1 ./...` passou depois. Isso não homologa um adapter de
produção nem estabelece segurança para uma aplicação que não encaminhe todos
os produtores pelo mesmo gate. Não há aqui proteção contra perda física de
energia ou cache, nem evidência de compatibilidade universal S3/R2.

### Reserva e espera: inspeção adicional

`try_accept()` não recebe tamanho estimado de escrita, crescimento máximo de
WAL/spool, identidade do produtor ou custo de upload: apenas verifica o gate
e contabiliza operações ativas. O coordenador compara o estado observado aos
limites antes de reabrir e entrega os limites ao callback; a busca no core
encontra `max_uploads_in_flight` apenas na configuração e validação. Portanto
não foi demonstrada reserva atômica de bytes ou limitação de uploads em voo.
Sem um adapter que conheça os custos e controle todos os produtores, não é
seguro inferir essas garantias ou transformar contagem de permits em quota.

Em `timed_out_flush_during_drain_stays_closed_until_retry`, um permit retido
impede a conclusão de `flush`; um timeout externo descarta a espera, novas
escritas continuam recusadas e, após liberar o permit, um novo flush conclui e
reabre o gate. O teste passou com Cargo 1.99.1 offline. Isso não estabelece um
timeout automático: o chamador deve fornecê-lo e uma operação aceita não é
interrompida; nem testa fechamento ou erro de adapters reais. A reserva e os
limites ainda exigem implementação cooperativa e ensaios de saturação.

### Controlador de referência por unidades em voo

O core passou a fornecer `CapacityGate`, um controlador local e não bloqueante
que combina `AdmissionGate` com reserva de unidades positivas declaradas pelo
produtor. A reserva é serializada sob mutex, recusa excesso com `WouldBlock` e
é liberada ao descartar `CapacityPermit`; o permit também mantém a admissão até
sua liberação. Os testes cobrem excesso, overflow, retomada, cancelamento,
16 produtores concorrentes para capacidade 2 e interação com a drenagem do
coordenador. Os seis testes de admissão e o teste de integração direcionado
passaram com Cargo 1.99.1 offline.

As unidades não são medidas de crescimento de WAL, spool ou uploads; são um
orçamento temporário de operações em voo fornecido pelo chamador. Liberar o
permit não significa que bytes persistidos desapareceram. A garantia depende de
todos os produtores compartilharem **a mesma instância**, fornecerem custos
conservadores, não contornarem `CapacityGate` diretamente com `AdmissionGate`
e manterem o permit até o fim. Não há persistência, limite por tenant, timeout,
fairness, mecanismo para atualizar orçamento ou reconciliação de ocupação real.
Não substitui adapter que imponha `Limits`, testes de saturação por recurso ou
homologação de aplicação; também não torna a API legada segura.

#### Condições mínimas para uma reserva conservadora

Inspeção do código: `CapacityGate::new` cria um contador independente, e
`CapacityPermit::drop` libera unidades independentemente do WAL ou LTX que
continua no disco. O coordenador só compara WAL e spool observados ao fim do
flush; o callback de admissão é fornecido pela aplicação. Portanto, embrulhar
`try_reserve` com valores rotulados como bytes não implementaria esse contrato.

Uma integração por recurso precisaria fechar a admissão quando qualquer custo
máximo por transação, ocupação inicial confiável ou cobertura de produtores for
desconhecido; reservar conjuntamente WAL, spool e staging antes de aceitar;
converter reservas em ocupação persistente após commit e reconciliar essa
ocupação após captura, exclusão e reinício. Precisaria também tratar falha de
rollback, checkpoints, limites do próprio SQLite, espaço transitório da captura
e escritores externos. Uma contabilização lógica não garante blocos físicos ou
espaço disponível compartilhado com outros processos. Os métodos, custos
máximos e ponto de reconciliação dependem da aplicação concreta e ainda não
foram implementados nem validados aqui; enquanto faltarem, o adapter estrito
deve recusar a admissão em vez de inferir um orçamento de disco do permit.

O teste `characterize_operation_quota_does_not_enforce_wal_bytes` demonstra
essa limitação com SQLite real e adapter de teste: após inicializar com WAL
truncado, `max_wal_bytes=9000` e `CapacityGate` de uma operação, três commits
sequenciais com permits elevaram o WAL a **12.392 bytes**. Uma nova reserva
ainda foi aceita antes de outra reconciliação. O teste passa justamente por
caracterizar a ausência de enforcement por byte; não é validação do contrato
estrito. A capacidade em voo foi liberada após cada commit, enquanto o WAL
persistiu. O adapter usado apenas registra transições; isso não prova que um
adapter cooperativo com orçamento por transação teria o mesmo resultado.

O teste `capacity_reference_controls_cooperative_sqlite_transactions` usa
SQLite real e capacidade de uma unidade: uma transação em outra thread mantém
o permit até o commit, e uma segunda reserva recebe `WouldBlock`; após o
commit e liberação, outra transação é aceita. O cancelamento durante essa
segunda transação impede novas reservas, mas a transação já aceita ainda pode
concluir; a consulta final confirmou duas linhas. Passou três execuções
consecutivas com Cargo 1.99.1 offline. Trata-se de um cenário de teste
cooperativo, não de adapter de produção, e a unidade mede uma operação em voo,
não bytes de WAL ou spool nem espaço liberado após persistência.
O teste `failed_sqlite_transaction_rolls_back_and_releases_capacity` também
provoca erro real de chave duplicada após um insert, confirma saturação enquanto
o permit está retido, executa rollback, verifica zero linhas persistidas e
confirma que uma reserva e inserção de retry funcionam. Passou três execuções
consecutivas com Cargo 1.99.1 offline. O rollback é explícito e o permit é
mantido até depois dele: isso não prova recuperação se o processo cair,
rollback falhar ou o aplicativo liberar o permit prematuramente.
Após esse teste, `make test` passou com 18 testes do coordenador, testes Go
sem cache, Clippy, fuzz smoke, conformance e Floci com restart. Drills CP-01
não foram repetidos nessa execução.
Após o teste ser adicionado, `make test` passou com Go 1.27.3, Rust 1.99.1 e
testes Go sem cache, incluindo 17 testes do coordenador, fuzz smoke,
conformance cruzada e Floci com restart. Os drills CP-01 não foram repetidos.

### Relação entre custo lógico e WAL — experimento limitado

Em 2026-11-06, Python/SQLite 3.54.0 no host Linux criou dois bancos temporários
independentes com páginas de 4.096 bytes, WAL, autocheckpoint desativado e WAL
truncado antes do ensaio. Cada banco recebeu 20 linhas com payload sintético de
100 bytes, total lógico de 2.000 bytes. Com todos os inserts em uma transação,
o WAL cresceu de 0 para **4.152 bytes**; com um commit por linha, cresceu de 0
para **82.432 bytes**. As conexões permaneceram abertas até a medição. Os
resultados correspondem, nesse ensaio, a um cabeçalho de 32 bytes mais,
respectivamente, 1 e 20 frames de `24 + 4096` bytes. Os bancos temporários
foram criados e removidos exclusivamente pelo ensaio.

Isso mostra que o mesmo volume lógico de payload pode produzir tamanhos WAL
diferentes conforme as transações; não fornece um limite superior geral ou
estima crescimento para outros esquemas, índices, triggers, checkpoints,
páginas ou concorrência. Uma unidade de `CapacityGate` não representa um byte
de WAL e um tamanho de payload não é, por si só, uma reserva segura de WAL.

Para spool, `capture.rs` lê WAL estável, cria um backup temporário em
`.staging`, traduz transações para LTX e pode gerar delta de reconciliação a
partir da imagem inteira quando há divergência. `status_internal` conta os
arquivos sob `repository_root/ltx`, não demonstrando que backup temporário,
overhead de filesystem ou staging inteiro estejam incluídos em `spool_bytes`.
Não foi medido crescimento de spool neste experimento. Assim, tamanho lógico
da escrita ou tamanho atual do WAL não estabelece sozinho um teto para todo o
espaço de disco necessário à captura/publicação.

Um limite comprovável exige que o adapter da aplicação imponha previamente um
orçamento conservador de frames e espaço transitório por transação, incluindo
concorrência, staging, reconciliação e consumo existente, e faça sua reserva
atômica com os demais produtores. A API atual não recebe esses limites por
escrita nem mede esses custos; não se definiu um fator universal ou se declarou
enforcement por bytes. Sem workload, política de transações e adapter concreto,
essa garantia continua não homologada; quotas reais e sua recuperação após
erro/restart exigem projeto e testes específicos. O experimento não é benchmark
de capacidade, nem teste de perda física de energia.

### Captura Rust: WAL, LTX e staging amostrados

O teste `characterize_wal_spool_and_staging_for_controlled_writes` usa SQLite
real, páginas de 4.096 bytes, autocheckpoint desligado, 20 inserts de 100 bytes
e snapshot prévio. No cenário `checkpointed`, executa `wal_checkpoint(TRUNCATE)`
antes da captura para provocar reconciliação da imagem. Em três execuções
consecutivas com Cargo 1.99.1 no host Linux x86-64, os resultados foram:

| Cenário | WAL antes | Frames | LTX acrescentado | Staging na imagem / após publicação | Transações capturadas |
|---|---:|---:|---:|---:|---:|
| Um commit | 4.152 B | 1 | 328 B | 8.192 / 8.192 B | 1 |
| 20 commits | 82.432 B | 20 | 5.286 B | 8.192 / 8.192 B | 20 |
| Checkpoint antes da captura | 0 B | 0 | 328 B | 8.192 / 8.192 B | 1, offset WAL 0 |

Os tamanhos são comprimentos lógicos de arquivos, não blocos físicos alocados,
uso de memória ou pico contínuo de staging: foram amostrados apenas nos hooks
`AfterSourceSnapshot` e `AfterSpoolPublication`. `ltx_added` é a diferença
no diretório `ltx` após a captura e exclui baseline, metadados e staging. O
cenário com checkpoint mostra que WAL zerado ainda pode exigir backup e gerar
LTX de reconciliação; não demonstra uma relação geral entre entrada, frames e
spool. Payload, índices, compressibilidade, tamanho do banco, concorrência e
falhas podem mudar os custos. O teste não impõe quotas nem prova capacidade
disponível, durabilidade contra perda física ou comportamento de produção.
Após adicionar o teste, `make test` passou com Go 1.27.3 e Rust 1.99.1,
incluindo testes Go sem cache, Clippy, fuzz smoke, conformance e Floci com
restart. Os drills CP-01 não foram repetidos nessa execução.

## Quem controla os escritores

### Fronteira de saturação do spool

Os coordenadores Go e Rust tratam `spool_bytes == max_spool_bytes` como
saturação: `Health` degrada e `Flush` deixa a admissão fechada. Os testes
`TestCoordinatorSpoolLimitAtCapacityAndRecovery` e
`spool_limit_at_capacity_blocks_and_higher_limit_recovers` usam um snapshot
SQLite real, medem o spool, exercitam igualdade e demonstram a reabertura com
um novo coordenador configurado com limite um byte maior. Antes da correção,
ambos falharam porque `Health` reportava saudável. Após a correção, o teste Go
passou três vezes com `-race` e o teste Rust passou uma vez.

Essa fronteira é uma verificação posterior à captura, baseada em comprimentos
lógicos sob `ltx`: não reserva espaço antes de aceitar cada escrita, não conta
todo o staging e não impõe quota física. A recuperação testada altera o limite
configurado; não demonstra liberação de espaço via GC, nem recuperação após
crash.

Na continuação, a mesma fronteira foi corrigida para WAL em Go e Rust: o
tamanho igual ao máximo também degrada `Health` e impede reabertura após
`Flush`. `TestCoordinatorWALLimitAtCapacityAndRecovery` e
`wal_limit_at_capacity_blocks_and_higher_limit_recovers` criam WAL SQLite real,
verificam saturação e recuperação mediante aumento do limite. Ambos falharam
antes da correção e passaram depois; Go repetiu três vezes com race detector.
Nesses testes o adapter de captura é um stub que informa o TXID do snapshot,
portanto eles testam somente o limiar observado, não captura das novas escritas
nem proteção de dados. Tampouco demonstram reserva antecipada ou quota física.

O teste Rust `real_capture_exceeds_spool_limit_preserves_restore_and_recovers_with_headroom`
exercita captura SQLite/LTX real: após o commit, a captura publica TXID 2 e
ultrapassa o limite de spool; `Flush` retorna erro, mantém admissão fechada e o
restore por TXID recupera a alteração. Um novo coordenador com limite maior
completa o retry e reabre admissão. Esse resultado demonstra explicitamente que
o limite observado pode ser ultrapassado antes da rejeição; não é quota física,
liberação por GC, nem teste de crash. O adapter e o orçamento são de teste.
O teste Go `TestCoordinatorRealCaptureExceedsSpoolLimitAndRestores` verificou
o mesmo fluxo de captura, erro de backpressure, restore do TXID 2 e retry com
limite maior, três vezes com `-race`. O write gate é no-op no ensaio sequencial;
isso não valida coordenação de escritores concorrentes nem quota física.

Na medição Rust, erros de `metadata` distintos de `NotFound` agora são
propagados em vez de serem interpretados como diretório vazio por `exists()`.
O teste unitário com laço de symlink falhou antes da correção e passou depois;
também verifica que um caminho ausente retorna zero. Esse teste isolado não
prova exclusão de alterações externas concorrentes, contabilização de staging
ou detecção de symlinks quebrados classificados como `NotFound`.

### Reserva cooperativa experimental em Go

`replicator.ConservativeBudget` fornece contabilização process-local de um
recurso, ligada ao `AdmissionGate`: inicia sem ocupação conhecida, reserva
atomicamente custos informados, impede reconciliação com reservas ativas e
invalida ocupação antes de `WithOccupancyMutation`. Erros de medição, abandono
e custo subestimado mantêm reservas bloqueadas até reconciliação. Go não tem
destrutor determinístico: cada reserva exige `Settle` ou `Abandon`; esquecimento
mantém o permit ativo e pode impedir o drain. Callbacks são síncronos sob mutex e
não devem reentrar ou esperar por escritor que precisa do mesmo budget.

Os testes `TestConservativeBudget*` passaram três vezes com `go test -race`:
cobrem saturação concorrente, falha, abandono, cancelamento e reinicialização
com medição de arquivo temporário real. A escrita de teste não é captura LTX.
O coordenador e `CaptureOnce` ainda não integram automaticamente o budget, e
custos, todos os produtores, staging, recovery e filesystem não são cobertos por
ele automaticamente. Valores fornecidos pelo chamador precisam ser limites
conservadores e a medição deve excluir todas as mutações externas. Não é quota
física, reserva de espaço no disco, recuperação durável ou proteção contra
processos que não cooperam.
O estado de conclusão é compartilhado entre cópias de um `BudgetReservation`,
impedindo liquidação duplicada; o próprio `ConservativeBudget` contém mutex e
não deve ser copiado. O teste de cópia e os demais testes de budget passaram
três vezes com o detector de corrida.

`CaptureOnceWithBudget` oferece integração explícita experimental em Go: adquire
o write gate fora do mutex, executa a captura sob `WithOccupancyMutation` e
libera o gate fora do mutex. Em erro de aquisição, captura ou liberação,
invalida a ocupação; erro após a captura não desfaz LTX já durável. Um callback
de liberação pode reconciliar independentemente após captura bem-sucedida, de
modo que a aplicação precisa coordenar essa medição. Os testes de integração
`TestCaptureOnceWithBudget*` passaram três vezes com `-race`; um restaura dados
de SQLite real e outro verifica invalidação após callback de liberação que tenta
reconciliar durante uma falha. O gate de teste é sequencial, não prova proteção
de produtores externos, limites de staging nem quota de disco. O coordenador não
conecta automaticamente essa API.
Um teste adicional mantém uma reserva ativa, verifica que a captura é rejeitada
e a ocupação fica desconhecida, depois liquida, reconcilia e repete a captura.
Isso testa o contrato cooperativo, não a drenagem automática pelo coordenador.

`TestCooperativeBudgetCoordinatorRealCaptureAndFailedMeasurement` conecta o
budget e o mesmo `AdmissionGate` a um coordenador Go de teste. Uma escrita
SQLite real é capturada; falha de reconciliação ao reabrir mantém o gate e o
budget fechados, enquanto o restore do TXID 2 contém a alteração. Com medição
novamente bem-sucedida, o retry reabre a admissão. Passou três vezes com
`go test -race`. A medição retorna custo lógico artificial zero; o teste não
mede WAL ou staging, não comprova limites físicos e não homologa o adapter.

`WithReservedMutation` e `CaptureOnceWithReservedBudget` adicionam uma
verificação lógica **antes** de iniciar a mutação/captura Go: se ocupação mais
custo informado exceder o limite, rejeitam antes do callback; após iniciá-lo,
invalidam a ocupação até nova medição. Testes `TestReserved*` passaram três
vezes com `-race`, cobrindo rejeição antes de publicar watermark, erro de
staging simulado, reconciliação, retry e restore SQLite por TXID. O custo 6 e
as ocupações 5/4 nos testes de captura são unidades artificiais, não limites
de bytes reais. O método não verifica se o callback respeita o custo e não
reserva blocos físicos: o chamador deve fornecer limite conservador incluindo
recovery, staging e spool, fechar e drenar todos os produtores e coordenar
mutações externas. Portanto quota física e integração completa permanecem
pendentes.

Rust oferece agora os equivalentes `with_reserved_mutation` e
`capture_once_with_reserved_budget`. Testes direcionados passaram para rejeição
antes do callback, erro de mutação, reconciliação e retry, e para captura e
restore SQLite por TXID após uma rejeição anterior sem watermark. As unidades de
custo também são artificiais; as mesmas limitações de estimativa fornecida pelo
chamador e ausência de reserva ou enforcement físico se aplicam.

Em Go, `CaptureOnceWithRecoveryImageLimit` combina a pré-verificação lógica
experimental com limite efetivo por escrita para a imagem temporária que
`spool.Recover` decodifica do snapshot base. A escrita que ultrapassaria o
limite lógico é rejeitada antes de ser enviada ao arquivo; falha remove o
temporário e não avança o watermark. Os testes de arquivo, recovery e captura
real passaram três vezes com `-race`, incluindo retry e restore após aumentar o
limite. O limite cobre somente a imagem de recovery durante essa captura opt-in;
não cobre `Coordinator.Status`, backup SQLite de origem, WAL, metadados, demais
temporários, LTX nem tamanho físico alocado. Não reserva blocos e não é quota
de filesystem.

Rust oferece o equivalente opt-in `capture_once_with_recovery_image_limit`,
apoiado por `recover_with_image_limit` na crate de spool. Testes direcionados
verificaram rejeição antes da escrita excedente, remoção do temporário e
watermark inalterado após falha, além de retry e restore SQLite real. As mesmas
exclusões de staging, outros temporários, `Coordinator::status` e blocos físicos
se aplicam; não é quota de filesystem nem limite global do repositório.

Foi acrescentado no adapter Rust um Online Backup experimental com teto lógico
da imagem principal usando `PRAGMA max_page_count` na conexão de destino,
ajustada ao tamanho de página da origem. O teste com SQLite embutido criou um
blob de 128 KiB, verificou falha com limite de 8 KiB, tamanho do destino não
superior ao teto nesse cenário e backup/`integrity_check` e leitura corretos no
retry com limite maior. Outro teste rejeitou destino não vazio sem modificá-lo
e teto menor que uma página. Destino precisa ser novo/vazio. Não limita journals, outros arquivos, blocos
físicos ou espaço disponível, nem prova comportamento de outros drivers/VFS.
O caminho opt-in `capture_once_with_image_limits` agora usa esse backup limitado
junto ao limite de recovery. Teste de captura com SQLite real verificou erro com
limite de backup insuficiente, watermark no TXID base, retry com limite maior e
restore do TXID 2. A medição de custo usada é lógica/artificial; o teste não
prova quota global nem a posição da admissão em uma aplicação real.

No adapter Go, `TestOnlineBackupImageLimitModernc` verificou o driver `modernc`
com a conexão de destino criada por `NewBackup` e pragmas na URI: uma imagem
de 128 KiB falhou com erro SQLite de disco cheio sob teto de 8 KiB, e o retry
com teto maior preservou os dados. O teste também preservou um destino não
vazio e usou nomes com caracteres especiais; passou três vezes com `-race`.
O caminho opt-in `CaptureOnceWithImageLimits` conecta esse backup ao limite de
recovery. `TestCaptureWithImageLimitsBackupFailureAndRetry` passou três vezes
com `-race`: verificou erro SQLite de disco cheio, watermark no TXID base,
limpeza do arquivo de staging, invalidação do budget e retry com restore do
TXID 2. A medição do budget no teste é artificial. A evidência é limitada ao
runtime e cenário testados, não cobre journals nem assegura quota física.
`TestOnlineBackupImageLimitPageSizes` também passou três vezes com `-race` em
páginas de 512, 1024, 4096, 8192 e 65536 bytes, verificando falha SQLite e
comprimento lógico do destino dentro do teto de duas páginas. Isso amplia a
matriz local desse adapter, mas não demonstra quota para arquivos auxiliares.

Em Go, `spool.PublishTransactionsWithBatchLimit` verifica a soma dos bytes LTX
já codificados antes de criar o repositório ou qualquer staging do lote. O teste
`TestPublishTransactionsBatchLimitRejectsBeforeStagingAndRetries` passou três
vezes com `-race`: lote acima do teto não chama hooks nem cria o repositório;
no teto exato, publica e atualiza watermark. A função é interna e ainda não
estava isolada da captura; não limita alocação em memória para codificação,
spool preexistente, outros produtores, metadados ou blocos físicos. Não é uma
quota global nem prova recuperação após crash.
O caminho interno experimental `CaptureOnceWithBatchLimit` agora aplica a
verificação após codificação e antes do staging LTX. Com SQLite real, o teste
`TestCaptureBatchLimitRejectsBeforePublicationAndRetries` passou três vezes com
`-race`: limite insuficiente preserva o watermark no TXID 1 e retry com teto
maior publica TXID 2; o restore desse TXID recupera o valor alterado. O caminho
agora também está acessível pela API experimental Go `CaptureOnceWithBatchLimit`.
O teste de API `TestCaptureWithBatchLimitFailureAndRetry` passou três vezes com
`-race`, com erro de limite, watermark preservado e restore após retry. Os testes
não comprovam quota física ou limite de memória; o equivalente Rust e a
contabilização do spool existente permanece pendente.
Na crate de spool Rust, `publish_transactions_with_batch_limit` agora rejeita
lotes codificados acima do limite antes de criar staging. O teste direcionado
`batch_limit_rejects_before_staging_and_retries` passou: rejeição não cria o
repositório nem chama o hook; retry no limite exato publica o watermark. Esse
método está ligado à captura experimental Rust por
`capture_once_with_batch_limit`: o teste de API com SQLite real confirmou
rejeição, watermark preservado e restore após retry. Não contabiliza spool
existente, memória de codificação, outros produtores ou blocos físicos.
Testes adicionais em Go e Rust confirmam que a checagem soma todo o lote:
dois itens individualmente dentro do teto foram rejeitados antes de criar o
repositório. Os itens são duplicatas intencionalmente inválidas, portanto esse
ensaio prova a ordem e a soma da pré-checagem, não publicação de uma cadeia
multitransação válida. Go passou três vezes com `-race`; Rust passou uma vez.
Em Go, `TestPublishTransactionsBatchLimitRetriesAfterPublicationFailure` passou
três vezes com `-race` após injetar erro depois da publicação LTX e antes do
watermark: o retry idêntico no mesmo teto concluiu a publicação. O mecanismo
atual regrava um staging antes de comparar o arquivo imutável existente;
portanto um limite futuro do spool total deve contabilizar esse espaço
temporário mesmo em retries. Esse ensaio não simula crash ou quota de disco.
O teste Rust `batch_limit_retries_after_ltx_publication_failure` também passou:
uma falha injetada após publicação preservou o LTX sem avançar o watermark;
retry idêntico no teto do lote concluiu. Não é um teste de crash real.

As funções de spool `PublishTransactionsWithSpoolLimit` (Go, interna) e
`publish_transactions_with_spool_limit` (Rust) verificam os comprimentos
lógicos existentes sob `ltx` e o lote inteiro antes de publicar, contando
inclusive staging residual e staging de retry idêntico. Os testes direcionados
`TestSpoolLimitCountsExistingStagingAndRetry` (três vezes com `-race`) e
`spool_limit_counts_existing_staging_and_retry` passaram com falha injetada e
retry após ampliar o teto. O chamador deve manter a trava do repositório e
excluir todas as mutações durante medição e publicação. O caminho interno Go
`CaptureOnceWithSpoolLimit` agora executa a checagem sob a trava local; seu
teste com SQLite real passou três vezes com `-race`, preservando watermark
após rejeição e restaurando TXID 2 após retry. Ele não está exposto na API
pública original; há agora uma API experimental com limites separados. Rust expõe
`capture_once_with_spool_limit`, que combina limites individuais de imagem,
lote e spool: o teste de API com SQLite real verificou rejeição, watermark
inalterado e restore após retry. A API experimental Go
`CaptureOnceWithSpoolLimit` também combina limites individuais, com teste real
de rejeição, watermark e restore após retry passando três vezes com `-race`.
Os testes também verificam que o limite de lote continua ativo ao habilitar o
limite de spool; Go passou três vezes com `-race` e Rust uma vez. Usam medição
artificial do budget e não provam quota combinada dessas imagens e do spool.
As APIs experimentais `CaptureOnceWithDerivedReservation` (Go) e
`capture_once_with_derived_reservation` (Rust) derivam o custo reservado como
soma dos tetos de recovery, backup e lote LTX, com verificação de overflow.
Testes direcionados rejeitaram overflow e capacidade insuficiente: Go passou
três vezes com `-race` e Rust uma vez. A medição nesses testes é artificial;
testes posteriores com SQLite real mediram os arquivos `ltx` existentes e
resíduos deliberados em `ltx` e `.staging`, e
verificaram rejeição um byte abaixo da soma reservada, retry no limite exato e
restore do TXID 2 (Go três vezes com `-race`, Rust uma vez). Os testes agora
usam `MeasureRepositoryLogicalBytes` em Go e `measure_repository_logical_bytes`
em Rust: a medição de `ltx` e `.staging` mantém a trava local e rejeita diretório
LTX ausente e symlinks; staging ausente conta zero. Os testes de falha passaram
três vezes com `-race` em Go e uma em Rust. A trava não permanece entre medição
e captura: o chamador precisa excluir outros produtores nesse intervalo. Os
testes não demonstram concorrência nem cobertura de todos os produtores. Assim, a
reconciliação e coordenação de todos os produtores seguem sendo obrigação do
chamador. Journals, metadados e blocos físicos não estão cobertos.
Testes adicionais de contenção confirmaram que a medição rejeita uma trava local
já adquirida e funciona após sua liberação (Go três vezes com `-race`, Rust uma
vez). Eles não eliminam o intervalo entre liberar a trava de medição e iniciar a
captura nem cobrem escritores não cooperativos.
Go oferece agora `CaptureOnceWithMeasuredReservation`: fecha o gate do chamador
e mantém o mutex do mesmo budget desde a medição até o término da captura. O
teste real de spool e resíduos passou três vezes com `-race` sem reconciliação
prévia; um teste adicional verificou exclusão de reservas e reconciliação após
mutação. Na versão posterior, a trava filesystem é mantida continuamente da
medição até o fim da captura. O teste Go com SQLite real verificou falha quando
outro detentor mantém a trava e retry com restore após liberação, três vezes com
`-race`. Não demonstra todos os interleavings nem operações não cooperativas.
Rust agora oferece `capture_once_with_measured_reservation` com o mesmo limite
de coordenação. Um teste com SQLite real e resíduos verificou rejeição e retry
com restore; outro verificou exclusão de reservas e reconciliação após mutação.
Ambos passaram uma vez. Na versão posterior, Rust também mantém a trava local
continuamente entre medição e captura. O teste de API com SQLite real verificou
rejeição com a trava mantida por outro descritor e retry com restore após sua
liberação. Operações que respeitam a mesma trava são excluídas mesmo com budgets
distintos; o teste não cobre todos os interleavings nem escritores que ignoram a
trava. Nenhum desses caminhos é quota física.
Testes adicionais pausaram capturas SQLite após adquirir o lease e tentaram uma
segunda aquisição da trava: Go testou o caminho público medido com falha
injetada e retry (três execuções com `-race`); Rust testou a função interna sob
trava já adquirida, também com falha e retry (uma execução). Não representam
uma prova exaustiva dos interleavings nem testam processos não cooperativos.
O teste Rust `locked_capture_excludes_other_repository_operation_and_retries`
foi ampliado: enquanto a captura interna real está pausada sob a trava, uma
captura pela API pública medida com budget independente é rejeitada; após a
falha injetada e liberação, a mesma API publica TXID 2. A execução direcionada
offline passou. Este cenário não testa restore nem prova todas as concorrências.
Após essa ampliação, `make test` passou, incluindo formatação, vet, Clippy,
testes Go e Rust, exemplos de telemetria, fuzz, 96 restores cruzados e Floci
local. Os testes Go do módulo principal usaram cache; os drills CP-01 opt-in
não foram executados. Floci não constitui homologação de AWS S3 ou R2 reais.
Testes adicionais verificaram que spool ausente ou contendo symlink é rejeitado
sem avançar watermark: Go passou três vezes com `-race` e Rust uma vez em Unix.
Isso não testa corridas com mutações não cooperativas durante a varredura.
Não cobrem metadados,
memória, blocos físicos ou participantes não cooperativos, e não são quota.

| Mecanismo | Controle efetivo e limite observado |
|---|---|
| Go `replicator.WriteGate` e Rust `WriteGate` | São callbacks/trait fornecidos pela aplicação para pausar escritores durante `CaptureOnce`/`capture_once`. Os contratos exigem encaminhar todos os escritores da aplicação pelo gate. O repositório não pode impedir que outra conexão ou processo ignore esse mecanismo. O gate é liberado após sucesso ou falha da captura, de acordo com o fluxo da API; não é por si só o admission controller do coordenador. |
| Go `SetWriteAdmission` e Rust `CoordinatorAdapter::set_write_admission` | Exigem que o callback da aplicação pare novas escritas ao fechar e reserve capacidade atomicamente ao abrir. O coordenador fornece os limites, mas a correção da reserva e a cobertura de cada escritor dependem da implementação externa. A validação de presença de callback/trait não comprova que ele realmente bloqueia escritores. |
| Implementações no checkout | As implementações encontradas de `CoordinatorAdapter` estão em testes e no exemplo de telemetria; diversas registram flags ou são no-op. Em Go, os callbacks encontrados estão em testes e exemplos, inclusive callbacks no-op. O gate Rust `SingleThreadWriteGate` do exemplo de conformance é no-op adequado apenas ao cenário sequencial do exemplo. Não foi identificado admission controller de produção homologado ou integração real com pool de conexões. |
| Lease SQLite interna | O teste com SQLite real em `rust/crates/core/src/capture_overlap_test.rs` demonstrou `SQLITE_BUSY` para a segunda captura enquanto a primeira detinha a lease, no cenário testado. Ele chama captura interna com hook e **não** passa pelo `WriteGate` público; a lease não comprova reserva de capacidade nem controle completo de todos os produtores. |

## Cancelamento, concorrência e limites

- Em Go, `Flush` fecha admissão antes da captura e passa `ctx` ao callback de
  reabertura; `Run` tenta fechar com `context.Background()` ao observar
  cancelamento. A efetividade e atomicidade continuam dependentes do callback.
  A inspeção não demonstra ausência de corrida entre cancelamento e reabertura
  nem homologa escritores reais. Um callback de fechamento pode falhar ou
  bloquear; não foi comprovado tempo máximo de fechamento.
- Caracterização posterior em
  `go/replicator/admission_cancellation_characterization_test.go`: no callback
  sintético de reabertura, o próprio callback cancela o contexto, abre um gate
  booleano e permite uma inserção SQLite real com contexto independente, antes
  de retornar. `Run` depois observa o cancelamento e fecha novamente: transições
  `[false, false, true, false]`; a linha inserida permanece no banco. O teste
  passou três execuções com `-count=3`. O callback é deliberadamente não
  cooperativo com cancelamento e não representa um adapter homologado; o teste
  caracteriza uma janela possível, não garante segurança nem prova que todo
  callback Go aceite escritas após cancelar. Não demonstra o retorno de `Flush`
  isolado ou a durabilidade da inserção no spool/remoto. Os testes usaram banco
  temporário local no host com ambiente reduzido e dependências Go offline.
  A execução posterior com Go 1.27.3 e `go test -race ./replicator -run
  '^TestCharacterizeCancellationInsideAdmissionCallbackCanAdmitWrite$' -count=3
  -v` também passou três vezes, sem relatório do detector de corridas. O
  callback e a inserção do teste ocorrem sequencialmente; `-race` não elimina a
  janela lógica nem comprova segurança de callbacks concorrentes ou adapters
  de produção.
- Em Rust, **na implementação examinada em 2026-11-05**, `run` verificava `watch` antes da chamada de reabertura, mas o sinal e
  `set_write_admission` não compartilham protocolo atômico. O teste deliberado
  `cancellation_inside_admission_callback_never_reopens_writes` falha quando o
  callback sinaliza cancelamento e em seguida abre. A proposta de sincronização
  estava então não aprovada; não inferir que fechar depois elimine a janela.
  Ver atualização de 2026-11-06 acima para a API atual.
- Descartar a future Rust libera o lock de `Flush` enquanto uma captura já
  iniciada em `spawn_blocking` pode continuar. A caracterização com adapter
  sintético e o teste limitado de lease real estão em
  `coordinator-dropped-flush-evidence.md`; não comprovam serialização geral nem
  parada do trabalho em andamento. Publicação já durável não é revertida.
- Go e Rust validam que `MaxUploadsInFlight`/`max_uploads_in_flight` é positivo
  e entregam limites ao callback. A busca no código dos coordenadores encontrou
  somente declaração e validação desse campo, não um semáforo de uploads ali;
  não concluir que uma quota de uploads concorrentes foi homologada pela API.
  O comportamento dos demais componentes e a política de reserva precisam de
  validação ponta a ponta.

## Evidências e pendências de homologação

Na data da inspeção inicial, testes exercitaram transições de callback, captura local, Floci e
drills específicos; muitos callbacks são sintéticos e não bloqueiam uma escrita
real. A regressão Rust falhou no gate e, portanto, `make test` integral não
passou. Isso não equivale a um teste de saturação ou de segurança do gate de
uma aplicação.

Após adicionar a caracterização Go, `make test` com Go 1.27.3, Rust 1.99.1,
target Rust separado, `GOFLAGS=-count=1` e dependências offline passou formato,
`go vet`, Clippy e todos os testes Go; os unitários do core Rust tiveram 20
sucessos e quatro ignorados. O gate parou no teste Rust de admissão conhecido
(`admission reopened after cancellation was signaled`), com sete sucessos e uma
falha nesse alvo. Fuzz, conformance e Floci não foram alcançados nessa execução.
Uma tentativa inicial parou antes, no Clippy, por mistura de artefatos de Rust
1.94.1 e 1.99.1; `cargo fmt` acionou instalação automática não intencional da
toolchain 1.94.1 no `RUSTUP_HOME` padrão. A segunda tentativa definiu
`RUSTUP_TOOLCHAIN=1.99.1`, desabilitou instalação automática e usou target
separado. A instalação e os caches existentes foram preservados.

Antes de declarar M5 concluído, identificar os adapters e drivers realmente
suportados e testar, com escritores reais e múltiplas conexões, ao menos:
inicialização fechada; reserva atômica diante de concorrência e limites; cancelamento
durante reabertura e captura; callback reentrante, falha e timeout; fechamento
de eventos; saturação e retomada; e escritores fora do controle. Medir e
documentar o comportamento de cada adapter, sem prometer bloquear processos
externos. Exercitar também falha do storage, restart e confirmação remota,
distinguindo espera cancelada de trabalho já durável ou ainda em andamento.

A mudança de contrato para garantir a ordenação estrita entre cancelamento e
aceitação de escritas foi aprovada e está em implementação conforme a atualização
acima. Nenhum adapter de produção foi identificado no checkout. As garantias de
captura continuam restritas a Linux x86-64 e combinações SQLite/toolchains
efetivamente testadas; não há alegação de resistência a perda física de energia
ou cache, nem de compatibilidade universal S3/R2.
