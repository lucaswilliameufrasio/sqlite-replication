# Protocolo de recuperação

## Escopo atual

M1 restaura somente o snapshot inicial LTX v2 para um novo arquivo. Não há
recuperação incremental, por TXID/timestamp, compactação nem retenção no código
atual. Uma lacuna nessa capacidade deve aparecer como indisponibilidade, não
como ponto recuperável.

## Descoberta e planejamento (incremental futuro)

1. Validar manifesto, `format_version`, `repository_id` e `database_id`.
2. Enumerar chaves imutáveis no layout do backend e decodificar cada header
   candidato sem confiar em manifesto mutável.
3. Construir grafo/cadeia por intervalos TXID inclusivos, geração, checksum
   pre/post e tamanho lógico; rejeitar gaps, forks e bases ausentes.
4. Selecionar o caminho completo até o alvo. `timestamp < alvo` é exclusivo;
   TXID é inclusivo. LTX compactado não pode ser parcialmente aplicado.
5. Produzir plano explicável contendo base, segmentos, alvo e validações; não
   mutar o repositório durante planejamento.

## Reconstrução

- Baixar/ler para staging isolado no diretório do destino, com tamanho limitado
  e cancelamento.
- Validar bytes LTX, índices, página, checksums, ordem e cobertura antes de
  promover.
- Aplicar páginas e truncamento lógico na ordem da cadeia.
- Rodar SQLite `quick_check` e `integrity_check` com runtime homologado; remover
  sidecars temporários incompatíveis.
- Sincronizar o arquivo e diretório, rejeitar destino ou sidecar preexistente e
  promover por operação atômica create-only.
- Falha em qualquer etapa preserva o destino anterior e remove apenas staging
  pertencente à operação.

## Alvos e limites

Alvos previstos: snapshot explícito, TXID inclusivo e timestamp exclusivo. A
precisão é a fronteira entre arquivos ainda retidos, não cada commit SQLite.
Retenção pode tornar pontos antigos indisponíveis; APIs devem indicar isso sem
inventar um estado restaurável.

## Evidência de teste

Testar corrupção/truncamento, alteração de checksum, página inválida, tamanho
lógico maior/menor, gap, fork, segmento duplicado, destino existente, sidecars
obsoletos, erro de leitura e falha de verificação. Nenhum caso de erro promove
arquivo parcial. CP-01 é pré-requisito para qualquer cadeia incremental.
