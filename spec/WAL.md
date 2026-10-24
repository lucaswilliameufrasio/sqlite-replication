# SQLite WAL — contrato de leitura

## Escopo e versão

Este contrato descreve o parser de arquivo WAL para SQLite WAL-format version
`3007000`. Ele define reconhecimento de bytes, não o protocolo de captura
concorrente nem autorização para checkpoint. A captura continua bloqueada por
[`CHECKPOINT_PROTOCOL.md`](CHECKPOINT_PROTOCOL.md) e pelo gate CP-01.

Fonte normativa de formato: [SQLite Database File Format, seção 4](https://sqlite.org/fileformat2.html#walformat)
e `src/wal.c` da release SQLite 3.51.3. O arquivo `-shm`/wal-index é estado
volátil de coordenação e nunca é uma fonte de backup.

## Cabeçalho WAL

O cabeçalho tem 32 bytes; todos os seus campos serializados são `uint32`
big-endian:

| Offset | Campo | Regra |
|---:|---|---|
| 0 | magic | `0x377f0682` ou `0x377f0683` |
| 4 | formato | exatamente `3007000` no perfil inicial |
| 8 | page size | potência de 2, 512–65536 inclusive; 65536 é armazenado literalmente |
| 12 | checkpoint sequence | metadado; não é TXID da aplicação |
| 16, 20 | salts | copiados para cada frame |
| 24, 28 | checksum acumulado | checksum sobre os primeiros 24 bytes |

O bit menos significativo do magic seleciona a ordem das palavras de checksum:
`0x377f0682` usa palavras little-endian; `0x377f0683` usa palavras
big-endian. Os valores calculados continuam serializados big-endian.
Não aplicar ao WAL header a codificação `1` do cabeçalho principal SQLite para
representar 65536; o WAL header armazena o tamanho real como `uint32`.

O checksum WAL é o par `s0,s1` de somas `uint32` com wraparound, inicializado
em zero no cabeçalho. Para cada par de palavras `x[i], x[i+1]` na ordem
selecionada: `s0 += x[i] + s1`; depois `s1 += x[i+1] + s0`. O cabeçalho cobre
seus 24 bytes iniciais. Cada frame continua o par anterior usando os primeiros
8 bytes do frame header e, em seguida, todos os bytes da página.

## Frames e commits

Cada frame tem `24 + page_size` bytes. Os seis campos do frame header são
big-endian:

| Offset | Campo | Regra |
|---:|---|---|
| 0 | page number | entre 1 e `0xfffffffe` |
| 4 | database size | zero em frame não-commit; não-zero marca commit e tamanho lógico em páginas (`<= 0xfffffffe`) |
| 8, 12 | salts | devem corresponder aos salts do cabeçalho |
| 16, 20 | checksum | par rolling após os primeiros 8 bytes do header e os dados da página |

Frames só são elegíveis se a sequência de checksums e salts permanecer válida.
O parser pára na primeira cauda incompleta ou inválida; não procura bytes
posteriores como se fossem um novo WAL. Apenas o prefixo até o último frame com
database-size não-zero é um limite de commit. Frames depois desse limite são
transação incompleta e não podem compor um LTX.

O limite identifica um conjunto ordenado de frames; não fornece sozinho um
TXID global. A posição persistida deverá combinar geração/epoch, checkpoint
sequence, salts, offset/frame e limite lógico do banco, conforme
`REPOSITORY_LAYOUT.md` e `CHECKPOINT_PROTOCOL.md`.

## Aplicação lógica

- Aplicar frames de uma transação na ordem do WAL; a última ocorrência de uma
  página vence dentro da transação.
- O tamanho indicado pelo frame de commit define truncamento/crescimento lógico
  do banco naquele limite.
- Nunca persistir frames sem commit como uma posição recuperável.
- Um salto de salt, reset, truncamento ou checksum inválido é uma fronteira que
  exige ressincronização/erro; nunca continuar por heurística.
- Não interpretar o wal-index ou `-shm` como prova de persistência ou como
  fonte portátil entre processos/arquiteturas.

## Separação do gate CP-01

Um parser correto não impede que SQLite reutilize/trunque o WAL enquanto o
replicador o lê. Até que `CHECKPOINT_PROTOCOL.md` tenha uma prova aprovada e
testes concorrentes determinísticos, crash e falha de spool, o parser não pode
ser conectado a checkpoint automático ou apresentado como replicação segura.
