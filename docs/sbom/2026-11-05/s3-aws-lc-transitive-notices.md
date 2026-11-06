# S3 Rust e AWS-LC: triagem parcial de notices

Retrato do checkout e lockfile em 2026-11-05, Linux x86-64. Não é parecer
jurídico nem lista final de atribuições. Os artefatos de distribuição e suas
features não foram definidos; um grafo Cargo não comprova quais bytes ou funções
estão presentes num binário específico.

## Grafo selecionado

`cargo tree --offline --locked -p sqlite-replication-storage-s3 -i aws-lc-sys
--target x86_64-unknown-linux-gnu` identificou o caminho
`aws-sdk-s3 1.157.0 → aws-smithy-runtime 1.16.2 →
aws-smithy-http-client 1.5.2 → rustls 0.23.46 → aws-lc-rs 1.19.0 →
aws-lc-sys 0.47.0`. Também há caminhos via `hyper-rustls 0.27.10`,
`tokio-rustls 0.26.6` e `rustls-webpki 0.103.16`. Outras dependências e
configurações de consumidor podem alterar o grafo. A árvore não determina por
si só a composição do pacote extraído, código nativo vinculado ou imagem final.

Os manifests locais declaram Apache-2.0 para `aws-sdk-s3` e
`aws-smithy-http-client`, `Apache-2.0 OR ISC OR MIT` para `rustls` e
`hyper-rustls`, ISC para `rustls-webpki`, e
`ISC AND (Apache-2.0 OR ISC)` para `aws-lc-rs`. Seus textos locais de licença
foram identificados; a revisão completa dos avisos de todas as dependências do
grafo permanece pendente. Escolhas de licença e aplicabilidade exigem revisão por
artefato, não dedução apenas do manifesto.

## AWS-LC e arquivos presentes

O manifesto de `aws-lc-sys 0.47.0` declara
`ISC AND (Apache-2.0 OR ISC) AND Apache-2.0 AND MIT AND BSD-3-Clause AND
(Apache-2.0 OR ISC OR MIT) AND (Apache-2.0 OR ISC OR MIT-0)`. Seu `LICENSE`
composto, SHA-256
`728536b4160e051f86d7c9c388f704866b3d512cd7df97ac3516c65279523c4e`,
identifica contribuições AWS-LC, BoringSSL, OpenSSL, mlkem-native, mldsa-native,
fiat-crypto, s2n-bignum, jitterentropy e referências Keccak/AES. O notice e os
textos completos aplicáveis devem ser revisados, não substituídos por esta lista.

Uma busca recursiva por nomes `LICENSE*`, `NOTICE*` e `COPYING*` na crate
encontrou apenas o arquivo acima, `aws-lc/LICENSE` (SHA-256
`977c541bd25ffe36975dde25c028c0cd15ddf3d08243983b58e8a6a3d9447523`)
e `aws-lc/third_party/fiat/LICENSE` (SHA-256
`43e358d7b6eb109d0f51f7b3a090fd82607965767c25fadee39e922475de2061`).
Este último é MIT e identifica os autores fiat-crypto.

O notice referencia caminhos relativos à árvore `aws-lc`: o arquivo fiat existe,
mas os caminhos `third_party/s2n-bignum/s2n-bignum-imported/LICENSE` e
`third_party/jitterentropy/jitterentropy-library/LICENSE` não constam nas
respectivas árvores locais, embora diretórios e fontes estejam presentes. Também
não foram encontrados os arquivos referidos para Google Test e Wycheproof na
busca por nomes. Ausência na crate local **não** prova ausência de termos ou
de fontes em outros pacotes upstream. Obter e conferir as versões upstream
correspondentes e seus textos integrais antes de fechar atribuições.

### Rastreio posterior das licenças ausentes

