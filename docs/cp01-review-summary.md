# Resumo para revisão do gate de recuperação incremental

**Estado:** os drills locais descritos abaixo foram executados no escopo
restrito. O plano e a ADR 0004 registram aprovação desse escopo, mas a versão
anterior deste resumo ainda exigia revisão explícita para liberar a capacidade;
essa divergência não estabelece, por si só, uma aprovação de uso ou distribuição.
A conclusão da validação local é separada dessas decisões do mantenedor.

## Escopo que você escolheu

- Começar com uma garantia restrita: Linux de 64 bits, Go e Rust nas combinações
  SQLite já testadas, e um único escritor coordenado pela biblioteca.
- Não prometer proteção contra perda física de energia/cache do host.
- A revisão para eventual liberação é uma decisão separada da validação local.

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
- Após o push autorizado, a GitHub Actions passou nos gates gerais do push
  (`40592146552`) e nos jobs nativos Linux ARM64 (`40592150987`, 6m27s) e macOS
  ARM64 (`40592150987`, 10m31s). Os jobs verificaram a arquitetura real e
  executaram `make test-native` para Go, Rust e conformance.

## Limites da evidência

### Reexecução local no commit `a8250ac`

Os três alvos opt-in foram repetidos no host Linux x86-64: `make
test-cp01-enospc`, `make test-cp01-fsync-interrupt` e `make
test-cp01-qemu-poweroff` passaram para Go e Rust. O ENOSPC usou tmpfs de 8 MiB
em container rootless; no FUSE cada processo publicador foi encerrado enquanto
o callback de `fsync` estava bloqueado, seguido de recuperação e nova tentativa.
No QEMU/KVM, o processo da VM foi encerrado após ambas as capturas terem sido
sincronizadas e validadas; a recuperação foi testada em ext4 sobre virtio-blk.
Isso não testa interrupção do QEMU durante a sincronização nem perda física de
energia ou de cache de hardware, e não constitui homologação de produção.

Uma primeira tentativa parou antes dos testes por conflito entre ferramentas Go
1.27.1 e 1.27.3 no ambiente herdado; a execução válida usou ambiente limpo,
Go 1.27.3 e Rust 1.99.1. O escopo inicial e a necessidade de revisão explícita
permanecem inalterados.

- FUSE, tmpfs e QEMU não simulam corte físico de energia nem perda da cache
  volátil do host/controlador/dispositivo. Essa garantia está fora do escopo
  escolhido.
- Os drills CP-01 específicos foram executados em Linux amd64. Linux arm64 tem
  agora os gates nativos aprovados, mas `make test-native` não executa os três
  drills de falha CP-01. macOS arm64 também passou o gate nativo, não os drills
  Linux. A garantia inicial escolhida continua somente Linux x86-64 até que os
  drills CP-01 apropriados sejam executados e revisados para ARM64.
- Não foram homologados outros drivers SQLite/VFS, filesystems de produção,
  escritores não coordenados ou gravações diretas no sidecar.
- O drill de espaço cheio cobre tmpfs limitado, não todos os filesystems.
- Nenhum teste Floci representa homologação de AWS S3, Cloudflare R2 ou outro
  serviço externo.

## Estado da revisão

Você escolheu a garantia inicial restrita a **Linux x86-64** e aprovou esse
escopo; ARM64 fica fora por enquanto. A validação de Actions amplia a evidência
de compatibilidade nativa, mas não altera essa fronteira porque não repetiu os
drills de falha CP-01 em ARM64. A decisão de escopo não cria por si só uma
implementação pública nem autoriza release/publicação; esses itens continuam
sujeitos aos gates M5/M6 e às evidências acima.

A escolha de escopo e os resultados locais não resolvem a divergência documental
sobre aprovação final. O estado de APIs existentes deve ser verificado no código;
este documento não afirma que sejam privadas. Não se infere autorização de uso em
produção ou distribuição a partir dos drills locais.
