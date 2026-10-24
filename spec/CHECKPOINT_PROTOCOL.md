# Protocolo de captura e checkpoint — CP-01 pendente

> **Estado: bloqueado / não aprovado.** A captura, o spool durável e a
> recuperação têm implementação interna e evidência local, mas não estão ligados
> à API pública e não executam checkpoint. Esta evidência parcial não autoriza
> alegar cobertura incremental saudável nem habilitar M2.

## Invariante a provar

Antes de uma operação poder reiniciar, truncar ou reutilizar WAL, todo commit
elegível precisa estar copiado, validado e durável em staging/spool. Nenhuma
janela entre liberação e nova fixação pode perder commits, trocar gerações ou
aceitar bytes de um ciclo de salts diferente. Se a prova/condição falhar, não
checkpointar e não reportar cobertura incremental saudável.

## Candidato de implementação

O desenho a validar usa adapter SQLite homologado, conexões gerenciadas e
políticas de conexão explícitas. O leitor fixa uma transação de leitura que
delimita snapshot/frames; o parser próprio valida header, checksum, salts e
commits; o encoder publica cada LTX completo e durável antes de liberar a
proteção. A rotação/checkpoint é solicitada somente sob locks e precondições
demonstrados. A leitura fixa, isoladamente, **não** é a prova.

Autocheckpoint é desligado em todas as conexões gerenciadas pelo adapter. As
conexões de teste também usam `busy_timeout=0`. Checkpoint continua sendo uma
operação externa/experimental: a captura não o solicita. Uma conexão não
gerenciada, driver diferente, VFS diferente, lock incompatível, erro real de
write/fsync ou estado WAL inesperado ainda precisa bloquear qualquer operação
destrutiva. `wal_hook` é apenas sinalização, nunca fonte única de verdade.

Cancelamento da captura é cooperativo: Go verifica o `context.Context` e Rust
usa um token atômico interno entre passos do Online Backup e limites de
publicação; uma syscall já em andamento não é preemptada. Antes da promoção
durável do watermark, cancelamento retorna erro, pode deixar LTX imutáveis
parciais e não avança o watermark; uma tentativa posterior reconcilia a cadeia.
Depois que o watermark e seu diretório foram sincronizados, a operação é
considerada concluída mesmo se o cancelamento chegar nessa fronteira final.

## Evidência local — parcial, não aprova CP-01

No Linux desta execução, os testes exercitaram Go/modernc SQLite **3.54.1** e
Rust/rusqlite bundled SQLite **3.53.2**, em filesystem local. O código exercitado
inclui `go/internal/capture`, `go/internal/walcapture`, `go/internal/spool`,
`go/internal/sqliteadapter` e os módulos Rust `core::capture`, `wal`, `spool` e
`sqlite-adapter`. `CaptureOnce` permanece interno e não executa checkpoint.

