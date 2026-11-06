# ADR 0015: barreira cooperativa de flush

- **Status:** decisão de direção aceita; implementação e homologação pendentes
- **Data:** 2026-11-06
- **Substitui parcialmente:** semântica de alvo amostrado de `Flush` na ADR 0011

## Contexto

A ADR 0011 definiu que `Flush` alcança o maior TXID observado no começo da
chamada. O adapter SQLite ainda não possui uma fonte confiável desse alvo:
commits checkpointados podem ser consolidados em um único TXID de reconciliação.
Contar commits do WAL ou usar o último TXID durável como alvo não comprova que
as alterações já aceitas estejam capturadas. Fazer captura durante `Status`
também introduziria efeitos colaterais em uma consulta de observação.

## Decisão

O mantenedor aprovou substituir a semântica de alvo amostrado por uma barreira
cooperativa. O coordenador deve fechar a admissão, drenar produtores aceitos,
capturar um estado SQLite consistente e usar a posição efetivamente protegida
pela captura como alvo. Sucesso local requer spool durável correspondente;
quando configurado remoto, requer confirmação remota correspondente. Chamadas
concorrentes continuam serializadas, e falhas não revertem trabalho já durável.
O instante de fechamento da admissão, não o início da espera pelo lock de
flush, delimita os produtores abrangidos pela barreira.

A garantia depende de os produtores relevantes cooperarem com o gate e de a
captura estabelecer uma imagem consistente mesmo diante dos checkpoints que
suporta. Não cobre escritas externas não coordenadas nem prova quotas físicas.
Os detalhes de cancelamento, integração de adapter, compatibilidade de drivers e
limites conservadores exigem implementação e testes; não são homologados por
esta decisão. Até a implementação ser concluída, o código existente ainda usa
o contrato antigo e não deve ser apresentado como barreira funcional.

## Alternativa e consequências

Capturar antes de reconhecer cada escrita poderia estabelecer um TXID já
publicado, mas adicionaria captura ao caminho de confirmação de toda escrita.
A barreira evita prever o mapeamento entre commits SQLite e TXIDs LTX, ao custo
de fechar e drenar a admissão e executar captura mesmo sem alvo antecipado.
Nenhuma dessas opções elimina a necessidade de coordenar produtores e validar
falhas, concorrência, checkpoints, cancelamento e confirmação remota.

O mantenedor também autorizou exigir o `AdmissionGate` compartilhado para
cancelamento coordenado de escritores cooperativos. O `run_legacy` com
`watch::Sender` independente não oferece garantia atômica; a transição de API
e seus consumidores ainda precisam ser implementados e verificados.