O `.cargo_vcs_info.json` da crate identifica o commit aws-lc-rs
`5d496f88b40f327e28315deecc1a5a1fd1e81e95`. A entrada de submódulo
`aws-lc-sys/aws-lc` nesse commit aponta para AWS-LC
[`cd481c207e38f06aeb5766f4a1cdfce10a4c0af8`](https://github.com/aws/aws-lc/tree/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8).
Nesse commit os dois arquivos de licença estão presentes, apesar de ausentes na
crate extraída localmente:

| Componente | Evidência de versão do import | Licença na árvore AWS-LC fixada |
|---|---|---|
| s2n-bignum | [`META.yml`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/third_party/s2n-bignum/META.yml): commit `4d1356a7470663c752660a59375dc3a9ef548428` | [`LICENSE`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/third_party/s2n-bignum/s2n-bignum-imported/LICENSE); SHA-256 `41c6380384dc6065456d01405ef0b43e5fe39ba1bccc4ec67801cc66142728e5`; textos Apache-2.0, ISC e MIT-0. |
| jitterentropy | [`META.yml`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/third_party/jitterentropy/META.yml): commit `c90ff465dcdb6a6949542c72a26a8ab496daa8cb`, target `v3.6.3` | [`LICENSE`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/third_party/jitterentropy/jitterentropy-library/LICENSE); SHA-256 `088d208b9d22530691f1e9406a85dd7f6f5a42e3cf23d3d6df6582408e3840f7`; BSD de três condições ou alternativa GPL-2 conforme o texto. |

Os dois textos obtidos da árvore AWS-LC são idênticos byte a byte aos arquivos
`LICENSE` dos respectivos commits de origem indicados nos metadados:
[`s2n-bignum`](https://github.com/awslabs/s2n-bignum/blob/4d1356a7470663c752660a59375dc3a9ef548428/LICENSE)
e [`jitterentropy`](https://github.com/smuellerDD/jitterentropy-library/blob/c90ff465dcdb6a6949542c72a26a8ab496daa8cb/LICENSE).
O BSD de jitterentropy exige, entre outros pontos, preservar copyright,
condições e disclaimer nas formas pertinentes; consultar o texto completo.
O README do import AWS-LC menciona `v3.4.0`, divergindo do `META.yml` que
registra `v3.6.3`; a razão não foi apurada e não se assume que todo arquivo
vendorizado corresponda sem alterações ao commit de origem.

Como amostra de correspondência com a crate local, `git hash-object` retornou
`c25cd5e2bb4f7a8da61f0f8c746de92e33d7a17a` para
`jitterentropy-base.c` e `f26d3eb3c1bd0fc1df6ddab042a98e8a752482ae`
para `s2n-bignum.h`, iguais aos blobs nas mesmas posições do commit AWS-LC.
São dois arquivos de amostra, não uma verificação integral da árvore, nem
prova de que esses componentes sejam vinculados ao artefato distribuído. O
importador de s2n-bignum também descreve alterações locais após a importação.
Os links fixados e hashes constituem evidência técnica, não um conjunto final
de notices nem confirmação jurídica de sua aplicabilidade.

### Histórico da divergência e evidência de build

Na história AWS-LC, o commit
[`bf6daa8e66aa677f2e1bcab85232faf2461b897d`](https://github.com/aws/aws-lc/commit/bf6daa8e66aa677f2e1bcab85232faf2461b897d)
de 2025-09-04 atualizou o `META.yml` para o commit e target `v3.6.3`. Já o
commit [`3948c3684743d870c7760c1811104be185b2c43b`](https://github.com/aws/aws-lc/commit/3948c3684743d870c7760c1811104be185b2c43b)
de 2026-08-04 alterou o README para documentar uma adaptação MinGW, mas não
alterou a linha `v3.4.0`. O histórico sustenta a hipótese de versão antiga no
README; não comprova que o import inteiro seja idêntico ao upstream 3.6.3 nem
explica a intenção dos mantenedores.

A árvore AWS-LC fixada contém ainda [`NOTICE` na raiz](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/NOTICE)
e [`NOTICE` de s2n-bignum](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/third_party/s2n-bignum/s2n-bignum-imported/NOTICE),
ambos com copyright Amazon; a busca recursiva da API nessa árvore retornou esses
dois nomes e não estava truncada. O repositório aws-lc-rs no commit fixado também
contém `NOTICE` na raiz. Essa busca por nome não substitui revisão de cabeçalhos,
submódulos ou dependências adicionais.

Foi inspecionado um arquivo intermediário de build já existente, gerado durante
a investigação do pacote `storage-s3` extraído com Cargo 1.99.1, não um release:
`other-target/debug/build/aws-lc-sys-cb0c5e872a5d3080/out/libaws_lc_0_47_0_crypto.a`,
SHA-256 `9b51d35e7f0a590bd9d19193879c9cd69e102f29bb1329784f0270400c3952f3`.
O `output` adjacente registra `cargo:rustc-link-lib=static=aws_lc_0_47_0_crypto`.
`ar t` mostrou seis objetos `jitterentropy-{base,gcd,health,noise,sha3,timer}.o`
e objetos com nomes `mlkem`, `mldsa` e `keccak`. Isso demonstra presença desses
objetos no **arquivo estático inspecionado**, não que cada objeto tenha sido
vinculado ou distribuído num executável. A filtragem por nomes não determina
todos os componentes, especialmente código inlined, gerado ou com outros nomes;
não se conclui ausência de fiat ou s2n-bignum. Um build final identificado,
seus objetos e forma de ligação ainda precisam de auditoria própria.

### Amostra de cabeçalhos dos objetos do arquivo estático

Foi feita uma revisão **amostral**, não exaustiva, dos fontes locais com nomes
correspondentes aos objetos acima. As listas `builder/cc_builder/universal.rs`
e `builder/cc_builder/linux_x86_64.rs` também incluem os caminhos citados;
nomes e listas apoiam o mapeamento, mas não substituem rastreamento integral de
comandos do compilador ou análise de um binário final:

| Objeto na archive | Fonte local correspondente sob `aws-lc/` | Cabeçalho observado |
|---|---|---|
| `jitterentropy-base.o`, `jitterentropy-sha3.o` | `third_party/jitterentropy/jitterentropy-library/src/jitterentropy-{base,sha3}.c` | Copyright Stephan Mueller, respectivamente 2014–2025 e 2021–2025; remetem ao `LICENSE` da raiz e incluem disclaimer. O texto integral ausente da crate foi localizado no commit upstream fixado acima. |
| `mlkem_ntt_avx2_asm.o` | `crypto/fipsmodule/ml_kem/mlkem/native/x86_64/src/mlkem_ntt_avx2_asm.S` | Autores mlkem-native; `Apache-2.0 OR ISC OR MIT`; o comentário afirma derivação de implementação Kyber de domínio público e cita suas referências. |
| `mldsa_ntt_avx2_asm.o` | `crypto/fipsmodule/ml_dsa/mldsa/native/x86_64/src/mldsa_ntt_avx2_asm.S` | Autores mlkem-native e mldsa-native; `Apache-2.0 OR ISC OR MIT`; o comentário afirma derivação de implementação Dilithium de domínio público e cita sua fonte. |
| `sha3_keccak_f1600.o` | `third_party/s2n-bignum/s2n-bignum-imported/x86_att/sha3/sha3_keccak_f1600.S` | Copyright OpenSSL Project e Amazon; `SPDX-License-Identifier: Apache-2.0`, com crédito a Andy Polyakov. Não confundir com a licença alternativa geral de s2n-bignum. |
| `keccak.o` | `crypto/keccak/keccak.c` | Copyright Amazon; `Apache-2.0 OR ISC`. O nome `keccak` sozinho não justifica aplicar CC0 a este arquivo. |

Em particular, a afirmação do notice composto sobre certas contribuições de
referência Keccak/AES como CC0 não implica que **todo** arquivo Keccak tenha
esse enquadramento. Avaliar cada cabeçalho e as referências pertinentes; as
alegações de domínio público citadas nos fontes não foram verificadas
independentemente. Outros objetos do arquivo estático, arquivos incluídos por
headers e avisos de terceiros não foram auditados aqui. Antes da distribuição,
identificar com link map ou método equivalente quais objetos e notices são
relevantes para cada artefato real, verificar textos completos e atribuições e
submeter a revisão jurídica; não inferir obrigação final apenas desta amostra.

### Varredura ampliada dos membros da archive

Na mesma archive identificada pelo SHA-256 acima, `ar t` listou **393 membros**.
Para triagem, removeu-se de cada nome `.o` seu prefixo hexadecimal de 16
caracteres e procurou-se o mesmo nome-base entre os arquivos locais da crate
com extensão `.c`, `.cc`, `.S` ou `.s`, inclusive fontes gerados. O resultado:

- **295 membros** tiveram um único candidato por nome-base;
- **98** tiveram dois ou mais candidatos; **zero** ficaram sem candidato;
- nos 295 candidatos únicos, busca de `SPDX-License-Identifier` nos primeiros
  4.096 caracteres encontrou 173 `Apache-2.0`, 52 `ISC`, 22
  `Apache-2.0 OR ISC`, 40 `Apache-2.0 OR ISC OR MIT`, dois
  `Apache-2.0 OR ISC OR MIT-0` e seis sem identificador nesse trecho.
  Os seis são os fontes `jitterentropy-{base,gcd,health,noise,sha3,timer}.c`,
  que requerem examinar o texto de licença referenciado e os cabeçalhos, não
  presumir ausência de licença.

Os dois candidatos `Apache-2.0 OR ISC OR MIT-0` são
`third_party/s2n-bignum/s2n-bignum-imported/x86_att/p384/`
`bignum_deamont_p384_alt.S` e `bignum_tomont_p384_alt.S`. Entre os nomes
ambíguos, `cfb.o` corresponde a fontes em `crypto/fipsmodule/modes/` e
`crypto/decrepit/cfb/`, `sign.o` a fontes em `crypto/evp_extra/` e ML-DSA,
e `mem.o` a fontes em `crypto/` e `provider/backend/`. É necessário resolver
esses casos usando comandos de compilação, metadados ou correspondência do
prefixo com o builder; escolher arbitrariamente um arquivo seria enganoso.

Os números contam **membros**, não licenças ou autores distintos. Nome-base
único não prova que aquele fonte tenha sido efetivamente usado, nem que seu
SPDX cubra headers, assembly gerado, código incorporado, avisos de terceiros ou
todo o conteúdo do objeto; a busca limitada pode perder avisos. Os 98 membros
ambíguos, a totalidade de seus cabeçalhos e dependências incluídas, e a ligação
ao artefato final continuam sem auditoria completa. Essa contagem não é um
pacote de notices nem substitui a revisão por artefato e jurídica.

### Correlação dos 98 nomes ambíguos

O código local `cc-1.11.0/src/command_helpers.rs` calcula o prefixo do objeto
com `DefaultHasher` sobre origem, diretório e extensão. Um pequeno programa
descartável compilado com Rust 1.99.1 reproduziu esse cálculo para os candidatos
locais, experimentando diretórios relativos ao manifesto, à raiz de fontes e
caminhos absolutos. Os 98 membros antes ambíguos tiveram **um único candidato
por prefixo**, sem candidatos múltiplos ou sem correspondência. Exemplos:

- `9edd4d1b3e53823d-cfb.o` → `aws-lc/crypto/decrepit/cfb/cfb.c`;
- `bbaad1828736bced-sign.o` → `aws-lc/crypto/evp_extra/sign.c`;
- `c183f190e0fda004-mem.o` → `aws-lc/crypto/mem.c`.

Nos primeiros 4.096 caracteres desses 98 candidatos, a busca identificou 50
`Apache-2.0 OR ISC OR MIT-0`, 17 `Apache-2.0 OR ISC OR MIT`, três
`Apache-2.0`, um `ISC`, um `Apache-2.0 OR ISC` e **26 sem SPDX**. Estes 26
são arquivos assembly sob `aws-lc/generated-src/linux-x86_64/`; não se deve
deduzir ausência de direitos ou licença a partir da falta de SPDX. Um deles é
`crypto/test/trampoline-x86_64.S`, cujo cabeçalho informa geração a partir de
script Perl BoringSSL: sua presença na archive não demonstra execução de teste
nem inclusão no binário final. A origem e os avisos dos geradores e os outros
arquivos gerados continuam pendentes de investigação.

O prefixo é uma evidência adicional útil, mas um hash não é prova criptográfica
da identidade do fonte nem substitui os comandos originais de compilação;
nenhuma comparação do conteúdo compilado ou link map de artefato final foi
realizada. As expressões acima são cabeçalhos detectados, não determinações
jurídicas nem levantamento integral de avisos em arquivos incluídos.

### Geradores dos 26 arquivos assembly sem SPDX no output

Os 26 arquivos `generated-src/linux-x86_64/` acima foram confrontados com os
scripts Perl locais de AWS-LC: 25 têm exatamente um script `.pl` com o mesmo
nome-base; para `sha256-x86_64.S`, `crypto/fipsmodule/CMakeLists.txt` associa
explicitamente a saída a `sha/asm/sha512-x86_64.pl`. Os próprios outputs
gerados remetem a scripts Perl BoringSSL de nome semelhante. A correspondência
de nome não prova reprodução byte a byte nem ausência de modificações no
pipeline; o caminho de origem é a árvore AWS-LC fixada no commit
[`cd481c207e38f06aeb5766f4a1cdfce10a4c0af8`](https://github.com/aws/aws-lc/tree/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8).

Cabeçalhos dos geradores locais inspecionados:

| Declaração no gerador | Outputs correspondentes (nome-base) |
|---|---|
| `Apache-2.0` | `chacha-x86_64`, `aesni-sha1-x86_64`, `aesni-sha256-x86_64`, `aesni-gcm-avx512`, `aesni-gcm-x86_64`, `aesni-x86_64`, `aesni-xts-avx512`, `ghash-x86_64`, `p256-x86_64-asm`, `rsaz-2k-avx512`, `rsaz-3k-avx512`, `rsaz-4k-avx512`, `rsaz-avx2`, `sha1-x86_64`, `sha256-x86_64`, `sha512-x86_64`, `vpaes-x86_64`, `x86_64-mont`, `x86_64-mont5` |
| `ISC` | `aes128gcmsiv-x86_64`, `chacha20_poly1305_x86_64`, `ghash-ssse3-x86_64`, `rdrand-x86_64`, `trampoline-x86_64` |
| `Apache-2.0 OR ISC` | `p256_beeu-x86_64-asm` |
| Declaração própria, não SPDX | `md5-x86_64`: o gerador identifica Marc Bevand e declara renúncia ao copyright e colocação em domínio público. A aplicabilidade e os demais insumos ainda requerem revisão. |

Os cabeçalhos incluem diferentes titulares, entre eles OpenSSL, Google, Intel,
Cloudflare, Amazon e Shay Gueron; os textos originais, créditos e disclaimers
devem ser examinados individualmente. Por exemplo,
`crypto/cipher_extra/asm/aesni-sha256-x86_64.pl` também menciona sequências
obtidas de `sha512-x86_64.pl`. O gerador `crypto/test/asm/trampoline-x86_64.pl`
declara ISC e diz ser ligado a libcrypto por conveniência; sua expectativa de
eliminação por linker não é prova de que isso ocorreu no artefato concreto.

Esta é uma triagem dos cabeçalhos dos **geradores**, não licença automaticamente
atribuída a todo output, nem auditoria completa dos tradutores perlasm, scripts,
headers ou referências incorporadas. Não foi regenerado assembly, comparado
byte a byte nem inspecionado link map de release. Preservar os textos completos
pertinentes e revisar toda a cadeia e distribuição escolhida antes de concluir
qualquer atribuição ou obrigação legal.

### Tradutor perlasm e includes compartilhados

Uma busca textual nos 25 scripts candidatos (o gerador SHA-512 é candidato
para duas saídas) encontrou referência a `x86_64-xlate.pl` para cada uma das
26 saídas. A árvore local contém um único arquivo com esse nome:
[`crypto/perlasm/x86_64-xlate.pl`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/crypto/perlasm/x86_64-xlate.pl),
cujo cabeçalho declara copyright 2005–2016 OpenSSL Project Authors e
`SPDX-License-Identifier: Apache-2.0`. Por exemplo, o gerador trampoline
procura o tradutor junto ao script ou em `../../perlasm/` e o executa com Perl.
Referência textual e arquivo disponível não provam que esse tradutor foi
efetivamente executado para produzir os arquivos pré-gerados da crate.

Uma inspeção dos 26 outputs existentes encontrou `#include
<openssl/asm_base.h>` em **26/26**. O arquivo local
`aws-lc/include/openssl/asm_base.h` declara copyright Google 2023 e ISC; ele
inclui `openssl/target.h`, cujo cabeçalho também declara Google 2023 e ISC,
e, quando em assembly, `openssl/boringssl_prefix_symbols_asm.h`. Há cópias
desse último tanto em `aws-lc/include/` quanto em `generated-include/`; a
cópia gerada examinada declara copyright Google 2018 e ISC. Sem rastrear a
ordem efetiva de includes do compilador, não se afirma qual cópia foi usada.

O tradutor emite texto com include de `asm_base.h`, mas a inspeção não cobre
todos os módulos Perl, referências cruzadas, headers transitivos nem possíveis
licenças de código incorporado. Não se inferiu que o Apache-2.0 do tradutor
substitui o aviso do gerador ou que os avisos dos headers bastam para o output.
Não houve regeneração nem comparação do assembly. Persistem revisão dos textos
originais e a identificação por link map dos objetos efetivamente presentes
em artefatos finais; as contagens referem-se somente à archive intermediária.

### Investigação adicional dos headers de prefixo

Na versão local de `builder/main.rs`, `setup_include_paths` lista primeiro os
headers Rust, depois `generated-include` e então `aws-lc/include`; ao copiar
diretórios usa `skip_exist(true)` com comentário de preferência aos caminhos
anteriores. No diretório `out/include/openssl/` associado à archive examinada,
`boringssl_prefix_symbols_asm.h` começa com copyright Google 2018 e SPDX ISC,
como a cópia em `generated-include/`. `sha256sum` retornou o mesmo valor
`a91ceea2e861c4d225b39f71cb2f06e616e9de036e904ccbe3905ccb4c56dffd`
para ambas. Esta é evidência da **cópia preparada no output**, não de uma linha
de compilação capturada ou da ordem efetiva de
busca do compilador para cada objeto.

Em contraste, `aws-lc/include/openssl/boringssl_prefix_symbols_asm.h` declara
copyright Amazon e `Apache-2.0 OR ISC` e é intencionalmente vazio. A cópia em
`generated-include/` referencia ainda `boringssl_prefix_symbols.h`; o cabeçalho
desta última também declara Google 2018 e ISC e define o prefixo
`aws_lc_0_47_0` quando não fornecido. Ambas as variantes e outros headers
podem ser relevantes conforme o artefato e processo de build; não substituir
seus avisos por um único rótulo genérico.

A inspeção local de `aesni-sha256-x86_64.pl` registra ainda que sequências de
instruções SHA-256 foram tomadas de `sha512-x86_64.pl`. Esse vínculo é um
comentário no fonte, não uma auditoria de todos os trechos nem prova de
identidade de output. `asm_base.h` também inclui `target.h` e, para assembly,
o header de prefixos. Referências transitivas, o tradutor e seus geradores
precisam ser considerados no levantamento de fontes distribuídas, mas a mera
referência ou cópia não estabelece a obrigação aplicável a um binário final.
Continuam pendentes a verificação completa de includes/geração por comandos
reproduzíveis, a composição do artefato de release e a revisão jurídica.

### Includes transitivos: inspeção e sondagem controlada

Uma nova busca textual em todos os 26 arquivos assembly gerados procurou
diretivas `#include`: em cada arquivo o único include **direto textual** foi
`openssl/asm_base.h` (26/26). Isso inclui diretivas em qualquer ramo
condicional e não comprova o resultado do pré-processador ou a inclusão em
executáveis. Em `asm_base.h`, além dos headers de target e prefixo já citados,
há `cet.h` condicionado a `__CET__` e `OPENSSL_X86_64`; `target.h` pode incluir
`endian.h` ou `sys/param.h` sob condições alternativas de plataforma. Sua
seleção e origem dependem de compilador, macros, sysroot e flags concretos.

Como sondagem independente e somente leitura, executou-se `cc -M -x
assembler-with-cpp -I <out/include> <generated-src/linux-x86_64/crypto/chacha/chacha-x86_64.S>`
no host Linux x86-64, usando o `out/include` associado à archive inspecionada.
O resultado listou a fonte, `asm_base.h`, `target.h`,
`boringssl_prefix_symbols_asm.h`, `boringssl_prefix_symbols.h` nesse diretório
e `/usr/include/stdc-predef.h`. A sondagem não incluiu `cet.h` ou `endian.h`.
Isso comprova apenas a resolução **dessa invocação atual**, não as flags,
headers, sysroot ou macros usados na compilação histórica dos 393 objetos; não
se executou um rebuild nem se demonstrou o vínculo de qualquer objeto final.
Os headers de sistema pertencem ao ambiente de build; verificar sua versão,
avisos e eventual distribuição separadamente, sem presumir que sejam
incorporados ou que imponham os mesmos termos ao executável.

O gerador `aesni-sha256-x86_64.pl` também credita explicitamente Andy Polyakov
ao projeto OpenSSL e documenta que as sequências SHA-256 vieram do gerador
SHA-512. Tais comentários e SPDX dos geradores são evidência para revisão dos
textos e créditos, não autorização para atribuir automaticamente ao output uma
licença única. Outras referências, código copiado, includes condicionais,
geradores e sysroots de diferentes toolchains permanecem por avaliar. A
composição e obrigações de distribuição só podem ser apuradas depois de fixar
artefatos, target, features, source bundle e processo de build.

### Conferência de proveniência dos insumos locais

Para uma amostra adicional, a API GitHub do commit AWS-LC fixado foi comparada
com `git hash-object` nos arquivos da crate local. Os IDs Git coincidiram:

| Caminho relativo a `aws-lc/` | ID do blob Git local e upstream |
|---|---|
| `crypto/perlasm/x86_64-xlate.pl` | `b2f28a0cf841407c220b1b7fdf4b5f044be2ca5c` |
| `crypto/fipsmodule/md5/asm/md5-x86_64.pl` | `b33f12476dd9b017ee6059ccf37aaca278b1e3d3` |
| `crypto/fipsmodule/sha/asm/sha512-x86_64.pl` | `3abee2aa17a75236b50b3e9e34543cc19dc8c202` |
| `include/openssl/asm_base.h` | `a1dbd820a8dbf9e92560dbc589d4549cd9cc97ea` |
| `include/openssl/target.h` | `687760f1fc62ca530a7ecf670c01ccc290c098d2` |
| `generated-src/linux-x86_64/crypto/chacha/chacha-x86_64.S` | `b4809fa07bf33f2470a4ebaff9f6480af710fafe` |

Os quatro primeiros cabeçalhos de licença e avisos relevantes já foram
registrados nas seções anteriores; `target.h` também declara ISC e copyright
Google 2023. O arquivo ChaCha gerado corresponde ao blob do commit AWS-LC,
portanto não é apenas um artefato encontrado no target de build local. Essa
amostra inicial não verificava os outros 25 outputs ou todos os geradores; a
comparação posterior abaixo amplia o escopo. Ela não demonstra qual versão
anterior do BoringSSL/OpenSSL foi usada nem prova geração reprodutível. IDs de
blob Git são evidência de identidade dos arquivos
consultados, não prova de titularidade ou atribuição suficiente.

A sondagem de pré-processador anterior identifica headers para uma invocação
local; os headers de sistema, dependências adicionais, opções do compilador e
conjunto de objetos incluídos dependem do ambiente e do artefato final. Não há
artefato de distribuição escolhido ou link map de release nesta revisão: os
avisos completos necessários, escolhas de licença e tratamento de cópias de
fontes/geradores permanecem sujeitos a revisão por artefato e jurídica.

### Comparação dos 26 outputs e 25 geradores com o commit fixado

Foi consultada a árvore recursiva GitHub de AWS-LC em
`cd481c207e38f06aeb5766f4a1cdfce10a4c0af8`, com resposta `truncated: false`.
Para os 26 arquivos `generated-src/linux-x86_64/` previamente identificados
na archive, o ID Git local foi calculado como SHA-1 de `blob <tamanho>\0`
seguido dos bytes e comparado com o ID do mesmo caminho na árvore upstream.
O procedimento também comparou os 25 scripts candidatos: correspondência de
nome-base, exceto SHA-256 cujo gerador SHA-512 é identificado pela regra CMake.

| Escopo | Comparados | IDs coincidentes | Ausentes upstream | Divergentes |
|---|---:|---:|---:|---:|
| Assembly gerado | 26 | 26 | 0 | 0 |
| Geradores Perl distintos | 25 | 25 | 0 | 0 |

A comparação confirma identidade dos arquivos locais com os caminhos daquele
commit, inclusive os 25 outputs além da amostra ChaCha. Não prova que executar
esses scripts e o tradutor reproduziria byte a byte os outputs: versões de
Perl, argumentos, ambiente, outros insumos e alterações posteriores à geração
não foram reconstruídos. Tampouco identifica a revisão anterior de
BoringSSL/OpenSSL, resolve direitos ou demonstra que os objetos foram ligados
a um executável. Os notices finais ainda dependem da escolha e inspeção dos
artefatos, fontes, toolchains e processo de distribuição, além da revisão
jurídica; não foram criados nem publicados artefatos nesta verificação.

### Imports ML-KEM e ML-DSA e seus notices

Os metadados na árvore AWS-LC fixada `cd481c207e38f06aeb5766f4a1cdfce10a4c0af8`
identificam imports `v2.0.0`. Estes `META.yml` foram consultados no commit
upstream AWS-LC; não estavam presentes nos diretórios correspondentes da crate
local inspecionada.

| Import | Commit de origem no `META.yml` | `LICENSE` de origem e SHA-256 |
|---|---|---|
| [`mlkem-native`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/crypto/fipsmodule/ml_kem/META.yml) | `d1b2fe782888bdb761a50336012923180be7f502` | [`LICENSE`](https://github.com/pq-code-package/mlkem-native/blob/d1b2fe782888bdb761a50336012923180be7f502/LICENSE): `1c730e3c2cd4f70e058519ef3e910d8bdd4ed822ae2ce689f3c63d32fc52314b` |
| [`mldsa-native`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/crypto/fipsmodule/ml_dsa/META.yml) | `834a90d5e846ffa1e1611bd24e160bb2e9b86d35` | [`LICENSE`](https://github.com/pq-code-package/mldsa-native/blob/834a90d5e846ffa1e1611bd24e160bb2e9b86d35/LICENSE): `22f7173776d0a36d3f13ec1f45d51e0502e67d17e384bddf39690d833974ff93` |

Os textos upstream descrevem alternativas Apache-2.0, ISC ou MIT para os
respectivos arquivos da biblioteca, apontam copyright nos cabeçalhos e alertam
que testes, exemplos e outros arquivos podem ter termos distintos. As árvores
upstream consultadas não estavam truncadas e incluem licenças adicionais de
`tiny_sha3` em exemplos. Não se conclui que esses exemplos ou sua licença
tenham sido importados para esta crate ou compilados. O notice composto
`aws-lc-sys/LICENSE` também identifica ambos os projetos; ele não substitui a
revisão dos arquivos e notices pertinentes ao artefato concreto.

Há uma divergência relevante na documentação AWS-LC: os READMEs fixados de
ML-KEM e ML-DSA dizem que o importador copia apenas a versão C, enquanto os
respectivos `importer.sh` no mesmo commit incluem cópias explícitas dos
backends x86_64/aarch64. A archive intermediária examinada contém objetos com
nomes `mlkem_*` e `mldsa_*` AVX2. A hipótese de README desatualizado é
plausível, mas não foi confirmada pelos mantenedores; não usar a frase "apenas
C" como prova do conteúdo. Os scripts também descrevem alterações de includes e
adaptações após copiar arquivos: não se presume identidade byte a byte de todo
o import. Os cabeçalhos amostrados no assembly local atribuem alternativas
`Apache-2.0 OR ISC OR MIT` e referem implementações Kyber/Dilithium como domínio
público; essas afirmações não foram verificadas independentemente.

As licenças e arquivos extras nos repositórios de origem não demonstram sua
inclusão no binário nem no source bundle final. É necessário rastrear cada
fonte/header realmente distribuído, seus avisos e as opções escolhidas; usar um
link map para um executável concreto, se houver, e revisão jurídica antes da
distribuição. Não se modificou o contrato de admissão ou se publicou artefato.

### Comparação dos imports com seus commits de origem

Em 2026-11-05, para cada arquivo presente nas subárvores locais
`aws-lc/crypto/fipsmodule/ml_kem/mlkem/` e `ml_dsa/mldsa/`, o ID Git do blob
local foi comparado ao caminho correspondente sob `mlkem/src/` ou
`mldsa/src/` nas árvores recursivas upstream fixadas acima. Os headers
`mlkem_native.h` e `mldsa_native.h` foram comparados com seus caminhos na raiz
de cada biblioteca; os dois arquivos `*_native_bcm.c`, agregados pelo
importador, foram classificados sem correspondência direta. As respostas de
ambas as árvores upstream indicaram `truncated: false`.

| Import local | Arquivos | IDs iguais | IDs diferentes | Sem caminho direto |
|---|---:|---:|---:|---:|
| ML-KEM | 68 | 37 | 30 | 1 |
| ML-DSA | 75 | 39 | 35 | 1 |

Uma busca nos primeiros 4.096 bytes de **cada arquivo local dessas duas
subárvores**, não de toda a dependência AWS-LC, encontrou a expressão
`Apache-2.0 OR ISC OR MIT` nos 68 e nos 75 arquivos, respectivamente. Isso
registra cabeçalhos encontrados, não comprova que todos os avisos de terceiros,
arquivos incluídos ou termos aplicáveis foram identificados. Esses arquivos
estão presentes na crate local; a comparação não demonstra que todos foram
compilados ou vinculados ao artefato final.

Os diffs completos de duas amostras x86-64, `mlkem_ntt_avx2_asm.S` e
`mldsa_ntt_avx2_asm.S`, mostram a substituição do include `common.h` e de
macros de namespace pelos macros `_internal_s2n_bignum_x86_att.h` e
`S2N_BN_*`, além da remoção das condições originais de backend nessas amostras.
Os cabeçalhos de copyright/SPDX permanecem nas amostras. Os importadores AWS-LC
fixados descrevem alterações de includes e adaptação de assembly, mas não se
conclui que todas as 65 diferenças sejam iguais ou puramente mecânicas. A
comparação posterior dos dois arquivos BCM é descrita abaixo; os seus insumos
não foram todos auditados linha a linha. Não houve regeneração, teste funcional ou auditoria jurídica; revisar
avisos e mudanças de cada arquivo relevante para o source bundle e link map do
artefato efetivamente escolhido.

### Comparação dos agregados BCM

Os dois arquivos locais `*_native_bcm.c` foram comparados com os arquivos
`mlkem/mlkem_native.c` e `mldsa/mldsa_native.c` dos commits upstream fixados,
respectivamente. O diff textual registra 225 linhas removidas e 17 adicionadas
para ML-KEM (692 linhas upstream, 484 locais); 157 removidas e 18 adicionadas
para ML-DSA (803 upstream, 664 locais). Contagens de diff dependem do algoritmo
e não medem quantidade de mudanças funcionais.

Em ambos os casos, o diff mostra caminhos de `#include` adaptados e remoção de
blocos upstream de FIPS-202 e de seus backends nativos; no ML-KEM também aparecem
remoções de includes condicionais RISC-V e PPC64LE. Isso é consistente com os
importadores e READMEs AWS-LC que descrevem integração com sua própria FIPS-202,
mas não prova equivalência funcional nem determina se cada bloco seria ativado
na configuração concreta. Os cabeçalhos dos agregados locais mantêm copyright
dos projetos correspondentes e `Apache-2.0 OR ISC OR MIT`.

Uma checagem textual encontrou 14 includes entre aspas no BCM ML-KEM e 15 no
BCM ML-DSA; todos os caminhos correspondentes existem ao lado dos arquivos
locais, incluindo os condicionais de arquiteturas. Isso não executa o
pré-processador, não cobre includes transitivos nem verifica que todos sejam
compilados. `ml_kem.c` e `ml_dsa.c` incluem seus agregados três vezes com
configurações de parâmetros diferentes. A revisão das 65 diferenças em outros
arquivos permanece parcial: não foi demonstrado que são todas mecânicas. A
escolha de avisos depende dos fontes/objetos efetivamente distribuídos e de
revisão jurídica; não foram regenerados arquivos ou construídos releases.

### Triagem por arquivo das 65 diferenças

Para os 65 arquivos alterados, foram obtidos os textos dos caminhos upstream
nos commits fixados e classificados os trechos de cada diff com
`difflib.SequenceMatcher(autojunk=False)`. A comparação encontrou mudanças em
**include, símbolos e condições** em cada um dos 65. Uma classificação textual
adicional marcou trechos `other` em 56 arquivos; esse grupo inclui continuações
de condições e macros e **não** significa necessariamente alteração de
instruções, nem prova que as mudanças sejam apenas mecânicas. Os nove restantes
não tiveram trecho `other` segundo esse filtro limitado. Não houve análise
semântica ou auditoria jurídica individual completa.

Os 65 caminhos são os itens abaixo. Cada item tem prefixo e sufixo indicados na
linha; as reticências representam a parte variável do nome. A marca `†` indica
os nove sem trecho `other` pelo filtro; todos os demais foram marcados com
`other` e necessitam revisão manual se relevantes para a distribuição.

| Diretório e padrão | Nomes completos após expandir o padrão |
|---|---|
| ML-KEM `native/aarch64/src/mlkem_…_aarch64_asm.S` | `intt`, `ntt`†, `poly_mulcache_compute`†, `poly_reduce`†, `poly_tobytes`, `poly_tomont`, `polyvec_basemul_acc_montgomery_cached_k2`, `polyvec_basemul_acc_montgomery_cached_k3`, `polyvec_basemul_acc_montgomery_cached_k4`, `rej_uniform` |
| ML-KEM `native/x86_64/src/mlkem_…_avx2_asm.S` | `intt`, `ntt`, `nttfrombytes`, `ntttobytes`, `nttunpack`, `poly_compress_d10`, `poly_compress_d11`, `poly_compress_d4`, `poly_compress_d5`, `poly_decompress_d10`, `poly_decompress_d11`, `poly_decompress_d4`, `poly_decompress_d5`, `poly_mulcache_compute`, `polyvec_basemul_acc_montgomery_cached_k2`, `polyvec_basemul_acc_montgomery_cached_k3`, `polyvec_basemul_acc_montgomery_cached_k4`, `reduce`, `rej_uniform`, `tomont` |
| ML-DSA `native/aarch64/src/mldsa_…_aarch64_asm.S` | `intt`†, `ntt`†, `pointwise_montgomery`, `poly_caddq`†, `poly_chknorm`†, `poly_decompose_32`, `poly_decompose_88`, `poly_use_hint_32`, `poly_use_hint_88`, `polyvecl_pointwise_acc_montgomery_l4`, `polyvecl_pointwise_acc_montgomery_l5`, `polyvecl_pointwise_acc_montgomery_l7`, `polyz_unpack_17`, `polyz_unpack_19`, `rej_uniform`, `rej_uniform_eta2`, `rej_uniform_eta4` |
| ML-DSA `native/x86_64/src/mldsa_…_avx2_asm.S` | `intt`, `ntt`, `nttunpack`, `pointwise_acc_l4`, `pointwise_acc_l5`, `pointwise_acc_l7`, `pointwise`, `poly_caddq`, `poly_chknorm`, `poly_decompose_32`, `poly_decompose_88`, `poly_use_hint_32`, `poly_use_hint_88`, `polyz_unpack_17`, `polyz_unpack_19`, `rej_uniform`, `rej_uniform_eta2`†, `rej_uniform_eta4`† |

Os marcados `†` são três ML-KEM AArch64, quatro ML-DSA AArch64 e dois
ML-DSA x86-64. A marcação automática permanece somente um filtro de revisão,
não uma classificação de equivalência de código.

Na comparação das primeiras oito linhas, quatro ML-KEM x86-64 diferiram porque
os includes/condições começam imediatamente após o bloco de licença:
`poly_mulcache_compute` e `polyvec_basemul_acc_montgomery_cached_k2`, `_k3`,
`_k4`. Os diffs desses quatro foram inspecionados: o bloco inicial de
copyright/SPDX permanece, enquanto include, condições e macros de símbolos são
alterados. Os outros 61 tinham as primeiras oito linhas idênticas. Isso não
cobre avisos posteriores ou conteúdo incorporado. Todos os 65 cabeçalhos
locais anteriormente procurados contêm `Apache-2.0 OR ISC OR MIT`, sem
determinar uma opção para o artefato final.

Não foi feita revisão humana linha a linha dos 65 diffs; não se declara a
auditoria exaustiva concluída nem equivalência funcional ou legal. Confirmação
dos avisos e das modificações relevantes requer identificar os arquivos
distribuídos, revisar manualmente diffs e headers completos e, quando aplicável,
inspecionar o link map; nenhuma licença final ou publicação foi autorizada.

### Triagem adicional dos 38 candidatos x86-64 na archive

Foi cruzada a lista de membros `ar t` da archive intermediária identificada
acima com os nomes-base dos arquivos locais `native/x86_64/src/*.S`. Os
**38** candidatos correspondentes (20 ML-KEM e 18 ML-DSA) tiveram seus textos
comparados aos caminhos dos commits upstream fixados, com `SequenceMatcher`
e `autojunk=False`. O método de nome-base é uma correlação com a archive, não
uma prova de que o objeto tenha sido vinculado a um executável final.

Para cada diff, foram examinadas as linhas inseridas e removidas e filtradas
as que começam com diretivas de pré-processador, comentários, `.global`,
macros `S2N_BN_`, `MLK_ASM_`, `MLD_ASM_` ou contêm nomes de configuração
especificados no filtro. Depois desse filtro **nenhum dos 38** apresentou
linha residual. O resultado explica por que parte das marcações `other` da
triagem anterior eram continuações de condições ou macros. Entretanto, o
filtro é deliberadamente textual e amplo: pode esconder alterações relevantes
em linhas filtradas, não avalia expansão de macros, comportamento condicional,
ABI, instruções geradas ou equivalência criptográfica. Não é uma prova de
instruções idênticas nem uma revisão semântica completa.

Os quatro cabeçalhos iniciais ML-KEM examinados individualmente na seção
anterior mantiveram copyright/SPDX; os demais 34 tinham as primeiras oito
linhas iguais aos respectivos arquivos upstream no levantamento anterior.
Outros comentários e avisos ao longo dos fontes e dos headers incluídos não
foram auditados integralmente. Os 27 arquivos AArch64 não entram nessa
triagem x86-64; sua presença em outras compilações e os requisitos de notices
dependem do target e do source bundle escolhido. A identificação final de
objetos vinculados exige build e link map de um artefato real, seguido de
revisão dos textos e direitos; nenhuma publicação foi realizada.

### Triagem dos 27 diffs AArch64

Os 27 arquivos locais `native/aarch64/src/*.S` alterados (10 ML-KEM e 17
ML-DSA) foram comparados aos caminhos nos commits upstream fixados usando
`difflib.SequenceMatcher(autojunk=False)`. Todas as 27 comparações confirmaram
diferenças. As linhas contendo `Copyright` e `SPDX-License-Identifier` nas
primeiras 15 linhas coincidiram com o upstream em **27/27**; isso não cobre
outros avisos ou fontes incorporados.

Aplicando às linhas alteradas o mesmo filtro textual amplo usado para x86-64
(diretivas, comentários, `.global`, macros de símbolos e referências de
configuração), **nenhum dos 27** apresentou linha residual. Assim como no
caso x86-64, isso é apenas uma classificação do texto: linhas filtradas podem
conter mudanças relevantes, e expansão de macros, ABI, condições de compilação
e comportamento criptográfico não foram verificados. Não se afirma que as
alterações sejam equivalentes ou apenas mecânicas.

Os nomes por arquivo estão na tabela da triagem dos 65 diffs acima. Esta
verificação leu fontes e dados upstream, mas **não compilou nem executou código
AArch64**, não comprovou a origem completa de todos os avisos e não determina
quais arquivos comporiam uma futura distribuição. A archive intermediária
analisada é x86-64; não extrapolar o resultado a um binário AArch64 nem ampliar
as garantias do projeto além do Linux x86-64 e combinações efetivamente
testadas. A revisão por artefato e jurídica permanece pendente.

### Diffs completos de quatro amostras

Foram lidos integralmente, sem filtro de linhas, os diffs entre os commits
upstream fixados e os quatro arquivos locais abaixo. Os diffs são textuais;
nenhum foi compilado, executado ou verificado semanticamente nesta etapa.

| Caminho relativo ao respectivo import | Mudanças observadas no diff | Aviso inicial local |
|---|---|---|
| `mlkem/native/x86_64/src/mlkem_poly_compress_d10_avx2_asm.S` | Remove `common.h` e condição de backend, API e parâmetros K; inclui `_internal_s2n_bignum_x86_att.h` e substitui macros de namespace/tamanho por `S2N_BN_*`. | Autores mlkem-native; `Apache-2.0 OR ISC OR MIT`. |
| `mldsa/native/x86_64/src/mldsa_polyz_unpack_19_avx2_asm.S` | Remove condição de backend, APIs sign/verify e parâmetros 65/87; troca include e macros de símbolo/tamanho por variantes `S2N_BN_*`. | Autores mldsa-native; `Apache-2.0 OR ISC OR MIT`. |
| `mlkem/native/aarch64/src/mlkem_intt_aarch64_asm.S` | Remove condição de backend e APIs encaps/decaps; troca include por `_internal_s2n_bignum_arm.h` e macros de símbolo/tamanho por `S2N_BN_*`. | Arm Limited, Hanno Becker, Amin Abdulrahman, Matthias Kannwischer e autores mlkem-native; `Apache-2.0 OR ISC OR MIT`. |
| `mldsa/native/aarch64/src/mldsa_polyz_unpack_19_aarch64_asm.S` | Remove condição de backend, APIs sign/verify e parâmetros 65/87; troca include e macros de símbolo/tamanho por variantes `S2N_BN_*`. | Autores mldsa-native e mlkem-native; `Apache-2.0 OR ISC OR MIT`. |

Nos quatro diffs completos não apareceram linhas de instrução do corpo
alteradas; as condições removidas e os macros substituídos, contudo, podem
alterar seleção, visibilidade, símbolos ou expansão e **não** são considerados
funcionalmente equivalentes apenas por esse fato. Os avisos da tabela são
cabeçalhos observados, não escolha de licença ou conclusão jurídica. As
referências, headers transitivos e os demais arquivos ainda requerem revisão
conforme o source bundle ou artefato distribuído; somente os dois caminhos
x86-64 foram correlacionados por nome com a archive intermediária. Os caminhos
AArch64 não ampliam o escopo de plataformas homologadas.

### Sete diffs completos adicionais de compressão ML-KEM x86-64

Foram lidos integralmente os diffs dos sete arquivos abaixo contra o commit
`d1b2fe782888bdb761a50336012923180be7f502`. Todos estão sob
`ml_kem/mlkem/native/x86_64/src/`, foram correlacionados por nome com membros
da archive intermediária e mantêm no cabeçalho copyright dos autores
mlkem-native e `Apache-2.0 OR ISC OR MIT`. A tabela descreve as **condições
upstream removidas do próprio arquivo**; não implica que o build AWS-LC careça
de outras condições em seus chamadores ou configuração.

| Arquivo | Condições upstream adicionais removidas, além de backend e shared |
|---|---|
| `mlkem_poly_compress_d11_avx2_asm.S` | API encaps/decaps e `K == 4` ou multilevel shared. |
| `mlkem_poly_compress_d4_avx2_asm.S` | API encaps/decaps e `K == 2 || K == 3` ou multilevel shared. |
| `mlkem_poly_compress_d5_avx2_asm.S` | API encaps/decaps e `K == 4` ou multilevel shared. |
| `mlkem_poly_decompress_d10_avx2_asm.S` | API decaps e `K == 2 || K == 3` ou multilevel shared. |
| `mlkem_poly_decompress_d11_avx2_asm.S` | API decaps e `K == 4` ou multilevel shared. |
| `mlkem_poly_decompress_d4_avx2_asm.S` | API decaps e `K == 2 || K == 3` ou multilevel shared. |
| `mlkem_poly_decompress_d5_avx2_asm.S` | API decaps e `K == 4` ou multilevel shared. |

Nos sete diffs, `common.h` foi substituído por
`_internal_s2n_bignum_x86_att.h`, e os macros de namespace/tamanho por
`S2N_BN_*`; não apareceram alterações textuais nas instruções dos corpos. A
remoção das condições e a troca de macros podem alterar o que é compilado,
exportado ou selecionado. Sem pré-processamento comparativo, inspeção dos
chamadores, testes apropriados e link map não se conclui equivalência, presença
em produto final ou obrigações finais de licença. Outros diffs continuam
pendentes de leitura integral; não foram construídos ou publicados artefatos.

### Quatro diffs completos ML-DSA x86-64: decomposição e hint

Foram lidos integralmente os diffs dos quatro arquivos a seguir contra o commit
`834a90d5e846ffa1e1611bd24e160bb2e9b86d35`. Todos estão sob
`ml_dsa/mldsa/native/x86_64/src/`, foram correlacionados por nome com membros
da archive intermediária e mantêm no cabeçalho copyright dos autores
mldsa-native e `Apache-2.0 OR ISC OR MIT`. As condições listadas são removidas
do próprio arquivo local; não provam ausência de condições nos chamadores ou
nas regras de build.

| Arquivo | Condições upstream adicionais removidas, além de backend e shared |
|---|---|
| `mldsa_poly_decompose_32_avx2_asm.S` | API sign e parâmetros 65 ou 87, ou multilevel shared. |
| `mldsa_poly_decompose_88_avx2_asm.S` | API sign e parâmetro 44, ou multilevel shared. |
| `mldsa_poly_use_hint_32_avx2_asm.S` | API verify e parâmetros 65 ou 87, ou multilevel shared. |
| `mldsa_poly_use_hint_88_avx2_asm.S` | API verify e parâmetro 44, ou multilevel shared. |

Nos quatro diffs, `common.h` foi trocado por `_internal_s2n_bignum_x86_att.h`
e os macros de namespace/tamanho por `S2N_BN_*`; não apareceram alterações
textuais nas instruções dos corpos. Essa observação não prova equivalência de
expansão, seleção, ABI ou comportamento criptográfico. Os headers, referências
e demais arquivos ainda exigem revisão de avisos por artefato e, quando
cabível, link map e testes; não houve compilação ou publicação nesta etapa.

### Quatro diffs completos ML-DSA x86-64: pointwise

Foram lidos integralmente os quatro diffs contra o commit
`834a90d5e846ffa1e1611bd24e160bb2e9b86d35`. Os caminhos são relativos a
`ml_dsa/mldsa/native/x86_64/src/`, correlacionados por nome com membros da
archive intermediária. Todos os cabeçalhos locais identificam os autores
mldsa-native e declaram `Apache-2.0 OR ISC OR MIT`.

| Arquivo | Condição removida do próprio arquivo upstream |
|---|---|
| `mldsa_pointwise_acc_l4_avx2_asm.S` | Backend x86-64, shared e `MLDSA_L == 4` ou multilevel shared. |
| `mldsa_pointwise_acc_l5_avx2_asm.S` | Backend x86-64, shared e `MLDSA_L == 5` ou multilevel shared. |
| `mldsa_pointwise_acc_l7_avx2_asm.S` | Backend x86-64, shared e `MLDSA_L == 7` ou multilevel shared. |
| `mldsa_pointwise_avx2_asm.S` | Backend x86-64 e shared. |

Em todos, `common.h` é substituído por `_internal_s2n_bignum_x86_att.h` e os
macros de namespace/tamanho por `S2N_BN_*`. Não apareceram mudanças textuais
nas instruções dos corpos. As condições podem existir em outro ponto e a troca
de macros pode alterar compilação, símbolos, ABI ou seleção; não foi
demonstrada equivalência funcional ou criptográfica. Avisos por fonte e
artefato, demais diffs, expansão de macros e vínculo final permanecem pendentes
de revisão; nenhuma compilação, publicação ou alteração de contrato ocorreu.

### Oito diffs completos ML-DSA x86-64: transformadas e rejeição

Os oito diffs abaixo foram lidos integralmente contra o commit
`834a90d5e846ffa1e1611bd24e160bb2e9b86d35`, sob
`ml_dsa/mldsa/native/x86_64/src/`. Os oito foram anteriormente correlacionados
por nome com membros da archive intermediária. Todos os cabeçalhos locais
declaram `Apache-2.0 OR ISC OR MIT`; `intt` e `nttunpack` creditam autores de
mlkem-native **e** mldsa-native, enquanto os outros seis creditam mldsa-native.

| Arquivo | Condições removidas do arquivo upstream |
|---|---|
| `mldsa_intt_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mldsa_nttunpack_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mldsa_poly_caddq_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mldsa_poly_chknorm_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mldsa_polyz_unpack_17_avx2_asm.S` | Backend/shared, APIs sign ou verify, parâmetro 44 ou multilevel shared. |
| `mldsa_rej_uniform_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mldsa_rej_uniform_eta2_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mldsa_rej_uniform_eta4_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |

Em cada diff, `common.h` e os macros `MLD_ASM_*` foram substituídos por
`_internal_s2n_bignum_x86_att.h` e `S2N_BN_*`; não apareceram alterações
textuais nas instruções do corpo. Não se inferiu que condições equivalentes
existam ou não em outro ponto, nem equivalência de compilação, ABI ou
criptografia. Avisos de headers e outros fontes, vínculo final e revisão legal
continuam pendentes; não foram executados testes ou publicados artefatos.

### Sete diffs completos ML-KEM x86-64: transformadas e redução

Os sete diffs abaixo foram lidos integralmente contra o commit
`d1b2fe782888bdb761a50336012923180be7f502`; os caminhos são relativos a
`ml_kem/mlkem/native/x86_64/src/`. Seus nomes foram correlacionados com
membros da archive intermediária. Todos os cabeçalhos locais identificam os
autores mlkem-native e declaram `Apache-2.0 OR ISC OR MIT`.

| Arquivo | Condições upstream removidas do próprio arquivo |
|---|---|
| `mlkem_intt_avx2_asm.S` | Backend x86-64, ausência de `MULTILEVEL_NO_SHARED` e pelo menos uma das APIs encaps/decaps habilitada. |
| `mlkem_nttfrombytes_avx2_asm.S` | Backend x86-64, ausência de `MULTILEVEL_NO_SHARED` e pelo menos uma das APIs encaps/decaps habilitada. |
| `mlkem_ntttobytes_avx2_asm.S` | Backend x86-64, ausência de `MULTILEVEL_NO_SHARED` e pelo menos uma das APIs keypair/encaps habilitada. |
| `mlkem_nttunpack_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mlkem_reduce_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mlkem_rej_uniform_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mlkem_tomont_avx2_asm.S` | Backend x86-64, ausência de `MULTILEVEL_NO_SHARED` e API keypair habilitada. |

Em cada arquivo, `common.h` e macros `MLK_ASM_*` foram substituídos por
`_internal_s2n_bignum_x86_att.h` e `S2N_BN_*`; os diffs não mostraram linhas
de instrução do corpo alteradas. Essas condições podem ser tratadas em outros
pontos e as macros podem mudar seleção, ABI ou símbolos; não foi demonstrada
equivalência funcional ou criptográfica. Os outros diffs, avisos por artefato
e vínculo final permanecem em revisão; não houve testes ou publicação nesta
etapa e o contrato de admissão não foi alterado.

### Quatro diffs completos ML-KEM x86-64: multiplicação e acumulação

Foram lidos integralmente os diffs dos quatro arquivos abaixo contra o commit
`d1b2fe782888bdb761a50336012923180be7f502`, sob
`ml_kem/mlkem/native/x86_64/src/`. A inspeção anterior das primeiras linhas
destes arquivos verificou o bloco inicial, mas não substituía a leitura integral
dos diffs. Os nomes foram correlacionados com membros da archive intermediária;
todos mantêm o aviso inicial dos autores mlkem-native e
`Apache-2.0 OR ISC OR MIT`.

| Arquivo | Condições removidas do arquivo upstream |
|---|---|
| `mlkem_poly_mulcache_compute_avx2_asm.S` | Backend x86-64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mlkem_polyvec_basemul_acc_montgomery_cached_k2_avx2_asm.S` | Backend/shared e `MLKEM_K == 2` ou multilevel shared. |
| `mlkem_polyvec_basemul_acc_montgomery_cached_k3_avx2_asm.S` | Backend/shared e `MLKEM_K == 3` ou multilevel shared. |
| `mlkem_polyvec_basemul_acc_montgomery_cached_k4_avx2_asm.S` | Backend/shared e `MLKEM_K == 4` ou multilevel shared. |

Nos quatro, `common.h` e macros `MLK_ASM_*` foram substituídos por
`_internal_s2n_bignum_x86_att.h` e `S2N_BN_*`; não apareceram linhas de
instrução do corpo alteradas nos diffs textuais. Isso não comprova condições
equivalentes nos chamadores, expansão de macros, ABI ou equivalência
criptográfica, nem vínculo de objetos em um executável final. Os avisos completos
e a composição do artefato escolhido ainda precisam de revisão; nenhuma
compilação, publicação ou alteração do contrato de admissão ocorreu aqui.

### Nove diffs completos ML-KEM AArch64

Os nove diffs restantes sob `ml_kem/mlkem/native/aarch64/src/` foram lidos
integralmente contra `d1b2fe782888bdb761a50336012923180be7f502`; o décimo,
`mlkem_intt_aarch64_asm.S`, foi examinado nas quatro amostras anteriores. Todos
declaram `Apache-2.0 OR ISC OR MIT`; `mlkem_ntt_aarch64_asm.S` também credita
Arm Limited, Hanno Becker, Amin Abdulrahman e Matthias Kannwischer além dos
autores mlkem-native. Os demais oito creditam no cabeçalho inicial os autores
mlkem-native.

| Arquivo | Condições removidas do próprio arquivo upstream |
|---|---|
| `mlkem_ntt_aarch64_asm.S` | Backend AArch64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mlkem_poly_mulcache_compute_aarch64_asm.S` | Backend AArch64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mlkem_poly_reduce_aarch64_asm.S` | Backend AArch64 e ausência de `MULTILEVEL_NO_SHARED`. |
| `mlkem_poly_tobytes_aarch64_asm.S` | Backend/shared e pelo menos uma das APIs keypair/encaps habilitada. |
| `mlkem_poly_tomont_aarch64_asm.S` | Backend/shared e API keypair habilitada. |
| `mlkem_polyvec_basemul_acc_montgomery_cached_k2_aarch64_asm.S` | Backend/shared e `MLKEM_K == 2` ou multilevel shared. |
| `mlkem_polyvec_basemul_acc_montgomery_cached_k3_aarch64_asm.S` | Backend/shared e `MLKEM_K == 3` ou multilevel shared. |
| `mlkem_polyvec_basemul_acc_montgomery_cached_k4_aarch64_asm.S` | Backend/shared e `MLKEM_K == 4` ou multilevel shared. |
| `mlkem_rej_uniform_aarch64_asm.S` | Backend AArch64 e ausência de `MULTILEVEL_NO_SHARED`. |

Os nove trocam `common.h` e macros `MLK_ASM_*` pelo header
`_internal_s2n_bignum_arm.h` e macros `S2N_BN_*`; os diffs textuais não
mostraram linhas de instrução dos corpos alteradas. Isso não demonstra
equivalência das condições, expansão de macros, ABI ou criptografia. Não foram
compilados/executados nem identificados em um artefato final; essa leitura não
homologa AArch64 e não amplia o escopo suportado de Linux x86-64. Avisos
completos e demais fontes permanecem sujeitos à revisão por artefato.

### Dezesseis diffs completos ML-DSA AArch64 restantes

Foram lidos integralmente os diffs dos 16 arquivos abaixo, sob
`ml_dsa/mldsa/native/aarch64/src/`, contra o commit
`834a90d5e846ffa1e1611bd24e160bb2e9b86d35`. O décimo sétimo arquivo,
`mldsa_polyz_unpack_19_aarch64_asm.S`, foi examinado nas quatro amostras
anteriores. Todos os 16 cabeçalhos declaram `Apache-2.0 OR ISC OR MIT` e
creditam os autores mldsa-native. `intt` e `ntt` também creditam Arm Limited,
Hanno Becker, Amin Abdulrahman, Matthias Kannwischer e autores mlkem-native;
`polyz_unpack_17` e os três `rej_uniform` também creditam mlkem-native.

A coluna resume condições removidas **do próprio arquivo upstream**, além do
backend AArch64 e da ausência de `MULTILEVEL_NO_SHARED`, comuns a todos os 16.
Sua remoção não prova que controles equivalentes estejam ausentes no build.

| Arquivo | Condições adicionais removidas |
|---|---|
| `mldsa_intt_aarch64_asm.S` | Nenhuma adicional. |
| `mldsa_ntt_aarch64_asm.S` | Nenhuma adicional. |
| `mldsa_pointwise_montgomery_aarch64_asm.S` | API sign ou verify habilitada, ou `REDUCE_RAM`, ou `UNIT_TEST`. |
| `mldsa_poly_caddq_aarch64_asm.S` | Nenhuma adicional. |
| `mldsa_poly_chknorm_aarch64_asm.S` | Nenhuma adicional. |
| `mldsa_poly_decompose_32_aarch64_asm.S` | API sign e parâmetro 65/87 ou multilevel shared. |
| `mldsa_poly_decompose_88_aarch64_asm.S` | API sign e parâmetro 44 ou multilevel shared. |
| `mldsa_poly_use_hint_32_aarch64_asm.S` | API verify e parâmetro 65/87 ou multilevel shared. |
| `mldsa_poly_use_hint_88_aarch64_asm.S` | API verify e parâmetro 44 ou multilevel shared. |
| `mldsa_polyvecl_pointwise_acc_montgomery_l4_aarch64_asm.S` | `MLDSA_L == 4` ou multilevel shared. |
| `mldsa_polyvecl_pointwise_acc_montgomery_l5_aarch64_asm.S` | `MLDSA_L == 5` ou multilevel shared. |
| `mldsa_polyvecl_pointwise_acc_montgomery_l7_aarch64_asm.S` | `MLDSA_L == 7` ou multilevel shared. |
| `mldsa_polyz_unpack_17_aarch64_asm.S` | API sign ou verify habilitada e parâmetro 44 ou multilevel shared. |
| `mldsa_rej_uniform_aarch64_asm.S` | Nenhuma adicional. |
| `mldsa_rej_uniform_eta2_aarch64_asm.S` | API keypair e `MLDSA_ETA == 2` ou multilevel shared. |
| `mldsa_rej_uniform_eta4_aarch64_asm.S` | API keypair e `MLDSA_ETA == 4` ou multilevel shared. |

Nos 16 diffs, `common.h` e macros `MLD_ASM_*` foram substituídos pelo header
`_internal_s2n_bignum_arm.h` e macros `S2N_BN_*`; nenhuma linha de instrução
do corpo apareceu alterada. Isso não demonstra equivalência das condições,
macros, ABI ou criptografia e não elimina a necessidade de revisar avisos de
headers e outros insumos. Nenhum código AArch64 foi compilado ou executado;
esta inspeção não homologa a plataforma nem comprova inclusão em artefato
final. A escolha de notices e revisão legal seguem dependentes da distribuição.

### Headers internos s2n-bignum usados pelos imports

Os imports ML-KEM/ML-DSA locais referenciam os headers
`third_party/s2n-bignum/s2n-bignum-imported/include/`
`_internal_s2n_bignum_x86_att.h` e `_internal_s2n_bignum_arm.h`. Ambos contêm
macros de nome, visibilidade e tamanho de símbolo; o x86 inclui `cet.h` sob
condições de compilação, e o AArch64 contém definições condicionais de BTI.
Nenhum dos dois arquivos locais ou seus correspondentes upstream examinados
contém uma linha SPDX ou copyright identificada por busca textual. Isso não
significa ausência de proteção ou de condições de licença; o `LICENSE` e o
`NOTICE` do import s2n-bignum foram localizados no commit AWS-LC e descritos
acima, devendo ser revisados juntamente com a proveniência de cada arquivo.

Comparando os arquivos locais integralmente com `include/` no commit de origem
`4d1356a7470663c752660a59375dc3a9ef548428`, o único diff textual observado em
cada um foi a inserção no início do comentário `Auto-added during import for
AWS-LC symbol prefixing support`, do include
`<openssl/boringssl_prefix_symbols_asm.h>` e de uma linha em branco:

| Header | SHA-256 upstream | SHA-256 local modificado |
|---|---|---|
| `_internal_s2n_bignum_arm.h` | `1da38c08533d5db37383537d2b1f0bd94083e4248460af7182174d8e74681a9a` | `cb1a1884b59cc3c698d3c6d7c5bd8046e78efe40128017d214614d52cba91b3d` |
| `_internal_s2n_bignum_x86_att.h` | `4071bcd6552e0bd6cb82163b82f11cba63c6aa731c32380c5e8bfd467e9bc91b` | `a78b30d0bec06e81d50fdd7c44baff22059a39d5be6f9fe6baba7e27d99ce8e5` |

Uma sondagem controlada atual com `cc -M -x assembler-with-cpp`, incluindo
os diretórios locais `out/include` e `s2n-bignum-imported/include`, para
`mlkem_ntt_avx2_asm.S`, listou `_internal_s2n_bignum_x86_att.h`, os headers
`boringssl_prefix_symbols_asm.h` e `boringssl_prefix_symbols.h` preparados em
`out/include`, e `/usr/include/stdc-predef.h`. Ela não selecionou `cet.h` nessa
invocação. O header de prefixos preparado declara ISC e copyright Google, como
registrado acima. A sondagem **não** reproduz necessariamente flags, macros,
ordem de include ou sysroot do build histórico, e não compila nem demonstra
vínculo do objeto a um executável. A variante AArch64 não foi pré-processada ou
executada nesta etapa. Revisar os headers, licença original, modificações e
avisos de sistema para o source bundle e build final escolhido; o projeto
continua limitado às combinações Linux x86-64 efetivamente testadas.

### Fiat-crypto: fontes gerados e origem documentada

O diretório local `aws-lc/third_party/fiat/` contém oito headers gerados:
`curve25519`, `p256`, `p384` e `p521`, cada um com variantes de 32 e 64 bits,
além do `LICENSE` MIT já identificado acima. O texto MIT credita os autores
fiat-crypto e pede a inclusão do aviso de copyright e permissão nas cópias ou
porções substanciais; a aplicabilidade precisa ser avaliada por distribuição.
Os cabeçalhos de amostra `p256_64.h` e `curve25519_64.h` identificam comandos
de geração Fiat, não o commit exato que os produziu.

Na árvore AWS-LC fixada, [`third_party/fiat/METADATA`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/third_party/fiat/METADATA)
registra versão fiat-crypto `6ccc6638716d4632304baf1adbb5c47c3a12ea6f`, data
de atualização 2022-03-22 e renomeação de arquivos para `.h` para integração
BoringSSL; descreve o `LICENSE` como `LICENSE-MIT` upstream. O
[`README.md`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/third_party/fiat/README.md)
afirma que alguns arquivos foram editados manualmente e que essas edições são
marcadas `NOTE: edited after generation`. Uma busca literal dessa marca nos
oito headers locais não retornou ocorrências; isso não prova que não existam
outras modificações ou que todos provenham byte a byte do commit fiat indicado.

Os IDs blob Git de **8/8** headers locais coincidiram com os mesmos caminhos
na árvore recursiva AWS-LC fixada, não truncada. Isso comprova identidade
com esse snapshot AWS-LC, não com arquivos originais fiat-crypto ou com um
binário. No código local, `bcm.c` inclui `ec/p256.c`, `ec/p384.c`, `ec/p521.c`
e `curve25519/curve25519_nohw.c`, que referenciam variantes fiat sob condições
de word size, `OPENSSL_SMALL` ou seleção s2n-bignum. A archive intermediária
contém `bcm.o` e objetos s2n-bignum P-256/P-384/P-521 por nome; a presença de
`bcm.o` não comprova qual ramo fiat foi pré-processado nem quais funções foram
vinculadas. `curve25519_nohw.c` credita ainda código ref10 SUPERCOP 20141124
e afirma que esse código é de domínio público; essa alegação não foi verificada
independentemente. Seus avisos e a seleção de fontes/objetos devem ser revistos
para o build e source bundle finais, sem inferir compatibilidade ou obrigações
finais a partir apenas da archive.

### Ref10 SUPERCOP: evidência de proveniência e direitos

O arquivo local `crypto/fipsmodule/curve25519/curve25519_nohw.c` declara
copyright Google 2020 e SPDX ISC. Seu comentário atribui parte do código ao
ref10 Ed25519 de SUPERCOP 20141124 e afirma que esse código é de domínio
público; também identifica substituições por código gerado Fiat. Essa é uma
alegação de proveniência do arquivo, não uma comparação direta com a archive
SUPERCOP daquela data.

Como evidência independente da declaração AWS-LC, a página oficial
[Ed25519: Software](https://ed25519.cr.yp.to/software.html) afirma que o
software Ed25519 no SUPERCOP está em domínio público. A página descreve a
disponibilidade desde 20110629 e cita implementações `ref` e assembly, mas
não identifica explicitamente o diretório `ref10` ou a edição 20141124; não
estabelece, sozinha, os direitos de cada trecho do arquivo AWS-LC. Um arquivo
histórico [BoringSSL fixado](https://boringssl.googlesource.com/boringssl/+/592b532dda4553e2a07e5a48cf860338f66b0897/crypto/curve25519/curve25519.c)
declara que era majoritariamente derivado do ref10 daquela edição, afirmando
domínio público para esse código e aplicando ISC ao arquivo BoringSSL. O
[diff histórico de integração Fiat](https://boringssl.googlesource.com/boringssl/+/b2c312d670b9967cf881419758f0ec286b66a25f%5E%21)
registra a substituição de partes por código Fiat e a licença MIT deste.
Esses registros corroboram a cadeia descrita, mas não demonstram que todo o
conteúdo atual seja idêntico à versão histórica ou que seus avisos bastem para
uma distribuição.

Não foi obtida nem comparada a archive original SUPERCOP 20141124, e não se
verificou independentemente a titularidade de todos os autores, a extensão de
eventual renúncia em cada jurisdição ou a incorporação de trechos específicos.
Preservar os avisos ISC e MIT pertinentes e submeter os textos completos e
o source bundle escolhido à revisão de direitos; esta triagem não é parecer
jurídico. A inclusão textual via `bcm.c` e a presença de `bcm.o` na archive
intermediária tampouco demonstram a presença desses trechos em binário final.

**Busca da archive histórica:** em 2026-11-05, requisições HTTP `HEAD` aos
caminhos `https://bench.cr.yp.to/supercop/supercop-20141124.tar.xz`, `.tar.bz2`,
`.tar.gz` e `.tar` retornaram **404**; o caminho equivalente `.tar.bz2` em
`https://cr.yp.to/supercop/` também retornou 404. A
[página oficial SUPERCOP](https://bench.cr.yp.to/supercop.html) consultada
anuncia uma versão atual, não um link para a archive 20141124. Uma busca web
por seu nome não encontrou um exemplar verificável nessa consulta limitada.
Esses resultados não provam que a archive não exista em outro endereço ou
espelho. Não foi obtido seu conteúdo, hash ou notice, portanto a comparação
direta dos arquivos ref10 com AWS-LC continua pendente. Não executar código de
arquivos de origem desconhecida; verificar proveniência e integridade de eventual
exemplar antes de usá-lo como evidência.

**Espelho histórico identificado posteriormente:** a API GitHub do repositório
de terceiros [`floodyberry/supercop`](https://github.com/floodyberry/supercop/commit/ec7a7b6056863dbec9cfcf95cdf0596076b09e21)
apresenta o commit `ec7a7b6056863dbec9cfcf95cdf0596076b09e21`, datado de
2014-11-24, com mensagem `supercop-20141124`. Sua árvore recursiva foi
retornada como não truncada e lista 69 entradas sob
`crypto_sign/ed25519/ref10/`. Essa identificação é uma alegação do espelho:
não há comparação com uma archive oficial autenticada ou checksum original.
Não foi encontrado arquivo `README`, `LICENSE` ou `COPYING` diretamente nesse
subdiretório pelo exame dos nomes; isso não estabelece ausência de direitos
ou de avisos em outras partes da árvore. A declaração oficial Ed25519 de domínio
público, citada acima, continua sendo evidência separada.

Como amostra, o [`ge_add.c` do espelho](https://github.com/floodyberry/supercop/blob/ec7a7b6056863dbec9cfcf95cdf0596076b09e21/crypto_sign/ed25519/ref10/ge_add.c)
inclui [`ge_add.h`](https://github.com/floodyberry/supercop/blob/ec7a7b6056863dbec9cfcf95cdf0596076b09e21/crypto_sign/ed25519/ref10/ge_add.h),
que contém operações `fe_add`, `fe_sub` e `fe_mul` em coordenadas de ponto;
`curve25519_nohw.c`, função `x25519_ge_add`, contém sequência correspondente
em alto nível, mas com temporários, chamadas `fe_mul_tll`/`fe_mul_tlt`/`fe_mul_ttl`
e `fe_carry` diferentes. Essa semelhança é coerente com a proveniência alegada,
mas não comprova cópia byte a byte, equivalência funcional ou quais trechos
foram incorporados ao objeto final. Também se leu o `sign.c` do espelho, sem
aviso de licença no conteúdo examinado; os demais arquivos e eventuais avisos
fora do diretório não foram auditados integralmente. Nenhum código do espelho
foi executado. A revisão de direitos e notices para cada artefato continua aberta.

**Busca textual adicional no espelho:** foram lidos os 69 blobs listados sob
`crypto_sign/ed25519/ref10/` no commit identificado; todos retornaram HTTP 200.
Uma busca sem distinção de maiúsculas pelas expressões `copyright`, `license`,
`licence`, `public domain`, `SPDX` e `permission is hereby` não encontrou
ocorrências nesses arquivos. É uma busca limitada a essas expressões e a esse
diretório, não uma auditoria de avisos em todo o SUPERCOP ou uma conclusão de
que os arquivos não tenham licença. A declaração do site oficial Ed25519
continua separada da alegação de identidade do espelho.

Como amostras adicionais, `ge_p2_0.c` e `ge_p3_0.c` no espelho inicializam
as mesmas coordenadas com zero e um, na mesma ordem, que as funções homônimas
em `curve25519_nohw.c`; a forma dos argumentos difere. Já `ge_p1p1_to_p2.c`
usa três chamadas `fe_mul` sobre os mesmos pares de coordenadas que
`x25519_ge_p1p1_to_p2` do AWS-LC, que usa `fe_mul_tll` e argumentos por
endereço. Essas semelhanças em funções curtas são evidência contextual, não
prova independente de origem, identidade integral ou equivalência matemática.
Nenhum código foi executado, e o commit do espelho permanece sem comparação
com archive oficial autenticada. Avaliar os avisos de cada bundle e sua
aplicabilidade com revisão de direitos antes de distribuir.

**Metadados externos a `ref10` no espelho:** no mesmo commit, o arquivo
[`version`](https://github.com/floodyberry/supercop/blob/ec7a7b6056863dbec9cfcf95cdf0596076b09e21/version)
contém `20141124`, corroborando a mensagem do commit dentro do próprio espelho,
mas não autenticando o conteúdo perante a archive oficial. A árvore recursiva
não truncada lista no diretório ancestral `crypto_sign/ed25519/` os arquivos
`description`, `designers` e checksums, sem `LICENSE`, `COPYING` ou `README`
direto. `description` descreve assinaturas EdDSA com Curve25519; o arquivo
[`designers`](https://github.com/floodyberry/supercop/blob/ec7a7b6056863dbec9cfcf95cdf0596076b09e21/crypto_sign/ed25519/designers)
nomeia Daniel J. Bernstein, Niels Duif, Tanja Lange, Peter Schwabe e Bo-Yin
Yang como designers. Isso não identifica, por si só, os titulares dos arquivos
ref10 ou os autores de todas as modificações.

Uma busca por nomes de arquivos de aviso em toda a subárvore Ed25519 do
espelho também identificou `implementors` para `ref` e duas implementações
amd64, mas não para `ref10`. O
[`ref/implementors`](https://github.com/floodyberry/supercop/blob/ec7a7b6056863dbec9cfcf95cdf0596076b09e21/crypto_sign/ed25519/ref/implementors)
lista os mesmos cinco nomes e designa Peter Schwabe como lead dessa implementação
`ref`; não se deve transferir essa atribuição automaticamente para `ref10`.
Não foram identificados arquivos de aviso nos nomes diretos da raiz do espelho
por esse levantamento; isso não é uma busca integral do conteúdo de toda a
archive nem prova inexistência de avisos em outros lugares. Não se executou
código. Permanecem necessárias autenticação independente da archive histórica,
análise de direitos e revisão por artefato antes da distribuição.

**Comparação com segundo espelho:** a árvore do commit
[`anonimal/supercop-ed25519-ref10@5019e3e8ac782cac768d2462f5f8c72ad69a7a83`](https://github.com/anonimal/supercop-ed25519-ref10/tree/5019e3e8ac782cac768d2462f5f8c72ad69a7a83)
registra mensagem `supercop-20160910` e data de autoria 2016-09-13. Comparando
por caminho relativo os IDs de blob Git dos 69 arquivos do diretório ref10 do
espelho 20141124 com os da raiz desse segundo espelho, **69 coincidiram, zero
divergiram e zero estavam ausentes**; ambas as respostas de árvore recursiva
da API GitHub indicaram `truncated: false`. A comparação cobre os 69 arquivos
da primeira árvore, não afirma que a segunda não tenha arquivos adicionais.

Esta concordância entre cópias publicadas em repositórios diferentes reforça a
identidade textual desses 69 arquivos conforme os IDs Git, mas não prova que
suas origens sejam independentes, que qualquer uma seja a archive oficial de
20141124, nem estabelece uma cadeia de custódia ou direitos. O segundo espelho
se identifica como versão posterior e não autentica retroativamente o pacote
inteiro. O README desse repositório declara domínio público, mas também é
afirmação de espelho, distinta da declaração oficial Ed25519 já citada. Nenhum
código foi executado; a revisão jurídica e por artefato continua pendente.

**Busca adicional em arquivos públicos:** a consulta à API Wayback
`archive.org/wayback/available` para
`bench.cr.yp.to/supercop/supercop-20141124.tar.xz` com timestamp `20141124`
retornou HTTP 200 e `archived_snapshots: {}`. A busca CDX com wildcard pelos
formatos da archive esgotou o tempo, logo **não** comprovou ausência de
capturas. A API Zenodo, consultada pelo termo exato `"supercop-20141124"`,
retornou zero resultados; uma busca web por archive e SHA-256 também não
identificou checksum histórico verificável. O [registro Zenodo localizado](https://zenodo.org/records/13795834)
refere-se à versão 20240909, não à 20141124, e seu MD5 não autentica a archive
histórica nem o espelho. As buscas têm escopo limitado e não excluem outras
fontes, nomes ou arquivos. Não foi obtido material novo para comparação direta
com o espelho e não se executou código; a identificação original continua sem
autenticação independente.

O notice composto apresenta fiat, s2n-bignum e jitterentropy na seção de
bibliotecas compiladas em libcrypto/libssl; identifica Google Test, runner Go e
Wycheproof como usados somente para testes/build. Essa é uma classificação do
próprio notice, não uma verificação do build concreto. No material local há
regras CMake para fontes jitterentropy e s2n-bignum, sujeitas a condições de
build e arquitetura; o builder Rust está em `builder/`, não em `build/`.
Regras presentes não demonstram sua execução nem sua inclusão final. Fontes,
ferramentas e testes distribuídos também precisam de revisão própria, mesmo
quando não vinculados ao binário.

O notice descreve jitterentropy como duplamente licenciado e registra a escolha
da Amazon pela BSD-3-Clause; não se deve generalizar essa escolha para todo
componente ou distribuição sem revisar os arquivos e contexto. Keccak/AES são
descritos no notice como contribuições CC0 com autores em comentários de fontes;
esses comentários e sua aplicabilidade ao artefato ainda não foram auditados.

## Pendências

Definir artefatos, targets e features; rastrear sources e objetos efetivamente
compilados/empacotados, inclusive TLS e bibliotecas dinâmicas; verificar textos
upstream faltantes e avisos em cabeçalhos; analisar o grafo restante e preparar
atribuições integrais com revisão jurídica. Esta triagem não resolve os bloqueios
de empacotamento, direitos das fixtures ou a regressão de admissão; não autoriza
push, tag, release ou publicação.