| Cenário coberto | Go | Rust | Escopo observado |
| --- | --- | --- | --- |
| `BEGIN IMMEDIATE` + reader pin; writer separado tenta iniciar | passou | passou | Writer bloqueado durante a lease; após sua liberação, writer separado consegue confirmar. |
| Barreiras determinísticas entre leitura, staging, publicação, liberação e nova captura | passou | passou | Writers recebem `SQLITE_BUSY` após leitura WAL, snapshot, tradução, publicação e no intervalo exato entre liberar o reader pin e o writer guard; `TRUNCATE` não completa nesse intervalo; watermark TXID 2 existe antes do checkpoint; após liberar o guard, novo commit é confirmado, truncado e reconciliado na próxima captura como TXID 3 (`WALOffset=0`). |
| `NOOP`, `PASSIVE`, `FULL`, `RESTART` e `TRUNCATE` durante guard/pin | passou | passou | `NOOP` é observado; os outros modos não modificam os bytes enquanto a lease está ativa. |
| Tradução de WAL produzido pelo runtime real | passou | passou | Checksums/página lógica comparados a Online Backup do SQLite. |
| Vários commits, páginas repetidas, crescimento/redução, salts, prefixo truncado/alterado e cauda parcial | passou | passou | Limites commitados preservados; fronteira estrita rejeita prefixo de recuperação inválido. |
| `TRUNCATE` entre ciclos e reconciliação por imagem consistente | passou | passou | Commit já checkpointado entra como delta sem falsa proveniência WAL. |
| Ordem `stage sync → LTX imutável → watermark atômico`; retry e chain recovery | passou | passou | Falha após uma publicação de várias não salta os commits restantes. |
| Erros injetados nos limites de write/fsync | passou | passou | Hooks simulam erros ENOSPC/EIO; não provocam falha real de write/fsync. ENOSPC real em tmpfs está na linha seguinte. |
| ENOSPC real durante publicação parcial e recuperação | passou | passou | `make test-cp01-enospc`, Linux amd64, container rootless e tmpfs limitado a 8 MiB: falta de espaço durante a publicação, watermark não avança sobre a cadeia parcial, lease é liberada e nova captura recupera o WAL após liberar espaço. Não prova comportamento de filesystems de produção nem falha física de `fsync`. |
| Cancelamento cooperativo antes e durante publicação | passou | passou | Testes cancelam após o primeiro LTX e após o `fsync` do watermark staged, antes da promoção: retornam `context.Canceled`/`Interrupted`, conservam o watermark anterior, liberam a lease e a próxima captura reconcilia a cadeia. Cancelamento após watermark+diretório duráveis retorna sucesso. Syscalls em andamento não são preemptadas. |
| Conexão de aplicação não gerenciada, busy timeout e autocheckpoint | passou | passou | Conexão separada com `busy_timeout=250ms` e `wal_autocheckpoint=1` recebe `SQLITE_BUSY` durante a lease; depois da captura confirma, checkpointa/trunca e a captura seguinte reconcilia o commit como delta sem proveniência WAL. Limita-se ao runtime/driver desta linha. |
| Processo filho morto com `SIGKILL` nos limites de stage, publicação e watermark | passou | passou | No Linux, restart recupera sem perder commit nem duplicar LTX já publicado. |

O gate local `make test` passou nesta execução: formatação, `go vet`, Clippy com
warnings negados, suites Go/Rust, conformance Go↔Rust e `superfly/ltx v0.4.0`, e
smoke Floci isolado. Os diretórios temporários da execução foram colocados em
Btrfs local (`/mnt/oioi`), em Linux amd64. Os testes de captura foram repetidos
25 vezes em Go e 20 vezes em Rust; a barreira reader-pin/writer-guard foi
repetida 20 vezes em cada engine. Isso não equivale à matriz de produção abaixo.
O drill opt-in `make test-cp01-enospc` também passou em ambos os engines no mesmo
host, provocando ENOSPC real em tmpfs de 8 MiB e verificando publicação parcial,
watermark sem avanço e recuperação após liberar espaço.
Os testes de `SIGKILL` são testes de processo em Linux; não são teste de perda
de energia nem de comportamento do filesystem após falha física.

