# LTX v2 — perfil de interoperabilidade

Este documento define o subconjunto LTX v2 usado pelo primeiro vertical. O
formato no disco usa o magic `LTX1`, compartilhado com v3; **a versão nunca é
inferida pelo magic**. O leitor deste perfil aceita apenas v2 e rejeita layouts
v3 explicitamente.

## Referência e proveniência

- Especificação de referência: `github.com/superfly/ltx` **v0.4.0** (tag
  publicada com `Version = 2`).
- O código de referência é permitido somente como ferramenta/oráculo de
  conformance, nunca como dependência de runtime nem como fonte de código a
  copiar.
- Litestream v0.5.x usa `superfly/ltx` v0.5.x, cujo `Version` é 3. Portanto, não
  é um oráculo válido para bytes v2. Fixtures devem registrar a versão exata do
  gerador v2 ou ser golden vectors independentes. A matriz externa de
  compatibilidade não deve chamar isso genericamente de “compatível com
  Litestream”.

## Perfil suportado

- Inteiros de largura fixa em big-endian; TXID e checksums são `uint64`.
- Página SQLite: potência de dois entre 512 e 65536 bytes; o valor `1` no
  header SQLite significa 65536.
- Header LTX: 100 bytes. Campos: `LTX1` (0..3), flags (4..7), page size
  (8..11), commit/page count (12..15), min TXID (16..23), max TXID (24..31),
  timestamp Unix em milissegundos (32..39), pre-apply checksum (40..47), WAL
  offset (48..55), WAL size (56..63), WAL salts (64..71), node ID (72..79) e
  20 bytes reservados zero (80..99).
- Neste perfil, flags são zero. Snapshot tem `MinTXID = 1`,
  `PreApplyChecksum = 0` e inclui todas as páginas `1..Commit`, exceto a página
  de lock SQLite (`0x40000000 / page_size + 1`). Um delta tem `MinTXID > 1`,
  `PreApplyChecksum` não-zero com `ChecksumFlag` e `MaxTXID >= MinTXID`.
- Cada frame consiste em `Pgno uint32` big-endian seguido por um LZ4 frame
  independente que descomprime para exatamente `PageSize` bytes. Não há campo
  de tamanho comprimido no header de página v2. A página 0 (quatro bytes zero)
  encerra a seção de frames.
- O page index contém, em ordem crescente de página, triplas de unsigned
  varints: page number, offset absoluto do frame no arquivo e tamanho codificado
  do frame. Varint de página zero encerra o índice; em seguida há um `uint64`
  big-endian com o tamanho das entradas, incluindo o terminador e excluindo
  esse próprio campo.
- Trailer: checksum pós-aplicação e checksum do arquivo, cada um `uint64`
  big-endian. O checksum do arquivo é CRC-64/ISO sobre header, headers de
  página, dados descomprimidos, terminador, índice e o checksum pós-aplicação;
  payloads LZ4 comprimidos e o próprio checksum final não entram no hash. O bit
  63 é a flag `ChecksumFlag`.
- Checksum de banco: para cada página (exceto lock page),
  `CRC64-ISO(BE32(pgno) || page_bytes)`; o checksum acumulado é a flag no bit
  63 OR do XOR dos checksums de página.
- O decoder valida faixa/ordem/índice das páginas, todos os comprimentos,
  checksums e ausência de bytes excedentes.

## Deltas

- `Commit` é o tamanho lógico do banco em páginas após aplicar o arquivo; zero
  representa a remoção do banco e exige checksum pós-aplicação igual a
  `ChecksumFlag`.
- Um delta lista somente páginas alteradas, em ordem estritamente crescente,
  com `Pgno <= Commit`. Não pode incluir a lock page. A seção pode estar vazia
  quando a transação só altera o tamanho lógico.
- `MinTXID..MaxTXID` é inclusivo. O checksum pré-aplicação deve corresponder ao
  estado após `MinTXID - 1`; o checksum pós-aplicação cobre o estado completo
  depois de aplicar todas as páginas e ajustar o tamanho para `Commit`.
- Para deltas originados de WAL, `WALOffset` é o offset absoluto do primeiro
  frame da transação e `WALSize` é o offset exclusivo imediatamente após o frame
  de commit correspondente (inclui o header WAL). Assim, um delta publicado
  individualmente identifica um limite recuperável sem avançar sobre commits
  posteriores presentes no mesmo arquivo WAL. Deltas de reconciliação por imagem
  não têm proveniência WAL e usam offset/tamanho/salts zero.
- Aplicar um delta exige verificar o checksum pré-aplicação, atualizar/truncar
  as páginas e validar o checksum pós-aplicação. Validar apenas o checksum do
  arquivo não prova continuidade com o banco-base.
- O perfil inicial não usa a flag `HeaderFlagNoChecksum`; cadeias sem checksum
  não são aceitas como replicação recuperável.

## Restore

O decoder escreve em destino temporário. A página de lock omitida é
reconstituída como zeros. O chamador só pode promover o arquivo após validação
do trailer e `quick_check` + `integrity_check` SQLite. Este perfil ainda não
define compactação, restauração por timestamp ou v3.

`DecodeSnapshot` só aceita snapshots. Aplicação de deltas é uma operação
separada que exige um banco-base correspondente e os checksums de posição.

## Fixtures

Fixtures devem cobrir page sizes 512, 1024, 4096, 8192, 32768 e 65536,
snapshot vazio/não vazio, lock page, truncamento, flags inválidas, checksum
alterado, índice inconsistente e arquivo com bytes extras. Os bytes da fixture
precisam ser verificados por implementação independente; round-trip da mesma
implementação não é prova de interoperabilidade.
