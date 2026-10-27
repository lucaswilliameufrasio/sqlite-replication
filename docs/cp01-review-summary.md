# Resumo para revisão do gate de recuperação incremental

**Estado:** aguardando revisão explícita do mantenedor. Até essa revisão, a
captura WAL e a recuperação incremental continuam privadas; este resumo não é
aprovação automática.

## Escopo que você escolheu

- Começar com uma garantia restrita: Linux de 64 bits, Go e Rust nas combinações
  SQLite já testadas, e um único escritor coordenado pela biblioteca.
- Não prometer proteção contra perda física de energia/cache do host.
- Você fará a revisão final deste resumo antes de liberar a capacidade.

## O que foi observado nos testes

Em Linux amd64, Go com `modernc.org/sqlite` 3.54.1 e Rust com SQLite 3.53.2
bundled via `rusqlite`:

- As barreiras de teste bloquearam escritores enquanto a captura protegia o
  WAL; os commits posteriores foram reconciliados sem avançar o watermark sobre
  uma cadeia incompleta.
- O cenário de disco cheio usou um tmpfs de 8 MiB dentro de container rootless
  isolado. Go e Rust recuperaram e repetiram a captura depois que o espaço foi
  liberado.
- O processo foi encerrado enquanto um pedido de `fsync` aguardava um callback
  FUSE; Go e Rust recuperaram e repetiram a publicação.
- Um guest QEMU foi encerrado abruptamente sobre ext4/virtio-blk; Go e Rust
  recuperaram no reboot do guest.
- Testes também encerraram processos em limites de gravação/sync e durante
  checkpoint real; a reabertura e as verificações SQLite passaram.
- O gate local completo, incluindo conformance e Floci local, passou depois da
  troca do caminho público do módulo Go.

## Limites da evidência

- FUSE, tmpfs e QEMU não simulam corte físico de energia nem perda da cache
  volátil do host/controlador/dispositivo. Essa garantia está fora do escopo
  escolhido.
- Os drills CP-01 específicos foram executados em Linux amd64. Linux arm64 tem
  suites nativas aprovadas, mas não a mesma execução completa dos três drills
  neste checkout. macOS arm64 também tem suites nativas, não os drills Linux.
- Não foram homologados outros drivers SQLite/VFS, filesystems de produção,
  escritores não coordenados ou gravações diretas no sidecar.
- O drill de espaço cheio cobre tmpfs limitado, não todos os filesystems.
- Nenhum teste Floci representa homologação de AWS S3, Cloudflare R2 ou outro
  serviço externo.

## O que a revisão precisa decidir

1. Se a garantia inicial deve cobrir **somente Linux x86-64**, onde os drills
   CP-01 foram executados, ou se deve esperar que os mesmos drills também passem
   em Linux ARM64.
2. Se o escopo restrito acima e as exclusões desta página são suficientes para
   aprovar a conexão da recuperação incremental à API pública.

Até haver sua resposta explícita aos dois pontos, CP-01 permanece bloqueado e
nenhuma capacidade incremental será ligada à API pública.