Na implementação SQLite 3.51.3 consultada, `SQLITE_CHECKPOINT_NOOP` não toma o
checkpoint lock nem o writer lock: o fluxo pula a aquisição desses locks e lê o
wal-index. Portanto NOOP é uma observação, não uma barreira de sincronização.
Referência: [`wal.c`, `sqlite3WalCheckpoint()`](https://github.com/sqlite/sqlite/blob/version-3.51.3/src/wal.c#L4314-L4394).

## Matriz CP-01 ainda obrigatória

Use barreiras determinísticas/test hooks para pausar exatamente em cada
transição; `sleep` ou stress probabilístico sozinho não é prova.

1. Writer confirma entre leitura do limite e staging; verificar que o commit
   fica no LTX ou continua protegido para a próxima leitura em cada adapter/VFS.
   **Linux amd64 Go/Rust:** a barreira determinística agora prova que o writer
   fica bloqueado em cada fase e que o commit posterior, mesmo checkpointado
   antes da próxima captura, é reconciliado. Repetir nas demais combinações.
2. Writer confirma imediatamente antes/depois da liberação do read pin e antes
   da nova fixação; não há janela perdida. **Linux amd64 Go/Rust:** barreira
   determinística entre liberar o reader pin e o writer guard confirma `BUSY`;
   commit após liberação completa e antes da próxima captura, seguido de
   `TRUNCATE`, é reconciliado. Repetir nas demais combinações.
3. Checkpointer tenta `PASSIVE`, `FULL`, `RESTART` e `TRUNCATE` com readers
   longos e writers ativos; resultados `BUSY`/contagens incompletas nunca são
   tratados como sucesso. **Linux amd64 Go/Rust:** testes existentes passaram;
   o novo teste também confirma que `TRUNCATE` não completa antes da liberação.
4. Reset/reuse do WAL, salts/checkpoint sequence alterados, cauda incompleta,
   arquivo truncado e alteração concorrente durante read são detectados.
5. `SIGKILL` nos pontos antes/depois de stage fsync, publicação e watermark foi
   exercitado nos runtimes/OS locais. Ainda falta cobrir a fronteira de
   checkpoint (não existe operação de checkpoint na captura), reinicialização
   sob cada filesystem/VFS homologado e validação independente do banco final.
6. Hooks simulam falhas de write/fsync; o drill agora provoca `ENOSPC` real em
   tmpfs limitado no Linux amd64 e verifica recuperação, mas ainda faltam falhas
   físicas de `fsync` e interrupção de syscalls em andamento. Uma conexão
   externa de aplicação com timeout/autocheckpoint passou nos dois engines;
   ainda faltam drivers/VFS alternativos, outras configurações de conexão e os
   caminhos completos de busy timeout/reabertura exigidos para produção. O
   cancelamento cooperativo entre fases passou nos dois engines.
7. Rodar para cada runtime SQLite/driver/OS explicitamente homologado e
   verificar banco restaurado contra os commits de origem.

### Resultado atual da matriz

| Plataforma da decisão D-16 | Resultado nesta execução |
| --- | --- |
| Linux amd64 — produção inicial | Testado em Btrfs local; Go `modernc.org/sqlite` 3.54.1 e Rust `rusqlite` bundled SQLite 3.53.2. Quality gate `make test` e drill ENOSPC em tmpfs passaram; CP-01 segue bloqueado. |
| Linux arm64 — produção inicial | `make test-native` passou em runner nativo `ubuntu-24.04-arm`, despacho manual 40113447285, commit `5aae79a`; o drill ENOSPC continua limitado a Linux amd64. |
| macOS arm64 — integração | `make test-native` passou em runner nativo `macos-15`, despacho manual 40113447285, commit `5aae79a`. |
| Windows 11 amd64 — fase posterior | Fora da primeira matriz D-16; não verificado. |

Permanecem sem prova: drivers/VFS alternativos e conexões externas além da
configuração de aplicação testada; autocheckpoint global da aplicação;
escrita direta ou concorrente no sidecar fora do lock SQLite; falha física de
`fsync`/energia e ENOSPC em filesystems de produção (a evidência de espaço
cheio é somente em tmpfs);
interrupção de syscalls em andamento; crash durante e recuperação de uma
operação real de checkpoint; falha de energia/VFS; e a matriz ampliada de
drivers/VFS/filesystems. Os testes nativos arm64/macOS passaram, mas não cobrem
essas lacunas. Os hooks provam ordenação e recuperação em pontos controlados,
não atomicidade de hardware ou durabilidade de todos os filesystems. O gate
segue bloqueado até haver evidência completa e revisão do protocolo.

## Critério para aprovação

Registrar sequência de locks/transações, versões, combinação driver/VFS/OS,
hooks/PRAGMAs, código exercitado, seed/carga, pontos de injeção e resultado de
cada caso. Exigir zero commits elegíveis perdidos, nenhum falso sucesso de
checkpoint e suites determinísticas repetidas em toda combinação homologada. A
aprovação deve ser revisada antes de conectar o protocolo à API pública. Até
lá, CP-01 permanece reprovado por ausência de evidência completa; M2 e promessas
de streaming permanecem bloqueadas.
