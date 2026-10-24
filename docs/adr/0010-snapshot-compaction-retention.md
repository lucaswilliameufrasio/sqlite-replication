# ADR 0010: snapshots, compactação e retenção

- **Status:** Proposta; ainda não congelada nem implementada
- **Data:** 2026-10-23

## Contexto

Histórico integral aumenta custo de restore e uso de storage. Compactação e GC
podem apagar o único caminho para um ponto de recuperação.

## Proposta

Snapshots periódicos imutáveis formam novas bases. Compactação cria e valida
arquivo de nível superior antes de remover fontes. Retenção futura parte da
proposta de 7 dias, ao menos dois snapshots completos verificados e horizonte
fino configurável. GC usa grafo de referências/restore points prometidos; expõe
`dry_run` e plano explicável e nunca exclui o último caminho necessário.

## Alternativas rejeitadas

- Apagar por idade ou prefixo sem conhecer a cadeia: pode remover bases ainda
  necessárias.
- Tratar compactação como checkpoint SQLite: são operações diferentes.
- Fixar os defaults de 7 dias/dois snapshots antes de medições e requisitos
  operacionais: seria uma promessa não validada.

## Consequências e evidência pendente

Defaults permanecem proposta. Definir contrato de restore points, grafo, dry-run,
concorrência com upload e crash recovery; testar que nenhum objeto necessário é
coletado e que cada snapshot/compactado restaura. M4 depende dessa prova.

## Reversão / reconsideração

Não apagar objetos durante rollout do GC. Começar em dry-run; só habilitar
remoção após validação em cópia de repositório e plano de rollback.
