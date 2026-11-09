# Investigação da evidência de licença Go

Em 2026-11-05 repetimos o CycloneDX GoMod **1.12.0** com `mod -verbose -json
-licenses -type library`, em container rootless sem rede externa, Go 1.27.3 e
caches locais somente leitura. O SHA-256 do arquivo de release do gerador foi
`004b9f5cc595b797fb5423e2ae4c97bcf0f18c712ed2faee1640b09e5efd6d15`.
O log reportou `modernc.org/libc@v1.80.1`: MIT, confiança 0,9815951; e
`modernc.org/sqlite@v1.61.1`: Apache-2.0, confiança 1,0. Isso reproduz os
valores dos dois SBOMs originais, embora os arquivos principais `LICENSE` dessas
versões contenham textos BSD-3-Clause, conforme a revisão de notices.

O [detector local v1.12.0](https://github.com/CycloneDX/cyclonedx-gomod/blob/v1.12.0/pkg/licensedetect/local/detector.go)
usa `licensedb.Detect` sobre o diretório do módulo, ordena os candidatos e
retorna somente o de **maior confiança**, desde que atinja o limiar 0,85. Seu
log informa classificação e confiança, mas não a origem do match. Os módulos
contêm múltiplos arquivos e textos de terceiros; não foi demonstrado qual
arquivo produziu cada resultado. Uma classificação única de maior confiança não
descreve por si a licença principal nem todos os notices aplicáveis.

## Correção conservadora e reproduzível

Os originais `go.cdx.json` e `go-with-tests.cdx.json` não foram alterados.
O script `conformance/quarantine-sbom-license-evidence.py` gera cópias derivadas
removendo apenas as duas entradas controversas de `evidence.licenses`. Ele
exige CycloneDX 1.6, nomes e versões exatos, os valores esperados e exatamente
um componente de cada tipo; falha sem criar saída se a entrada não corresponder
e não sobrescreve arquivos. **Ausência de evidência não é licença desconhecida
no código upstream nem aprovação de conformidade.** Em particular, não se
substitui a evidência por BSD como se isso cobrisse SQLite, musl e todos os
avisos de terceiros. Os arquivos originais e a revisão técnica permanecem parte
da proveniência. As cópias retêm o serial e timestamp do original e devem ser
tratadas como derivadas dele, não como SBOMs independentes de novos builds.

```sh
python3 conformance/quarantine-sbom-license-evidence.py \
  docs/sbom/2026-11-05/go.cdx.json /path/new-go-quarantined.json
python3 conformance/quarantine-sbom-license-evidence.py \
  docs/sbom/2026-11-05/go-with-tests.cdx.json /path/new-go-tests-quarantined.json
```

As cópias `go-evidence-quarantined.cdx.json` e
`go-with-tests-evidence-quarantined.cdx.json` foram geradas desta forma. A
primeira foi gerada duas vezes e teve o mesmo SHA-256
`9aaaf6b6e269fd4caac087570d1ec8492660316db4ae2e719cf5c7f0751bd6f6`.
Comparação estrutural confirmou que apenas as duas entradas foram removidas em
cada cópia; uma entrada adulterada foi recusada sem criar saída. Ambas as cópias
passaram em `cyclonedx validate --input-format json --input-version v1_6
--fail-on-errors`, usando CycloneDX CLI 0.33.1 verificada por SHA-256
`bfc8b2538da86fe239bc53658bbb63c1c8c510a293c1e6891aa5bea5d3c58746`.

Esta quarentena elimina duas inferências potencialmente enganosas; não resolve
os avisos de terceiros, demais inferências automáticas, direitos de fixtures ou
notices para os artefatos de distribuição ainda não definidos. Não publicar ou
criar licença final, release ou tag com base nesta correção.

## Conferência externa das versões fixadas

O Go module proxy informa para `modernc.org/sqlite@v1.61.1` o repositório
`https://gitlab.com/cznic/sqlite`, tag `v1.61.1` e commit
`014351f110f66c8f485cff948fea608d7dcfafb1`; para `modernc.org/libc@v1.80.1`,
`https://gitlab.com/cznic/libc`, tag `v1.80.1` e commit
`612e8b0f2f1b1923d5c24c08bc233722500499cd`. Os `LICENSE` lidos nas
[tags sqlite](https://gitlab.com/cznic/sqlite/-/blob/v1.61.1/LICENSE) e
[libc](https://gitlab.com/cznic/libc/-/blob/v1.80.1/LICENSE) apresentam,
respectivamente, copyright The Sqlite Authors e The Libc Authors e as três
condições BSD; essa evidência apoia a leitura BSD-3-Clause dos arquivos
principais, em contraste com as classificações isoladas Apache-2.0 e MIT do
detector. Não implica que todos os arquivos de cada módulo sejam BSD.

As entradas `h1` de ambos os módulos e seus `go.mod` em `go/go.sum` coincidem
textualmente com os registros consultados em
[`sum.golang.org` sqlite](https://sum.golang.org/lookup/modernc.org/sqlite@v1.61.1)
e [libc](https://sum.golang.org/lookup/modernc.org/libc@v1.80.1). Essa
comparação de registros não recalculou o hash dos módulos nem verificou a
cadeia de transparência; também não demonstra composição de um artefato.
Tentativa de baixar os `LICENSE` por commit com Python no host recebeu HTTP
403, portanto não foi feita nesta etapa comparação byte a byte com o cache
local; os hashes locais já constam em `native-notices-review.md`. Os textos
de terceiros e a discrepância entre a matriz sqlite e a versão libc escolhida
permanecem sujeitos à revisão por artefato. Os SBOMs originais e em quarentena
foram preservados sem modificação; não se escolheu licença final.
