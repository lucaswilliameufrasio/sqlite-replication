# Estado dos notices por artefato — revisão local parcial

Esta matriz registra evidência disponível, não aprovação jurídica, licença final
ou autorização para distribuir. Os candidatos foram gerados antes dos commits
`578f56e`, `c70c1dd` e `22c8ba0`; a comparação posterior dos arquivos de código
e fixtures está registrada abaixo. A documentação desta matriz foi ampliada
depois da geração da archive Rust, que não representa automaticamente a árvore
inteira atual. Os SBOMs originais e suas cópias em quarentena foram preservados.

## Conferência de entradas

Os SHA-256 abaixo foram conferidos no checkout atual e coincidem com os
registrados em `README.md` para a triagem anterior:

| Entrada | SHA-256 |
|---|---|
| `go/go.mod` | `f5bfda4168e3619d018e4a284d245fabcab41f13f8921ac1b6262d1548d1c8b2` |
| `go/go.sum` | `208cbd4dee9e3ec082d9dc710bec018ac04915db306fe4555813be5f59f1caa7` |
| `rust/Cargo.toml` | `94f7a02b159a500b1da2f0f8d4ab864a30fc819be31c05039332c6d5dfd5f962` |
| `rust/Cargo.lock` | `7083d5346548c1fea412bd9cdb33db7dda1d1d81bf58c70f02654d4d8cd3bb96` |

Essas igualdades não comprovam composição efetiva, seleção de features ou
dependências de outros workspaces; tampouco verificam o conteúdo dos SBOMs.

## Estado por possível artefato

| Artefato | Evidência existente | Pendência antes de distribuição |
|---|---|---|
| Fontes Go e consumidores | SBOMs Go e `native-notices-review.md`, escopo Linux amd64. | Identificar conteúdo efetivo e revisar textos integrais e avisos aplicáveis; manter em quarentena a classificação automática controversa de `modernc`. |
| Workspace Rust com fixtures | `rust-workspace-candidate-notice-matrix.md` e inventário de archives anteriores. | Gerar e examinar candidato do estado escolhido, verificar arquivos e avisos incluídos e revisar textos integrais; candidatos antigos não cobrem mudanças posteriores. |
| Crates Rust individuais | `distribution-candidate-inventory.md` relata dependências locais e fixtures ausentes em pacotes isolados. | Resolver estratégia de empacotamento e verificar cada pacote extraído; não presumir que o workspace validado prove crates independentes. |
| Binários Go/Rust | `native-notices-review.md`, `sqlite-rust-transitive-notices.md` e `s3-aws-lc-transitive-notices.md` são triagens de configurações anteriores. | Definir binários, targets e features; verificar componentes efetivos, SQLite, AWS-LC e avisos correspondentes. |
| Exemplos de telemetria | `telemetry-examples/` tem inventários separados; exemplo Rust é workspace separado. | Definir inclusão, revisar suas dependências e confirmar os termos próprios do exemplo. |
| Fixtures e documentos SBOM | `fixture-provenance-review.md`, inventário anterior e originais/quarentena preservados. | Conferir os arquivos efetivamente incluídos e atribuições aplicáveis; confirmação do usuário sobre direitos de contribuições e fixtures não substitui revisão dos avisos de terceiros. |

As revisões identificadas são parciais. A definição do conjunto final de
artefatos e a aprovação de distribuição continuam distintas da validação local;
nenhuma publicação, tag, licença final ou push está autorizada por este registro.

### Triagem adicional: constantes ML-DSA x86-64 AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ml_dsa/mldsa/native/x86_64/src/` tiveram SHA-256
locais coincidentes com o índice. Os cabeçalhos creditam autores de mldsa-native
e declaram SPDX `Apache-2.0 OR ISC OR MIT`; `arith_native_x86_64.h` também
credita autores de mlkem-native:

| Arquivo | SHA-256 |
|---|---|
| `arith_native_x86_64.h` | `d9e0ddca1f4d9d4dbe6f579989f0454d9d699a308d4118b96fc9f85f10f1eecc` |
| `consts.c` | `1a07207e7c98d8b7f2ab47ea1184e8dc5620a29eb12e9e7afee9cea57309e433` |
| `consts.h` | `8329b71eeb273a2d09ffe57c4d7d184a3c4c1eb2f63255f41ccdefceab705e7b` |

Os arquivos de constantes declaram geração por `scripts/autogen` no repositório
mldsa-native; não foram reproduzidos. A conferência cobre somente os cabeçalhos
indexados e essa indicação, não outros avisos ou proveniência integral. A
revisão dos imports e objetos observados consta em
`s3-aws-lc-transitive-notices.md`; a presença de objeto em archive nativa local
não prova ligação em binário final. A crate não é vendorizada na archive-fonte
Rust candidata `8avAZthl`. Notices por artefato permanecem abertos; o Go não foi
examinado nesta etapa.

### Triagem adicional: interface ML-KEM AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ml_kem/` tiveram SHA-256 locais coincidentes com o
índice. Seus cabeçalhos creditam Amazon.com ou afiliadas e declaram SPDX
`Apache-2.0 OR ISC`:

| Arquivo | SHA-256 |
|---|---|
| `ml_kem.h` | `0709c8738f69680c9c108cc96e2dfea9410bfa95f0ff8b9cdd43d1a3829deb1b` |
| `ml_kem_test.cc` | `c1226eaea984f6ab0fecf96e49201ace1373ce2e1a3de42aeed06e240b5980a1` |
| `mlkem_native_backend.h` | `7fd60dc8b4380e7c73cf5310deb9b182c2e7cf08e6f8a6aaa655e297c81a44af` |
| `mlkem_native_config.h` | `5a2766bc43ecfb8f6cfb98011c5f763726bc8ef8ea74cd043554b50585e8a514` |

Esta conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. A revisão dos imports e objetos observados
consta em `s3-aws-lc-transitive-notices.md`; a presença de objeto em archive
nativa local não prova ligação em binário final ou inclusão do teste. A crate
não é vendorizada na archive-fonte Rust candidata `8avAZthl`. Notices por
artefato permanecem abertos; o Go não foi examinado nesta etapa.

### Triagem adicional: amostragem ML-KEM AWS-LC

Seis arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ml_kem/mlkem/` tiveram SHA-256 locais coincidentes com
o índice. Seus cabeçalhos creditam autores de mlkem-native e declaram SPDX
`Apache-2.0 OR ISC OR MIT`:

| Arquivo | SHA-256 |
|---|---|
| `sampling.c` | `9263037c401d0cc4130a6f0dba76299d170b498c330f5d5c32b5ae5921bacd5f` |
| `sampling.h` | `d25df90afc96a02a794dcb737236b2cb42cbd2db411ccaf274cf5ea3bf5e3c87` |
| `symmetric.h` | `168d0ec3bf50bd45cb0a103d7984e9ccf52f78697718b7e85efdf1beda7abc95` |
| `sys.h` | `0ead27bb8c8879b62b29b9bb86c36aeaac270598d42ee2f2258a478eed8efa5c` |
| `verify.c` | `f8f89f1b8c8e5811ce134c0d90fb7828027336afeed231dc3ccb7ee6b73b9335` |
| `verify.h` | `76fd0af32577f9118d1ea90a66ff77e18cdf99d08dcf1c9df137833041e8492f` |

Esta conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência integral. A revisão dos imports e objetos observados consta em
`s3-aws-lc-transitive-notices.md`; a presença de objeto em archive nativa local
não prova ligação em binário final. A crate não é vendorizada na archive-fonte
Rust candidata `8avAZthl`. Notices por artefato permanecem abertos; o Go não foi
examinado nesta etapa.

### Triagem adicional: polinômios ML-KEM AWS-LC

Seis arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ml_kem/mlkem/` tiveram SHA-256 locais coincidentes com
o índice. Os cabeçalhos creditam autores de mlkem-native e declaram SPDX
`Apache-2.0 OR ISC OR MIT`:

| Arquivo | SHA-256 |
|---|---|
| `params.h` | `450fe3e0e50496921920473ae4321660f178c23d51f1453f3c537ee63c4158cb` |
| `poly.c` | `f0e0643a2c3743181b7927a9d47af0930ce7870a9d79671f64e6089fe028982e` |
| `poly.h` | `86224c6d1e7ec5a4c0dcd829ae90c0cb43b4855d84dd719973d47e71b8e962d4` |
| `poly_k.c` | `56130249963b5b62e97dc6413adeed21f4e542da6531e34cf640baca161fa5ff` |
| `poly_k.h` | `839852fd9c8d155f37a7a07dc1116c2ea18fade754b1d882f73c4affef5e62d3` |
| `randombytes.h` | `137eb6e5854b8e58c2caec6eea86530c8e7495706f6f5b3e007e25a9875e164f` |

A conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. A revisão dos imports e objetos observados
consta em `s3-aws-lc-transitive-notices.md`; a presença de objeto na archive
nativa local não prova ligação em binário final. A crate não é vendorizada na
archive-fonte Rust candidata `8avAZthl`. Notices por artefato permanecem
abertos; o Go não foi examinado nesta etapa.

### Triagem adicional: constantes ML-KEM x86-64 AWS-LC

Cinco arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ml_kem/mlkem/native/x86_64/src/` tiveram SHA-256
locais coincidentes com o índice. Seus cabeçalhos creditam autores de
mlkem-native e declaram SPDX `Apache-2.0 OR ISC OR MIT`:

| Arquivo | SHA-256 |
|---|---|
| `arith_native_x86_64.h` | `d6a1cc9de9e36702bbecd52d360096eab50f55798a4d61fa113819ba092a355d` |
| `compress_consts.c` | `d6cf85fb275e35f5bb84f1d17ea3fcd216741160f84a0963deaf2b82ffbba641` |
| `compress_consts.h` | `b801a347a9eaa4f3e75ee162374d83ef8fac5e405361cf8379625ab718c811e4` |
| `consts.c` | `92b37dd2361425c2816f624a982b64bbb4b4a7563f02604470c7541a83acf435` |
| `consts.h` | `f0046a67ebd4195f6180b179c0fe329c1bbfddc37203825a5ebf0dffa7991e15` |

Os quatro arquivos de constantes declaram geração por `scripts/autogen` no
repositório mlkem-native; não foram reproduzidos. A conferência cobre esses
cabeçalhos indexados, não todos os avisos ou a proveniência integral. A revisão
geral dos imports e dos objetos observados consta em
`s3-aws-lc-transitive-notices.md`; um objeto em archive nativa local não prova
ligação em binário final. A crate não é vendorizada na archive-fonte Rust
candidata `8avAZthl`. Notices por artefato permanecem abertos; o Go não foi
examinado nesta etapa.

### Triagem adicional: fontes mlkem-native AWS-LC

Seis arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ml_kem/mlkem/` tiveram SHA-256 locais coincidentes com
o índice. Os cabeçalhos creditam autores de mlkem-native e declaram SPDX
`Apache-2.0 OR ISC OR MIT`:

| Arquivo | SHA-256 |
|---|---|
| `compress.c` | `a13d44b3a96962ae982722fba8b1ff6744f16e7b3b760883edfe762061cb26b5` |
| `compress.h` | `746ce7e55d088f75220ca0a5b08fc515c033167cbcacfd74be127e14f1f5e128` |
| `indcpa.c` | `526505cbf09ab80e9644563eb808373c502aad2270a0b53c8370510ea4c0b70d` |
| `indcpa.h` | `ac6f47c959fdc97aa1882bca5f78ecfcee595ff21769f937866f2c1d1bf29109` |
| `kem.h` | `ba22bd6de66141138329aaf53aedc73350c4ceaa241a341d0e5136c5df493dbd` |
| `mlkem_native_bcm.c` | `dfac4c6d9ec898ef57e893e291716e2817a605ac272bd59d8b9bc0fdce72ac19` |

O último arquivo declara geração por `scripts/autogen` no repositório
mlkem-native; não foi reproduzido. A conferência cobre os cabeçalhos indexados,
não todos os avisos ou a proveniência integral. A revisão geral dos imports e
dos objetos observados consta em `s3-aws-lc-transitive-notices.md`; a presença
de um objeto na archive nativa local não prova sua ligação em binário final.
A crate não é vendorizada na archive-fonte Rust candidata `8avAZthl`. Notices
por artefato permanecem abertos; o Go não foi examinado nesta etapa.

### Triagem adicional: fontes jitterentropy AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/third_party/jitterentropy/jitterentropy-library/src/` tiveram SHA-256
locais coincidentes com o índice. Os cabeçalhos remetem ao `LICENSE` da raiz e
incluem disclaimer:

| Arquivo | SHA-256 | Copyright no cabeçalho |
|---|---|---|
| `jitterentropy-gcd.c` | `a765116ee03085d26594917312426d4322c7c3c1468a400592da5bc6ef2f517d` | Joshua E. Hill e Stephan Mueller, 2021–2025. |
| `jitterentropy-health.c` | `2bba9859af2eecc346508724911db5dfa34e822f90660e5d173a3d1c64843c26` | Joshua E. Hill e Stephan Mueller, 2021–2025. |
| `jitterentropy-noise.c` | `6fdbc48eaa0d6150bf213083a7db1f6c924589104547e934dfe598a045c3a7a1` | Stephan Mueller, 2021–2025. |
| `jitterentropy-timer.c` | `765935f9f05ea8d9737b9a6d08e87200c21be352c11aac95f1e6163b6d417efd` | Stephan Mueller, 2021–2025. |

A revisão de `LICENSE` upstream e a observação dos objetos
`jitterentropy-{gcd,health,noise,timer}.o` em uma archive nativa local já estão
em `s3-aws-lc-transitive-notices.md`. A presença dos objetos nessa archive não
prova sua ligação em binário distribuído; esta triagem dos cabeçalhos também
não estabelece todos os avisos. A crate não é vendorizada na archive-fonte Rust
candidata `8avAZthl`; notices por artefato permanecem abertos e o Go não foi
examinado nesta etapa.

### Triagem adicional: tabelas ML-DSA AArch64 AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ml_dsa/mldsa/native/aarch64/src/` tiveram hashes
locais coincidentes com o índice. Os cabeçalhos creditam autores de mldsa-native
e declaram SPDX `Apache-2.0 OR ISC OR MIT`; o header também credita autores de
mlkem-native:

| Arquivo | SHA-256 |
|---|---|
| `aarch64_zetas.c` | `afe51c739e24487d4b5f3c9021ff246fb375717d48b9e55be17ea94cfada8ed1` |
| `arith_native_aarch64.h` | `0b5ef0f298404840b8ed38fc3728184a9905f44e6f1868be93b8d779e5640364` |
| `polyz_unpack_table.c` | `09f85b0ebce82d274b0a01dd7d4dca940c363702f59e2cec025289fe0455d366` |
| `rej_uniform_table.c` | `a70b3888a426cb7fc1abf8faaba72bb1f8fd46bad8b650a0c9b0e8b682e18e64` |

Os três arquivos `.c` declaram geração por `scripts/autogen` no repositório
mldsa-native; não foram reproduzidos. A conferência cobre os cabeçalhos
indexados e essa indicação, não outros avisos ou a proveniência integral. A
revisão geral dos imports consta em `s3-aws-lc-transitive-notices.md`. O grafo
Linux x86-64 da extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova
compilação das variantes AArch64 ou inclusão em binário. A crate não é
vendorizada na archive-fonte candidata; o Go não foi examinado nesta etapa.

### Triagem adicional: suporte mldsa-native AWS-LC

Seis arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ml_dsa/mldsa/` tiveram SHA-256 locais coincidentes com
o índice. Todos os cabeçalhos creditam autores de mldsa-native e declaram SPDX
`Apache-2.0 OR ISC OR MIT`; os três marcados abaixo também creditam autores de
mlkem-native:

| Arquivo | SHA-256 | Também credita mlkem-native |
|---|---|---|
| `cbmc.h` | `355704060689792c1ef7307844b486ac54bd397db5474a912cdf1d5ee4e0d2e7` | Não. |
| `context.h` | `c75d2d0f6555230e5fa6ea9147d523d5375fba247b314fe080d0454f89267719` | Não. |
| `ct.h` | `706be081c657b742247b25588b6eb75f399f73b84b33ef20179ceeb721731078` | Sim. |
| `debug.c` | `0200f23feaebfd12d35b39232c2ae566f7322d86993f981a705ddc6355a42edc` | Sim. |
| `debug.h` | `4effd0eef9b63bdf434369b69ec5d14293ad7d22031b4f5ba237c69ce2fda383` | Sim. |
| `mldsa_native_bcm.c` | `00848e250a1a9859b379d2dc10704bbc9e63b7a28b4ebab9d64e26d44f254247` | Não. |

`ct.h` lista referências a FIPS 204, libmceliece e uma publicação sobre
opt-blockers; a simples referência não fixa revisão nem prova cópia. O cabeçalho
de `mldsa_native_bcm.c` diz que é gerado por `scripts/autogen`; não foi
reproduzido. A revisão geral dos imports consta em
`s3-aws-lc-transitive-notices.md`. Esta triagem cobre somente os cabeçalhos
indexados e comentários citados, não todos os avisos ou proveniência integral.
O grafo Linux x86-64 da extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não
prova compilação desses arquivos ou inclusão em binário; a crate não é
vendorizada na archive-fonte candidata. O Go não foi examinado nesta etapa.

### Triagem adicional: interface ML-DSA AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ml_dsa/` tiveram SHA-256 locais coincidentes com o
índice. Seus cabeçalhos creditam Amazon.com ou afiliadas e declaram SPDX
`Apache-2.0 OR ISC`:

| Arquivo | SHA-256 |
|---|---|
| `fips202_glue.h` | `b4124d859df641dd4b09697fd8c1c29ffbbf6b85db9c4dbd9c57fc2449b57a0f` |
| `fips202x4_glue.h` | `79a9771028aba7cfa3e25acac6ee4901eaf7133c0998d86781179e0954fcbc16` |
| `make_corrupted_key_tests.cc` | `a1d5733ea2831f72463f71274ffdbcd0d3d241ade08e3b6b4dd57c7ca76de53f` |
| `ml_dsa.h` | `bdcce2a8995c40cecafd5a1d25d689959a4505f2be1457df7487296c5019e050` |

O comentário de `fips202x4_glue.h` descreve uma interface entre mldsa-native e
a API AWS-LC; `make_corrupted_key_tests.cc` descreve geração de vetores de
teste com chaves corrompidas intencionalmente. A conferência cobre os cabeçalhos
indexados e esses comentários, não todos os avisos ou a proveniência integral.
O grafo Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona
`aws-lc-sys`, mas não prova compilação desses arquivos ou inclusão dos vetores
em binário. A crate não é vendorizada na archive-fonte candidata. Notices por
artefato seguem abertos; o Go não foi examinado nesta etapa.

### Triagem adicional: suporte do módulo AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `delocate.h` | `49b97155b7100743a0304fabb16a1997564ac181e7336722a00317df8d041c8b` | Google Inc., 2017; SPDX `ISC`. |
| `fips_shared_library_marker.c` | `7f77570e916a04aed7097b2749442c9d037985b727dc7b0beb6620fc79168c79` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`; descreve marcadores de compilação compartilhada para macOS, iOS e Windows. |
| `fips_shared_support.c` | `1c407ee0ab906335d5b4b2b5174fe9e0e0d354529e64da6b0738cfb6e648c00d` | Google Inc., 2019; SPDX `ISC`. |

A conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral. O grafo Linux x86-64 já registrado para a extração Rust
`8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação desses arquivos ou
uso da configuração compartilhada descrita. A crate não é vendorizada na
archive-fonte candidata. Notices por artefato permanecem abertos; o Go não foi
examinado nesta etapa.

### Triagem adicional: KDF e MD5 AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `kdf/kbkdf.c` | `a2164a6dbef96fdacaafe0d1496d1f913d4387f7cdedcaa5a75d47346aee93f8` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`. |
| `kdf/sskdf.c` | `939b0ec80f57fa35c35e9237a7530f41032797ebf9d320fe286537304a042423` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`. |
| `md5/md5.c` | `95e047e61fd61f14792f9546a1978b9d115c360c833a5fda33c16aaacde0b2c4` | Eric Young, 1995–1998; SPDX `Apache-2.0`. |
| `md5/md5_test.cc` | `2f732fc20062d10ff94ded38436e665c86d29da5952ed609d052ee1278d10db1` | Google Inc., 2018; SPDX `ISC`. |

Esta conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação dos
arquivos ou inclusão do teste em binário. A crate não é vendorizada na
archive-fonte candidata. Notices por artefato permanecem abertos; o Go não foi
examinado nesta etapa.

### Triagem adicional: adaptadores EVP AWS-LC

Cinco arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/evp/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `p_hkdf.c` | `e1855a6c15f0e7b5c1f47540a15cc27c759858683d4f1d19f55d25b2dcecc959` | Google Inc., 2022; SPDX `ISC`. |
| `p_hmac.c` | `2937ac10e3111b1a0df90c056b2ca7bdb21339a2b257183908c4346ab84cb6f6` | OpenSSL Project, 2007; escrito por Stephen N Henson; SPDX `Apache-2.0`. |
| `p_kem.c` | `e9e4f5d52ed01ead0a79763afac33700cd13a90ce038ee26e1e871899f68aeeb` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`. |
| `p_pqdsa.c` | `46a05649f08121fe162d980ecc7c179b72ed6e902104b6ccbdaf64ce165e65d1` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`. |
| `p_rsa.c` | `f5e41d71cfef0837d0444b65f757c7a21cd9f8815daeddb7af7305c3b25e388e` | OpenSSL Project, 2006; escrito por Stephen N Henson; SPDX `Apache-2.0`. |

A conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: contexto e chaves EVP AWS-LC

Cinco arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/evp/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `evp_ctx.c` | `f1f84cc4acf72e70b3dcca07e50696d3be72326e8395b3c96e7eab2eaed1ccd4` | Eric Young, 1995–1998; SPDX `Apache-2.0`. |
| `evp_ctx_test.cc` | `f8b340110a25fff034d6f987d0298a5a3662c04dfb4c4d4cd50093dc95ea39cf` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`. |
| `p_ec.c` | `4d703bc34795f72d8e83055ee450dc0c8a827f5fc0ecda1da65cd216acbe9b34` | OpenSSL Project, 2006; escrito por Stephen N Henson; SPDX `Apache-2.0`. |
| `p_ed25519.c` | `ea2efd95cf23d9b43fc634515cdff6f550b8692fd65bd7d9bd69bdb2f7be5b5a` | Google Inc., 2017; SPDX `ISC`. |
| `p_ed25519ph.c` | `d5922060e1d73106c422680e78852a9eb0eaa1bfefee681d5a3a416553d44a27` | Google Inc., 2017; SPDX `ISC`. |

A conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral. O grafo Linux x86-64 já registrado para a extração Rust
`8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação desses arquivos ou
inclusão do teste em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: ECDH e EVP AWS-LC

Dois arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `ecdh/ecdh.c` | `031e24e980825968d893652becb013923ea7b3b4cc6a70ef9da698799d7d90b7` | Sun Microsystems, 2002, e OpenSSL Project, 2000–2002; atribui a autoria original de ECDH a Douglas Stebila e descreve contribuição de código ECC ao OpenSSL; SPDX `Apache-2.0`. |
| `evp/evp.c` | `467008f6e90a5767a454c0453e7ad9085d14f177f6617ceb2e47d9d6a026ad24` | Eric Young, 1995–1998; SPDX `Apache-2.0`. |

A conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: suporte P-256 AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ec/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `p256-nistz.h` | `14d5fba009e6aac71930552f1dead9f31b0d599d0ee214366ed68916d8318294` | OpenSSL Project Authors, 2014–2016, e Intel, 2014; nomeia Shay Gueron e Vlad Krasnov; SPDX `Apache-2.0`. |
| `p256-nistz_test.cc` | `09b94a42f86a2c72289231477b3804027361ed37abc8a584de6eee570e4ae36b` | Google Inc., 2016; SPDX `ISC`. |
| `p256_table.h` | `af40920bba1203aa6ffd3a5da819bbefc66aadc8f98fbba19c553dbc4183abca` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`; declara geração por `make_tables.go`. |

A conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos; a tabela gerada não foi reproduzida. O
grafo Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona
`aws-lc-sys`, mas não prova compilação desses arquivos ou inclusão do teste em
binário. A crate não é vendorizada na archive-fonte candidata. Notices por
artefato permanecem abertos; o Go não foi examinado nesta etapa.

### Triagem adicional: aritmética EC AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ec/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `scalar.c` | `c80c94a92d3633b53b20ecf270d340a0e0c8679062ef54e8be35e35fc8587d07` | Google Inc., 2018; SPDX `ISC`. |
| `simple.c` | `a945ab61ea2232e6c9ebc9554557820d34bc284e09f9248a77c854a48fc72be9` | OpenSSL Project e Sun Microsystems; atribui autoria original a Bodo Moeller e a aritmética polinomial binária a Sheueling Chang Shantz e Douglas Stebila; SPDX `Apache-2.0`. |
| `simple_mul.c` | `7301f6e919be1a43ca7927f1d2ac86db7d0b5903308dd3d7cb400250235f31cd` | Google Inc., 2018; SPDX `ISC`. |
| `wnaf.c` | `eb8fada0f7569f6afb68fd8c0a14f884b135287b98f0bbceb727d6551548870c` | Mesmos créditos e SPDX de `simple.c`. |

Esta conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: integração Curve25519 AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/curve25519/` tiveram SHA-256 locais coincidentes com
o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `curve25519_s2n_bignum_asm.c` | `e685219f3987834e5022c78190864765f574e52945d6aecadf77ddd6b19d635b` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`; inclui interface s2n-bignum sob condição de compilação. |
| `curve25519_tables.h` | `3c3e5be8fb0ed8fe43890fdf1659af42c580da69b9de45b51a780a99bcd671ca` | Google, 2020; SPDX `ISC`; indica geração por `make_curve25519_tables.py`. |
| `ed25519_test.cc` | `e3570a000480e65329aac0c6a741a920c2562da56559c365901ad2c15a3f5a93` | Google, 2015; SPDX `ISC`. |
| `x25519_test.cc` | `bd1dee52c8cfa60aa42267df853b6a6a39cdc8e138559afc06242370ce813a6c` | Google, 2015; SPDX `ISC`. |

A conferência cobre os cabeçalhos indexados e a indicação de geração, não todos
os avisos ou a proveniência integral. A tabela não foi reproduzida. O grafo
Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona `aws-lc-sys`,
mas não prova compilação desses arquivos ou inclusão dos testes em binário. A
crate não é vendorizada na archive-fonte candidata. Notices por artefato seguem
abertos; o Go não foi examinado nesta etapa.

### Triagem adicional: outras capacidades de CPU AWS-LC

Nove arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/cpucap/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `cpu_aarch64_sysreg.c` | `5363f39123b48764fd1d0fe3ad603e73db3404f075cb1cb5cf6ec10c53d31b5b` | Google, 2023; ISC. |
| `cpu_aarch64_win.c` | `7a02bdedb4fb108d6cffb8e4184d848cdc7694c523e4212764e8a786c8c94dff` | Google, 2018, e Arm Ltd., 2020; ISC. |
| `cpu_arm_freebsd.c` | `e9416024979a21a9247b2236ca8027038b28124d829cefe5a5b80c940f4049c5` | Google, 2022; ISC. |
| `cpu_arm_linux.c` | `e65bd2c2523e6be158a408a53473885158c72201eda417c0582ba2b7028c85bb` | Google, 2016; ISC. |
| `cpu_arm_linux.h` | `7a92b59948afb9a715be394fd38af10eecfe74fed9ed882cb276f81223bac8a7` | Google, 2018; ISC. |
| `cpu_arm_linux_test.cc` | `31247ea5b7ca74fabbebd413748a32f0a230a74a9c65e8281c3e5d3a2b9d9a0d` | Google, 2018; ISC. |
| `cpu_getauxval_linux.h` | `119927d6ca410958720d74ef70de3ab56c2e1f3828e9278357d1b06761c0ea51` | Amazon ou afiliadas; `Apache-2.0 OR ISC`. |
| `cpu_intel.c` | `c48e87873debc9c15e159e4af280dcb5785942edf0bd1a8e39006799072e3ee3` | Eric Young, 1995–1998; `Apache-2.0`. |
| `cpu_ppc64le.c` | `fa70b2639b9d0920c296f2b9d78976e1853b623f337a90c1dd0b8345ba28e8f6` | Google, 2016; ISC. |

As expressões acima são as declarações SPDX dos respectivos cabeçalhos. A
conferência cobre esses trechos indexados, não outros avisos ou a proveniência
integral dos arquivos. O grafo Linux x86-64 já registrado para a extração Rust
`8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação de cada variante ou
inclusão do teste em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: suporte CPU AArch64 AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/cpucap/` tiveram SHA-256 locais coincidentes com o
índice. Seus cabeçalhos creditam Amazon.com ou afiliadas e declaram SPDX
`Apache-2.0 OR ISC`:

| Arquivo | SHA-256 |
|---|---|
| `cpu_aarch64.c` | `038455923f8851e8bfa8083b669cc1445288117262a8a8e637668d38fa358c1f` |
| `cpu_aarch64.h` | `1bb061b73ca61fc20d0870227cdf06fce5a817c5ab0cb58b35c331f3b7c67ed8` |
| `cpu_aarch64_dit_test.cc` | `e57bd4712994427a8f12ecffade9dd2ec94dab7bbbab6061c393554405cbbffa` |

O trecho inicial de `cpu_aarch64.c` condiciona seu código a `OPENSSL_AARCH64`
e à ausência de `OPENSSL_STATIC_ARMCAP`. A conferência cobre os cabeçalhos
indexados, não outros avisos ou a proveniência integral. O grafo Linux x86-64
já registrado para a extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não
prova compilação desses arquivos ou inclusão do teste em binário. A crate não é
vendorizada na archive-fonte candidata. Notices por artefato permanecem abertos;
o Go não foi examinado nesta etapa.

### Triagem adicional: CPU AArch64 BSD AWS-LC

Dois arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/cpucap/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `cpu_aarch64_netbsd.c` | `2a601fdede7e5e680564848c61517b7220394ba90922a7cc5212c6d25cc58efa` | Amazon.com ou afiliadas; SPDX `Apache-2.0 OR ISC`. |
| `cpu_aarch64_openbsd.c` | `e0347b274ee840941820b71bb9260e6cba8e4b4bfab14451b2293e2b8726b66b` | Robert Nagy, 2022; SPDX `ISC`. |

Os trechos iniciais condicionam as implementações às respectivas plataformas
AArch64. A conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação dessas
variantes ou inclusão em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: detecção de CPU AArch64 AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/cpucap/` tiveram SHA-256 locais coincidentes com o
índice. Seus cabeçalhos creditam Google Inc. e declaram SPDX `ISC`:

| Arquivo | SHA-256 | Ano no cabeçalho |
|---|---|---|
| `cpu_aarch64_apple.c` | `96f41efe48b7d3c4b77bcf7f47316f9bf1c72093c616ba46921ec7d2bedd1548` | 2021 |
| `cpu_aarch64_freebsd.c` | `63590becdc4d9609cac5cc7b456fc9e32fc1601d007b075d34d3bb5795b6dc92` | 2022 |
| `cpu_aarch64_linux.c` | `2943796cf7dba39f43841674e43dc1af7efa30157081e6494d9daef269d17716` | 2016 |

Os trechos iniciais condicionam cada implementação a AArch64 e à plataforma
correspondente. A conferência cobre apenas os cabeçalhos indexados, não outros
avisos ou a proveniência integral. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação dessas
variantes ou inclusão em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: CMAC AWS-LC

Dois arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/cmac/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `cmac.c` | `e31536bc9041a2d780e7815772e3b4817b2f62e09c1787d001206933456dec9f` | OpenSSL Project, 2010; SPDX `Apache-2.0`. |
| `cmac_test.cc` | `45a9dd961263501cc56dea87288e82e07b06b94f9dfb41fc4c3cf35c0483dd4b` | Google Inc., 2015; SPDX `ISC`. |

Esta conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão do código ou teste em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: aritmética bignum AWS-LC

Seis arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/bn/` tiveram SHA-256 locais coincidentes com o índice.
Todos os cabeçalhos declaram SPDX `Apache-2.0`:

| Arquivo | SHA-256 | Crédito no cabeçalho |
|---|---|---|
| `bn.c` | `d1dcac7dbabd174fc6413a4b20de7169a4d8908ac67a2d745d71bcfd8c5dfae8` | Eric Young, 1995–1998. |
| `cmp.c` | `3b27a6b34055a693e30686b787b1aeac031a029480eb4ca6e286906557b6d8f8` | Eric Young, 1995–1998. |
| `ctx.c` | `108768c96480fd3b8ad85d08e8975f9dd6ba1f3d91756f9e717f82fd28487f46` | OpenSSL Project, 1998–2004; escrito por Ulf Moeller. |
| `div.c` | `3495bea2555d55146382be704a51d6f1c460ba9face738dfd3fac73a02402b17` | Eric Young, 1995–1998. |
| `gcd.c` | `7ebb35b113d29f804567efef43204725224c8d806b59fdab679eb1c9cb00691e` | Eric Young, 1995–1998, e OpenSSL Project, 1998–2001. |
| `mul.c` | `d3fcdd64dad4605092646bc372c041e39ab501a7120620b615f6254733aeb390` | Eric Young, 1995–1998. |

Esta conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: HKDF AWS-LC

Dois arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/hkdf/` tiveram SHA-256 locais coincidentes com o
índice. Seus cabeçalhos creditam Google Inc., 2014, e declaram SPDX `ISC`:

| Arquivo | SHA-256 |
|---|---|
| `hkdf.c` | `f47cf81d43a0097e4ffcb20408c48c66144e8501c55f8ee32b0199ca6a51f914` |
| `hkdf_test.cc` | `d5773633aa327ce15e8acacec5ca53ef12169c86378b07ca82e5e73ba5aa0bea` |

Esta conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão do código ou teste em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: Diffie-Hellman AWS-LC

O arquivo indexado `aws-lc-sys 0.47.0/aws-lc/crypto/fipsmodule/dh/dh.c`
teve SHA-256 local
`defa9f70178c84abff5b55b90bf101c7a9c67ab348d493077a4a7caa936e0a86`,
coincidente com o índice. Seu cabeçalho credita Eric Young, 1995–1998, e
declara SPDX `Apache-2.0`.

A conferência cobre somente o cabeçalho indexado, não todos os avisos ou a
proveniência integral do arquivo. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desse arquivo em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: ECDSA AWS-LC

Dois arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ecdsa/` tiveram SHA-256 locais coincidentes com o
índice. Seus cabeçalhos creditam OpenSSL Project, 1998–2005, e declaram SPDX
`Apache-2.0`:

| Arquivo | SHA-256 |
|---|---|
| `ecdsa.c` | `b04467a4d6e462ab52bc512b85aefffe2b8e7773f4352a3c005e94e407dfd8c0` |
| `ecdsa_test.cc` | `fae5c546ab4f7d1ad18802d0f2e0f7c9d7a08db9d1750f445e24a0380ce2f831` |

A conferência cobre somente os cabeçalhos indexados, não todos os avisos ou a
proveniência integral desses arquivos. O grafo Linux x86-64 já registrado para
a extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão do código ou teste em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: P-256 nistz AWS-LC

Dois arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ec/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `p256-nistz-table.h` | `6cef07bcefb38a051abaa8921fa6ea57e3be8d4e46e6473136d8cfbc0b25d8fe` | OpenSSL Project Authors, 2014–2016, e Intel, 2015; SPDX `Apache-2.0`; descreve tabela pré-computada para `p256-nistz.c`. |
| `p256-nistz.c` | `b0da7b17fb4b7d3c350e7aaaa4cdfb4158fcb0594b4ce9a1990833db76add43a` | OpenSSL Project Authors, 2014–2016, e Intel, 2014; nomeia Shay Gueron e Vlad Krasnov e cita artigo dos autores; SPDX `Apache-2.0`. |

A conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos; a tabela pré-computada não foi reproduzida.
O grafo Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona
`aws-lc-sys`, mas não prova compilação ou inclusão desses arquivos em binário.
A crate não é vendorizada na archive-fonte candidata. Notices por artefato
permanecem abertos; o Go não foi examinado nesta etapa.

### Triagem adicional: operações EC AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ec/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `ec_test.cc` | `0a99d7379467b33a50e5c0172fcac3f79091b7a8d2c3dfeb47a47fadeab7db8f` | Google Inc., 2014; SPDX `ISC`. |
| `felem.c` | `e204a41cc99b074acda85de261fb16e75b595611b336d2a3e945a4bc9ec47c52` | Google Inc., 2018; SPDX `ISC`. |
| `oct.c` | `9f53dd456f821da8a7cb8010b242f1b8c5663d19210165f3b7164b426b026b26` | OpenSSL Project e Sun Microsystems; atribui autoria original a Bodo Moeller e a aritmética polinomial binária a Sheueling Chang Shantz e Douglas Stebila; SPDX `Apache-2.0`. |

Esta conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: curvas elípticas AWS-LC

Cinco arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/ec/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `builtin_curves.h` | `7e3ed1af37fd1a1af1fd8890417efb78eec6ee7aef55bbe9bf2afeda32439c7b` | Google, 2023; SPDX `ISC`; declara geração por `make_tables.go`. |
| `ec.c` | `7a1787a45788ff7b41b045e364277024a8aa55afa53e2ea535ee78aad90a8a23` | OpenSSL Project e Sun Microsystems; escrito originalmente por Bodo Moeller, com atribuição específica da aritmética polinomial a Sheueling Chang Shantz e Douglas Stebila; SPDX `Apache-2.0`. |
| `ec_montgomery.c` | `ea61a6526abc1f1dc16dc5c2ae42f7059728d10f668a29881c891523af887f03` | OpenSSL Project e Sun Microsystems; atribui autoria original a Bodo Moeller e Nils Larsch e a aritmética polinomial aos mesmos autores acima; SPDX `Apache-2.0`. |
| `ec_nistp.c` | `4c6d3e938a8d418ce7392e203fc1331e9d408776f2684dc3e9e78a5545de922f` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`. |
| `ec_nistp.h` | `7d0c60ad4c42c83b3dc8fa18c6b96f5fb56622455cacdbc1ad91372dabf88fcf` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`. |

A conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos; a tabela gerada não foi reproduzida. O grafo
Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona `aws-lc-sys`,
mas não prova compilação ou inclusão desses arquivos em binário. A crate não é
vendorizada na archive-fonte candidata. Notices por artefato permanecem abertos;
o Go não foi examinado nesta etapa.

### Triagem adicional: RSA AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/rsa/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `blinding.c` | `c2e56aa0e49e2c1c690c3435a67cee7590c13674804b864d6a6d42f02e6c4796` | OpenSSL Project, 1998–2006, e Eric Young, 1995–1998; SPDX `Apache-2.0`. |
| `padding.c` | `39359d355230192058eddac2b0fcc8d0a03c7a3325cec243ea53d0813b0d419c` | OpenSSL Project, 2005; escrito por Stephen N Henson; SPDX `Apache-2.0`. |
| `rsa.c` | `39873ec6c8e0789bc35e8a6017fefb948f9daa9f021c1b13da3eb98a5cd6c0b4` | Eric Young, 1995–1998; SPDX `Apache-2.0`. |
| `rsa_impl.c` | `a123af42f18090e89d3f0c5699c88b4e2ddd1e2bde8a3376e097cf824a9b323d` | Eric Young, 1995–1998; SPDX `Apache-2.0`. |

A conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: testes rand AWS-LC

Dois arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/rand/` tiveram SHA-256 locais coincidentes com o índice.
Seus cabeçalhos creditam Amazon.com Inc. ou afiliadas e declaram SPDX
`Apache-2.0 OR ISC`:

| Arquivo | SHA-256 |
|---|---|
| `rand_isolated_test.cc` | `ef58f8c8b9e53fc05fa738ea1e1c0890d3325608ffab1223888a16d58ee93615` |
| `rand_test.cc` | `70840f9fdf421d3a06ab011631c078e8b15975a4bfe581bc9e41ef9701eefab8` |

A conferência cobre somente os cabeçalhos indexados, não todos os avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses testes em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: fontes de entropia AWS-LC

Cinco arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/rand/entropy/` tiveram SHA-256 locais coincidentes com
o índice. Seus cabeçalhos creditam Amazon.com Inc. ou afiliadas e declaram SPDX
`Apache-2.0 OR ISC`:

| Arquivo | SHA-256 |
|---|---|
| `entropy_source_test.cc` | `49b0f5ab7faf07f70a5b7b511601f776eb29875dca188bb0c2e4b63c7e9bf4b5` |
| `entropy_sources.c` | `eaa890ca6085f9de2a54d1792683c2ffc90422d6a78a19316220a96e3aa6e454` |
| `internal.h` | `5e09a7151ba63077fdfcecb9e90b30c4bdbd9eb268557eb75163ce69202a821e` |
| `tree_drbg_jitter_entropy.c` | `d570423831ee75597b5325befb6f7c6df5fc2ad93ae6017addd4719733c307eb` |
| `tree_drbg_jitter_entropy_isolated_test.cc` | `f079873c2753a6758fb390226b18fe6bac29e945444eb3a525dfbcf8f1cd7d2e` |

A conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência integral desses arquivos. Os avisos de jitterentropy de terceiros
foram examinados separadamente em `s3-aws-lc-transitive-notices.md`. O grafo
Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona `aws-lc-sys`,
mas não prova compilação destes arquivos ou inclusão dos testes em binário. A
crate não é vendorizada na archive-fonte candidata. Notices por artefato seguem
abertos; o candidato Go não foi examinado nesta etapa.

### Triagem adicional: geração aleatória AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/rand/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `cpu_jitter_test.cc` | `f9f309a7cb93a6775b91e1c76066d19d650209621c073244bf024e359bdeb7fe` | Amazon ou afiliadas; SPDX `Apache-2.0 OR ISC`. |
| `ctrdrbg.c` | `a33b6c853c5d6279e7421409df585c9b93787a84f45df5aa30c86ad88305c69c` | Google Inc., 2017; SPDX `ISC`. |
| `ctrdrbg_test.cc` | `29072dc7ac9441b8312e200c306a842a4520370e9183d76cbe6d10a37b9460a3` | Google Inc., 2017; SPDX `ISC`. |

Esta conferência cobre apenas os cabeçalhos indexados, não outros avisos ou a
proveniência integral desses arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação dos
arquivos ou inclusão dos testes em binário. A crate não é vendorizada na
archive-fonte candidata. Notices por artefato permanecem abertos; o Go não foi
examinado nesta etapa.

### Triagem adicional: HMAC AWS-LC

O arquivo indexado `aws-lc-sys 0.47.0/aws-lc/crypto/fipsmodule/hmac/hmac.c`
teve SHA-256 local
`014b175870b63c31ab0ecf956f2e743a4725530d8ce0e77e87829f4426654265`,
coincidente com o índice. Seu cabeçalho credita Eric Young, 1995–1998, e
declara SPDX `Apache-2.0`.

A conferência cobre somente o cabeçalho indexado, não todos os avisos ou a
proveniência integral do arquivo. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desse arquivo em binário. A crate não é vendorizada na archive-fonte
candidata. Os notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: digest AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/digest/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `digest.c` | `6a962df0d88d4156cafd0c56ea875e67f8d67d3db4985d96fc0fc9c341d2daa5` | Eric Young, 1995–1998; SPDX `Apache-2.0`. |
| `digests.c` | `3196814e58e9fb9d5f67f7b7af5158d57d4d0f287800bf1aed1521454b62e21d` | Eric Young, 1995–1998; SPDX `Apache-2.0`. |
| `md32_common.h` | `c824291015b560de254a859553a3806298af94e1301d6909c27243d312f796c6` | OpenSSL Project, 1999–2007; SPDX `Apache-2.0`. |

A conferência cobre somente os cabeçalhos indexados, não todos os avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Os notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: interface de cifras AWS-LC

Dois arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/cipher/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `aead.c` | `7c800d9241cc036ba699606625e39000c02c12debdc6c4deadfb2776568ee097` | Google Inc., 2014; SPDX `ISC`. |
| `internal.h` | `f51607d8488ec22ead879c951315b56d35d85741b78f3a41325246904c9b0157` | Eric Young, 1995–1998; SPDX `Apache-2.0`. |

Esta conferência cobre apenas os cabeçalhos indexados desses arquivos, não
outros avisos ou sua proveniência integral. O grafo Linux x86-64 já registrado
para a extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação
ou inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: cifras AES AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/cipher/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `cipher.c` | `b40e79909b53a7e0eb6533e73ac328d099699fa4ea26fc18a859c6000b2754e7` | Eric Young, 1995–1998; SPDX `Apache-2.0`. |
| `e_aes.c` | `79c53148604509bbc7d0b6ff9b49e1ad04b981997b8a59cfce5f0d8992fddf8d` | OpenSSL Project, 2001–2011; SPDX `Apache-2.0`. |
| `e_aesccm.c` | `a5a114ebd78412f842ec18395f5311b59f5533674c6bcdf6725e9567e6aaa258` | OpenSSL Project, 2008; SPDX `Apache-2.0`. |

A conferência cobre os cabeçalhos indexados, não todos os avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Os notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: aceleração e testes bignum AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/bn/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `asm/x86_64-gcc.c` | `74ce827415fcb770c9cb9d3309d207c9b97216900d746e26b49cc0246bd02fa7` | Implementação atribuída a Andy Polyakov para OpenSSL; SPDX `Apache-2.0`. |
| `bn_test.cc` | `cc20f4ebb235129b2edefe0b8c9cd00a56a5140f511c5dcbc357c8b913d32458` | Copyright Eric Young, 1995–1998, e Sun Microsystems, 2002; atribui a aritmética polinomial binária a Sheueling Chang Shantz e Douglas Stebila; SPDX `Apache-2.0`. |
| `rsaz_exp.c` | `4fce3109fc34aff004afa33caf1706d74e1a4792a496b08e9f78e5a03d01644c` | Copyright OpenSSL Project Authors, 2013–2016, e Intel, 2012; nomeia Shay Gueron e Vlad Krasnov; SPDX `Apache-2.0`. |

A conferência cobre os cabeçalhos indexados, não outros avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Os notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: operações bignum AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/bn/` tiveram SHA-256 locais coincidentes com o índice.
Os três cabeçalhos creditam Eric Young, 1995–1998, e declaram SPDX
`Apache-2.0`:

| Arquivo | SHA-256 |
|---|---|
| `bytes.c` | `dc96b981cd49f7cbfd7af490c00442b7416956b2a60db1e7458f2ab698595e32` |
| `generic.c` | `6bb830f2bcb207fbf3dc81605766618a581307fbe310725ca2041fa82798d39c` |
| `shift.c` | `95a250bab4fed4e3380381765deac07cc47869aae926091a3ca9b7745fa742d9` |

A conferência cobre os cabeçalhos indexados, não todos os avisos ou a
proveniência integral dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
inclusão desses arquivos em binário. A crate não é vendorizada na archive-fonte
candidata. Os notices por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: outras fontes bignum AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/bn/` tiveram SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Cabeçalho examinado |
|---|---|---|
| `prime.c` | `4bc6d5f3cf12119920118713fc835953922609df75c0e596dcaa6b101acdb198` | Eric Young, 1995–1998, e OpenSSL Project, 1998–2001; SPDX `Apache-2.0`. |
| `random.c` | `37291038bdc50a6c38f795a5e52699fcc4a3f77a94cdbac5df845961344feff9` | Mesmo copyright e SPDX de `prime.c`. |
| `rsaz_exp.h` | `249ad96c47dfb038a7e4b81a1d9565ca15893cc66aa21a8ae4c309cc48f459d3` | OpenSSL Project Authors, 2013–2016, e Intel, 2012; nomeia Shay Gueron e Vlad Krasnov; SPDX `Apache-2.0`. |

A análise abrange os cabeçalhos indexados, não todos os avisos ou a proveniência
integral dos arquivos. O grafo Linux x86-64 anteriormente registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, sem provar que esses arquivos
foram compilados ou incluídos em binário. A crate não é vendorizada na
archive-fonte candidata. Os notices por artefato permanecem abertos; o candidato
Go não foi examinado nesta etapa.

### Triagem adicional: bignum AWS-LC

Cinco arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/bn/` tiveram SHA-256 locais coincidentes:

| Arquivo | SHA-256 | Cabeçalho |
|---|---|---|
| `bn_assert_test.cc` | `00595af8ca375ba4b692676da7bc0b6677667006a278dc21b13126a3ff1efeb3` | Amazon ou afiliadas; `Apache-2.0 OR ISC`. |
| `div_extra.c` | `5854f58df73ef3731a97f578a2983c2f8c72fc03870ad9edea910a5bb8ad9008` | Google, 2018; `ISC`. |
| `exponentiation.c` | `a858ca843eeabed2102e8686cc55aaffb6854176eb39f5322cd8d0278434a86e` | Eric Young, 1995–1998, e OpenSSL Project, 1998–2005; `Apache-2.0`. |
| `montgomery.c` | `073cba671b1c22ebf57dea832835a4b9e179a153326de60d8a2a0ab05753a1fc` | Eric Young, 1995–1998, e OpenSSL Project, 1998–2006; `Apache-2.0`. |
| `rsaz_exp_x2.c` | `129a3e482b7cd8ad43108619a45d39f58b119627fe65222c27489a3c17c3e71b` | OpenSSL Project Authors e Intel, 2020–2021; nomeia Sergey Kirillov e Andrey Matyukov e agradece Ilya Albrekht; `Apache-2.0`. |

A conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência completa dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou
ligação destes arquivos. A crate não é vendorizada na archive-fonte candidata;
notices por artefato permanecem abertos. O Go não foi examinado nesta etapa.

### Triagem adicional: fontes AES AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/fipsmodule/aes/` tiveram SHA-256 locais coincidentes:

| Arquivo | SHA-256 | Cabeçalho |
|---|---|---|
| `aes_nohw.c` | `510fc98fa0b00a81155271a88f1266053d42bcbc81790fb4b150bcd54b08ecd5` | Google Inc., 2019; SPDX `ISC`. |
| `aes_test.cc` | `2a2edc7de944688bb83cc71f6617212ebd28f1106fa258892501de0538d3e4ca` | Google Inc., 2015; SPDX `ISC`. |
| `key_wrap.c` | `9a61ee985baf091f886fedf3a301d5585dffae5e406c79e2f723a8a8335356a7` | The OpenSSL Project, 2001–2011; SPDX `Apache-2.0`. |
| `mode_wrappers.c` | `affd4248af4f78fc4d5d54e71173efce9f11cfba170980951bd73e38a2c3f9cf` | The OpenSSL Project, 2002–2006; SPDX `Apache-2.0`. |

A conferência cobre somente os cabeçalhos indexados, não outros avisos ou a
proveniência completa dos arquivos. O grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação desses
arquivos ou inclusão do teste em binário. A crate não é vendorizada na
archive-fonte candidata. Notices por artefato permanecem abertos; o candidato Go
não foi examinado nesta etapa.

### Triagem adicional: outros utilitários de teste AWS-LC

Os 13 arquivos indexados adicionais de `aws-lc-sys 0.47.0` sob
`aws-lc/crypto/test/` tiveram hashes locais coincidentes. Os dez primeiros da
tabela creditam Google Inc. e declaram ISC; os três últimos creditam Amazon.com,
Inc. ou afiliadas e declaram `Apache-2.0 OR ISC`:

| Arquivo | SHA-256 |
|---|---|
| `file_test_gtest.cc` | `c6e49e2c5fafd84fb6300ae0fdf6e5fe43919ec171b6440511734e7b8cdff01b` |
| `file_util.cc` | `c29dc1aaafc79b10a45f0b959d1e3dc5c965ceefd927cb3081a114c48873906c` |
| `file_util.h` | `05cf9b6e7c9171c318597d9e00a13079452abe91faff842fce4ca96f4428d1dd` |
| `gtest_main.cc` | `97a5d8a8fcabb70c1c99e002bd5b71da02de83da205d499bf577dac449abd152` |
| `gtest_main.h` | `238440bb87b1bcf099fcfa9d20f4800e62136bcd028dfdec715c3faa1a6e1b34` |
| `malloc.cc` | `419b1853f1a48a3631e4a7ecf2844a00aeaa2332ee5d3fd11a2bb893ecb9ca16` |
| `test_util.cc` | `1e6e7d8324b5ca8e55d3373d8b3e80f5b2e1fcd75324046a7c4c2c299317130a` |
| `test_util.h` | `e133a20fd4298ee04bf625d0058a1f98f723871c5264552428d3ea3c052d3594` |
| `wycheproof_util.cc` | `4563a5dc9029c9c417e3c49dfaa78ba449727858f02b79cf7853f2b66dfeec0a` |
| `wycheproof_util.h` | `c2df45d39c67f06e25b36f2a2adb67479c3b8d46ef62f5fe542f4899c1035847` |
| `ube_test.h` | `4b9347531824af804b1f32b719395a519b649902ea3eaba4ef92ea6d4f6be6e1` |
| `x509_util.cc` | `448df20216435df5d6680213708b014f6f023c84579f59ac6868bf136c8f326f` |
| `x509_util.h` | `9ad60efbe5ddef0d257c190403fe5252cfd2c1709cc89160fa5ded440f3e97a2` |

A triagem cobre somente os cabeçalhos indexados, não outros avisos, fontes ou
dados usados pelos testes. O grafo Linux x86-64 já registrado para a extração
Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova compilação ou inclusão
desses utilitários em binário. A crate não é vendorizada na archive-fonte
candidata. Notices por artefato continuam abertos; o Go não foi examinado.

### Triagem adicional: infraestrutura de testes AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` sob `aws-lc/crypto/test/`
tiveram SHA-256 locais coincidentes. Seus cabeçalhos creditam Google Inc. e
declaram `SPDX-License-Identifier: ISC`:

| Arquivo | SHA-256 | Ano no cabeçalho |
|---|---|---|
| `abi_test.cc` | `4f67d85526e3788deae1f79ffef6a89081fccbb713af0a14ff838341d6405751` | 2018 |
| `abi_test.h` | `ade45e64ab1b6ebe3bd2a88310b5599538380bde1bcf4229cb8ce40e5c5e5e13` | 2018 |
| `file_test.cc` | `507a0821d29d0a8a3211cf4c525551a1d9d2b6197e186ce9bb1c6ccb0c4fabbe` | 2015 |
| `file_test.h` | `eb0f749510adb6ecaadb1a26722dd6ee60c65b2c406c3aaa29175bb2057c6c31` | 2015 |

Essa conferência cobre somente os cabeçalhos indexados desses quatro arquivos,
não todos os avisos ou a proveniência de seu conteúdo. O grafo Linux x86-64 já
registrado para a extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas não prova
compilação ou inclusão dessa infraestrutura de testes em binário. A crate não é
vendorizada na archive-fonte candidata; notices por artefato permanecem abertos
e o candidato Go não foi examinado nesta etapa.

### Triagem adicional: seleção de biblioteca de sistema AWS-LC

Dois arquivos indexados de `aws-lc-sys 0.47.0` sob `builder/` tiveram SHA-256
locais coincidentes:

| Arquivo | SHA-256 | Contexto das ocorrências `origin` |
|---|---|---|
| `system_detect_tests.rs` | `1d77d8ae664232ca6c62aa7e1fa8814bca0e1bb93ef5cb6f9fc5a4e4b0ad7567` | Linha 233 descreve um caminho de inclusão derivado de uma configuração de diretório. |
| `system_library.rs` | `81b5a5aca6d726a23517c81dd9ba9c64e59633ee48647b6a8abaa826288dcf1e` | Linha 71 descreve um prefixo derivado de caminho; linha 456 diferencia versões mínimas declaradas de versões derivadas do submódulo. |

Esses trechos não atribuem origem ou cópia de código; a classificação cobre
somente as ocorrências indexadas e não esgota os avisos dos arquivos ou da crate.
O grafo Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona
`aws-lc-sys`, mas não comprova uso desses caminhos de configuração ou inclusão
em binário. A crate não é vendorizada na archive-fonte candidata; os notices por
artefato permanecem abertos e o Go não foi examinado nesta etapa.

### Triagem adicional: outros s2n-bignum ML-DSA AVX2

Seis arquivos adicionais de `aws-lc-sys 0.47.0` sob
`aws-lc/third_party/s2n-bignum/s2n-bignum-imported/x86_att/mldsa/` tiveram
hashes locais coincidentes com o índice. Todos declaram SPDX
`Apache-2.0 OR ISC OR MIT` e creditam Amazon.com, Inc. ou afiliadas; os dois
arquivos `eta` também creditam os autores de mldsa-native:

| Arquivo | SHA-256 |
|---|---|
| `mldsa_decompose_88.S` | `72c2c41d6e457d2f085c869c4e2170ddb956ebb38cfeaa83b9f851d20da2f52b` |
| `mldsa_poly_use_hint_88.S` | `ecc34b0ae85ea4f8546b7373991dae931b9c2123028aa57a2883f2d553c40c53` |
| `mldsa_polyz_unpack_19.S` | `7f330c24825335a10ce05b5cd0b9cb12699cd413d64b9cf386bb6fc11cffff05` |
| `mldsa_rej_uniform_VARIABLE_TIME.S` | `add31407de84c74387499a17437c7e560cc59297d5ee3b67495317981502650a` |
| `mldsa_rej_uniform_eta2_VARIABLE_TIME.S` | `1d2bc8f3e3e9d8abe8dd65f4fea6b7af7b20514c2f40b1f55308888bf138f3ef` |
| `mldsa_rej_uniform_eta4_VARIABLE_TIME.S` | `10d99a7710b6688322709ccc2acdda3db25aa5a0dde8cc0a6fc0a4062bb41459` |

Os comentários atribuem as implementações a Dilithium AVX2, descrita como
domínio público; cinco mencionam `pq-crystals/dilithium` via mldsa-native e um
nomeia autores da implementação otimizada. Não fixam revisão nos trechos
indexados; não houve comparação integral ou determinação dos avisos por
artefato. A revisão geral dos imports consta em `s3-aws-lc-transitive-notices.md`.
O grafo Linux x86-64 da extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas
não prova compilação ou ligação desses objetos; a crate não é vendorizada na
archive-fonte candidata. O Go não foi examinado nesta etapa.

### Triagem adicional: s2n-bignum ML-DSA AVX2

Quatro arquivos adicionais de `aws-lc-sys 0.47.0` sob
`aws-lc/third_party/s2n-bignum/s2n-bignum-imported/x86_att/mldsa/` tiveram
hashes locais coincidentes com o índice. Seus cabeçalhos creditam Amazon.com,
Inc. ou afiliadas e declaram `Apache-2.0 OR ISC OR MIT`:

| Arquivo | SHA-256 |
|---|---|
| `mldsa_decompose_32.S` | `fe326294de644ccdc1b5d07f3ba56add9af94f39201ac639139f960d7934b26a` |
| `mldsa_nttunpack.S` | `167691da1c6e2318f29440e1db8d4139919828820c525476fdc044f279fad382` |
| `mldsa_poly_use_hint_32.S` | `c806b181b51851a309c945b66cc2f1f4796300402d22d6907be6f40c1585a0e0` |
| `mldsa_polyz_unpack_17.S` | `ff5a205c10ac51c09ba7540c15a17661de8baa061b265727997524088ccfad13` |

Os comentários atribuem as implementações a Dilithium AVX2, descrita como
domínio público. Três mencionam `pq-crystals/dilithium` via mldsa-native;
`mldsa_nttunpack.S` nomeia também os autores da implementação otimizada. Os
trechos examinados não fixam revisões; não foi feita comparação integral nem
determinada a aplicabilidade dos avisos por artefato. A revisão geral dos
imports consta em `s3-aws-lc-transitive-notices.md`; estes quatro arquivos são
distintos de `mldsa_caddq.S` e `mldsa_chknorm.S` já classificados abaixo.
O grafo Linux x86-64 da extração Rust `8avAZthl` seleciona `aws-lc-sys`, mas
não prova compilação ou ligação desses objetos; a crate não é vendorizada na
archive-fonte candidata. O candidato Go não foi examinado nesta etapa.

### Triagem adicional: s2n-bignum X25519 e ML-DSA

Quatro arquivos de `aws-lc-sys 0.47.0` sob
`aws-lc/third_party/s2n-bignum/s2n-bignum-imported/` tiveram hashes locais
coincidentes com o índice:

| Arquivo | SHA-256 | Referência examinada |
|---|---|---|
| `arm/curve25519/curve25519_x25519.S` | `ad7bff4496bf394c3612757d22dc839cde861265a702c9c246fc8ea870b55cba` | Atribui derivação substancial à implementação X25519-AArch64 de Emil Lenngren e a uma reelaboração baseada em SLOTHY; cabeçalho `Apache-2.0 OR ISC OR MIT-0`. |
| `arm/curve25519/curve25519_x25519_byte.S` | `c99e77052afa785252e5364db4235f89b574610989ab8299aa21b7aca2bc0fdf` | Mesma atribuição e expressão SPDX. |
| `x86_att/mldsa/mldsa_caddq.S` | `7dc4f5328cf5d6d53443647257d0bdb27c1d7d1b4325adeb7e3bea81cd9c9ee2` | Atribui derivação à implementação AVX2 Dilithium descrita como domínio público, via mldsa-native; cabeçalho `Apache-2.0 OR ISC OR MIT`. |
| `x86_att/mldsa/mldsa_chknorm.S` | `697d7bce281a6b88688ff8392759e08cef87d138c446bc0924810c9a8695030f` | Mesma atribuição e expressão SPDX. |

Os quatro cabeçalhos também creditam Amazon.com, Inc. ou afiliadas. As
referências não fixam as revisões originais nos trechos indexados; não foi feita
comparação integral nem determinada a aplicabilidade dos avisos às saídas. A
revisão geral dos imports ML-DSA e s2n-bignum consta em
`s3-aws-lc-transitive-notices.md`; esta triagem cobre estes quatro arquivos.
O grafo Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona
`aws-lc-sys`, mas não prova compilação ou ligação desses objetos. A crate não é
vendorizada na archive-fonte candidata; o Go não foi examinado nesta etapa.

### Triagem adicional: headers jitterentropy AWS-LC

Sete headers indexados de `aws-lc-sys 0.47.0`, sob
`aws-lc/third_party/jitterentropy/jitterentropy-library/`, tiveram hashes locais
coincidentes. Em cada um, a ocorrência `origin` da linha 19 integra a cláusula
de não endosso do aviso de licença, não atribuição de origem de código:

| Arquivo | SHA-256 |
|---|---|
| `arch/jitterentropy-base-power.h` | `0c19740d18aebd7cfbc1e79a6aafcd09d0b81dcc79307b8d3dbcb43a6da55e46` |
| `arch/jitterentropy-base-s390.h` | `f8c37f8d04662f370559abb10e57faa5d90ae25b4f17aa3b8d7dc45815d54af3` |
| `arch/jitterentropy-base-windows.h` | `257bdb22ae1ac823b2e4c5c7990a0f50626b7fe15bac380a7d596d1b049592e8` |
| `arch/jitterentropy-base-x86.h` | `1702f78435d9dffc86053516715f67b3165e5387d9a7e0835c89ac5d7137a2a0` |
| `jitterentropy-base-user.h` | `884102244d27c0a313aad3c77e94a01a04cd2dc26c4254227014d6ca11804896` |
| `jitterentropy.h` | `064ef5cf5d8920d538f9b2afbfadc14f388077a8f369bf551efa9fdceaa04f2e` |
| `src/jitterentropy-internal.h` | `20e42748ed6746c1fa5c64aaf1764cdeec7845100cceff1b09a1097737abbbcf` |

Os cabeçalhos creditam Stephan Mueller; cinco indicam 2013–2025 e os dois
últimos 2014–2025. O cabeçalho x86 examinado contém condições BSD de três
cláusulas, alternativa GPL e disclaimer. A revisão anterior do `LICENSE`
upstream e os limites de composição estão em `s3-aws-lc-transitive-notices.md`;
esta triagem não seleciona licença nem determina obrigações por artefato.
O grafo Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona
`aws-lc-sys`, mas não prova inclusão desses headers específicos em binário;
a crate não é vendorizada na archive-fonte candidata. O Go não foi examinado.

### Triagem adicional: GHASH AArch64 gerado

Três arquivos `aws-lc-sys 0.47.0` sob `aws-lc/generated-src/`, com ocorrência
`origin` em `crypto/fipsmodule/ghash-neon-armv8.S`, tiveram hashes locais
coincidentes com o índice:

| Variante | SHA-256 |
|---|---|
| `ios-aarch64` | `942c484d99c7e2069efd1a9391dd26953157e0265da21298b547dec6cac83522` |
| `linux-aarch64` | `52a3c4ca74c0a447b901c84bcc221ae22a6e4ba38910e30bdc74ae70b63b6e1e` |
| `win-aarch64` | `8db89c7085f0d83aced19e31e3e274c816eafcae2210f945d1273c54b36caf67` |

O comentário indexado afirma que `gcm_init_neon` é adaptada de `gcm_init_v8`;
os cabeçalhos consultados nas variantes Linux e Windows dizem que o arquivo é
gerado de um script Perl de nome semelhante na árvore BoringSSL. Os comentários
não fixam revisão ou licença do script. Não foram reconstruídas as saídas, nem
identificados a revisão efetivamente usada ou os notices aplicáveis; a triagem
cobre apenas essas ocorrências indexadas.

O grafo Linux x86-64 já registrado para a extração Rust `8avAZthl` seleciona
`aws-lc-sys` via `aws-lc-rs`, mas não demonstra compilação das variantes
AArch64 ou presença em binário. A crate não é vendorizada na archive-fonte
candidata. A revisão por artefato permanece aberta; o Go não foi examinado.

Investigação adicional do gerador: a crate no cache contém
`aws-lc/crypto/fipsmodule/modes/asm/ghash-neon-armv8.pl`, cujo SHA-256 é
`88fac2af25f80d7e64a686e096ff1f2d69a115c06f66112eff7f46dc8edecd8a`.
Os bytes coincidiram com o [arquivo AWS-LC no commit
`cd481c207e38f06aeb5766f4a1cdfce10a4c0af8`](https://github.com/aws/aws-lc/blob/cd481c207e38f06aeb5766f4a1cdfce10a4c0af8/crypto/fipsmodule/modes/asm/ghash-neon-armv8.pl)
(HTTP 200). O script contém o mesmo comentário sobre `gcm_init_v8`, credita
copyright 2010–2016 The OpenSSL Project Authors, nomeia Andy Polyakov e declara
SPDX `Apache-2.0`. Seu texto descreve adaptação AArch64 de `ghash-armv4.pl` e
cita um artigo sobre multiplicação polinomial. O `ghash-armv4.pl` local também
declara Apache-2.0 e credita os autores do OpenSSL, com anos 2010–2018.

A coincidência do script local com esse commit e o comentário comum não provam
que ele e seus insumos produziram byte a byte as três saídas; não se executou o
gerador, nem se estabeleceu a revisão anterior de origem ou a aplicabilidade dos
avisos ao artefato. A revisão por composição e os limites AArch64 acima permanecem.

### Triagem adicional: geração ML-DSA e auxiliares AWS-LC

Três arquivos indexados de `aws-lc-sys 0.47.0` tiveram SHA-256 locais
coincidentes:

| Arquivo | SHA-256 | Contexto da ocorrência |
|---|---|---|
| `aws-lc/crypto/fipsmodule/ml_dsa/mldsa/native/aarch64/src/rej_uniform_eta_table.c` | `282765dc029fec5444b93978959a582aaa3cf1bb86ebacbaf7ccd29a71397351` | Linha 22 atribui adaptação da tabela de ML-KEM para ML-DSA e remete a `autogen`. O cabeçalho credita autores de mldsa-native, declara `Apache-2.0 OR ISC OR MIT` e diz que o arquivo é gerado; não fixa a revisão da fonte adaptada. |
| `aws-lc/crypto/fipsmodule/self_check/self_check.c` | `27fc18e9ff4ce063fe3b94474d62de6b1b989e56953e044f08f3c5755fbb5548` | Linha 2959 descreve vetor de teste derivado de exemplo da RFC 8448; a referência não determina os avisos aplicáveis. |
| `builder/system_detect.rs` | `bbf27449fae5d448839a8b61578e3c38b6fe6742211328f6c8320d292b3aabb5` | Linha 102 descreve prefixo derivado de caminho de inclusão, não origem de código. |

A revisão cobre apenas as ocorrências citadas e não comprova reprodução do
arquivo gerado ou licenças aplicáveis a artefatos. Conforme o grafo Linux
x86-64 já registrado para a extração Rust `8avAZthl`, `aws-lc-sys` é selecionada
via `aws-lc-rs`; isso não prova compilação de qualquer trecho, especialmente o
backend AArch64, ou inclusão em binário. A crate não é vendorizada na
archive-fonte candidata; o Go não foi examinado nesta etapa.

### Triagem adicional: KEM e SSL AWS-LC

Quatro arquivos indexados de `aws-lc-sys 0.47.0` tiveram SHA-256 locais
coincidentes. As ocorrências `origin` examinadas descrevem valores, testes ou
um comentário copiado, sem atribuição identificada a fonte externa:

| Arquivo sob `aws-lc/` | SHA-256 | Contexto |
|---|---|---|
| `include/openssl/experimental/kem_deterministic_api.h` | `dbee5c4a86125205351dd44f483e1e9c00c96b06a7eb7b1cc41ad3790f55334e` | Linha 39: segredo derivado de uma seed. |
| `include/openssl/trust_token.h` | `d9fcaa4635e759d465a3bde203005026aad68589593640890c85a88195b6b616` | Linhas 144 e 270: token derivado de uma mensagem. |
| `ssl/ssl_misc_test.cc` | `17576d68fdcb77216df06dc62a4dd8fa82daeb3ca7debdcd28eeccf8e61215d9` | Linha 1891: testes de regressão derivados de um blob de teste identificado localmente. |
| `ssl/ssl_transfer_asn1.cc` | `64d3737fabb5d53a1ccdd7a3a7cc382d088765cc7c06b488baeafa0fc4e1f6bc` | Linha 790: comentário copiado de `SSL_do_handshake`, sem fixar revisão ou proveniência anterior. |

Essa classificação cobre apenas as ocorrências indicadas, não todos os avisos
dos arquivos ou da crate. Conforme o grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl`, `aws-lc-sys` é selecionada via `aws-lc-rs`; isso não
prova compilação dos arquivos ou sua inclusão em binário. A crate não é
vendorizada na archive-fonte candidata. Os notices por artefato permanecem
abertos; o Go não foi examinado nesta etapa.

### Triagem adicional: headers de API AWS-LC

Quatro headers indexados de `aws-lc-sys 0.47.0` sob
`aws-lc/include/openssl/` tiveram SHA-256 locais coincidentes. As ocorrências
`origin` examinadas descrevem operações sobre dados ou tipos, não atribuições
de origem de código:

| Arquivo | SHA-256 | Contexto |
|---|---|---|
| `digest.h` | `5f822fc5dc71f9c440e272ea1cb77b3a8cc198d3482c1d936ba7ff72981fa573` | Linha 305: membro copiado de outra estrutura. |
| `span.h` | `fb009d1ba717800941aa7612488e145ffae05b062e1f32ecfbe44ebed094a94a` | Linha 29: relação de herança de tipo. |
| `ssl.h` | `8ca1595cffed7fada5073c8afb17a433e0f98b06a4ca8727c3d538d558544971` | Linhas 1961 e 4744: valor derivado de segredo e informação derivada de handshake. |
| `x509.h` | `eb26792b2c68236fe6cfb3a207a0e150a0b7ebb2abf7afc49e4d77dffd6fcc4e` | Linha 4604: tipo ASN.1 copiado de outro identificador. |

A classificação cobre somente as ocorrências indicadas; não esgota outros
avisos dos headers ou da crate. O grafo Linux x86-64 da extração Rust
`8avAZthl`, já registrado para `aws-lc-sys`, seleciona a crate via `aws-lc-rs`,
mas não prova inclusão desses arquivos em binário. A crate não é vendorizada
na archive-fonte candidata. Os notices por artefato permanecem abertos; o
candidato Go não foi examinado nesta etapa.

### Triagem adicional: pin-project-lite

Os 40 arquivos indexados de `pin-project-lite 0.2.17` tiveram SHA-256 locais
coincidentes com os respectivos registros em `rust-embedded-notice-index.json`:
um arquivo `src/lib.rs` e 39 arquivos sob `tests/`. Todas as linhas indexadas
declaram `SPDX-License-Identifier: Apache-2.0 OR MIT`. Por exemplo, o SHA-256
de `src/lib.rs` é
`09a84ff233021dabb75200ef71ebcb1c5a47c94484ce294ef5dcf43797873ba4`;
os caminhos e hashes individuais dos 39 arquivos de teste permanecem no índice
preservado. A conferência cobre essas linhas indexadas, não todos os avisos ou
textos de licença da crate, e não determina obrigações por artefato.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i pin-project-lite --depth 1`
selecionou `pin-project-lite 0.2.17` via `tokio`, `hyper`, `aws-smithy-async`
e outras dependências. O grafo não prova inclusão dos arquivos ou testes em
binário; a crate não é vendorizada na archive-fonte candidata. O candidato Go
não foi examinado nesta etapa; notices por artefato permanecem abertos.

### Triagem adicional: aws-smithy-async

Os 17 arquivos indexados de `aws-smithy-async 1.3.1` tiveram SHA-256 locais
coincidentes. Seus cabeçalhos creditam Amazon.com, Inc. ou afiliadas e declaram
`SPDX-License-Identifier: Apache-2.0`:

| Arquivo sob `src/` | SHA-256 |
|---|---|
| `future/mod.rs` | `1c66a59c19f2203399814a352698b35f0e3d864c32ec153f8772152539a09b0c` |
| `future/never.rs` | `b38adbd7983a5dc28153fbe841722e935209752c8843bffbf4a78c29178f7df5` |
| `future/now_or_later.rs` | `5c98a374fdc23f6ebb9d50f177f4d6465f3b72f63e783a3e47c17e5cc170cc45` |
| `future/pagination_stream/collect.rs` | `08d0369671707390095b171c704ca87010f64d929d43136333cdeb4462326148` |
| `future/pagination_stream/fn_stream.rs` | `82e02981f50042d724bebfe9094d270df04e641c6a57a91d3be5962c521a743d` |
| `future/pagination_stream.rs` | `121c0a55ed18cb170ab98d8d1473d4680218290fb73b59fbfdefe1266aaff957` |
| `future/rendezvous.rs` | `5fd2c05c6cec17e3b6cf2a89fcc2d13b3687292cfe104618d827076327d5106d` |
| `future/timeout.rs` | `bd2c19b8ce28b50baf24cfb33e9c21fc9af210fa0d9a2fefaf0854d0be52a5e2` |
| `lib.rs` | `55674e3457d44ee436e32c4e8942972a470dfa9db0c14a961995ddd27a377dac` |
| `rt/mod.rs` | `730aad2a29f2b23f47bd0f606b46820ba04399610dbeb2bb756c4cf6feac6980` |
| `rt/sleep.rs` | `cb15d481b9f29f9c00ad0198ccd021f15185d64c77edb956b9b0663b1747b31c` |
| `test_util/controlled_sleep.rs` | `9ae0e61084c478c6c0e97a39513b64369e709209c48f349f66acca5223a1e3f8` |
| `test_util/instant_sleep.rs` | `3e2d9f48232d08d29549ddc9927f07279d04525ea33173b16ced3b294a3a1aae` |
| `test_util/manual_time.rs` | `8e8eb835524a743171fb9274a336451b9e4ced39026c1dc7bd4886c6b919a0ce` |
| `test_util/tick_advance_sleep.rs` | `43bb97296f19807fb6431427e9e932d08e176b9b50bb840304a6b834a42ac980` |
| `test_util.rs` | `ed17989249727d850d1741a060a3f78571a1896c153b92a59adc1684ee3b55c0` |
| `time.rs` | `a0abe589ca73514ef646d45010cca5e82b8ac303c5caa454bfddd75ae036a4af` |

Além do cabeçalho Apache, `future/timeout.rs` afirma que seu código foi copiado
e modificado de Tokio e preserva um aviso MIT com copyright 2021 Tokio
Contributors, condição de inclusão do aviso e disclaimer. Não identifica a
revisão original. Esse aviso deve ser considerado separadamente do SPDX do
arquivo; sua aplicabilidade depende dos materiais efetivamente distribuídos.
A triagem cobre os trechos indexados, não todos os avisos ou notices upstream.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i aws-smithy-async --depth 2`
selecionou a crate via `aws-sdk-s3`, `aws-runtime` e outras dependências,
incluindo arestas de desenvolvimento. O grafo não prova inclusão dos arquivos
em binário; a crate não é vendorizada na archive-fonte candidata. O candidato Go
não foi examinado nesta etapa.

### Triagem adicional: aws-smithy-eventstream

Os 11 arquivos indexados de `aws-smithy-eventstream 0.61.6` tiveram SHA-256
locais coincidentes. Seus cabeçalhos creditam Amazon.com, Inc. ou afiliadas e
declaram `SPDX-License-Identifier: Apache-2.0`:

| Arquivo | SHA-256 |
|---|---|
| `benches/write_message_performance.rs` | `cf609ec1353280a90f83656c3acb8c033a950e23b9b62c8ef611bc112a3a7a05` |
| `src/arbitrary.rs` | `283e92a621d57764e306bb6b94dfe32d689f5b99d29633598198f11c53c1e052` |
| `src/buf/count.rs` | `ea307749c2d482ea1b9cf2b7c24930d6d66db093e23d222c89d2e92414331cd8` |
| `src/buf/crc.rs` | `c5bf9c72d868cb98ae4c14761f87d5c5fa8e58ef2fc32b8cb59514ded4a9084b` |
| `src/buf.rs` | `0e7d1787d85a464f8055f0bf88e931b897db234b9be068a80284305a63fcf704` |
| `src/error.rs` | `03e604a46a6027747d67285f6d40c894846bf6c7481c93f10e8504d05f6fd44d` |
| `src/frame.rs` | `d0f6ad72a2472b05e239f8427cccb9152ae95a241d4f95987ea2e298999c1e42` |
| `src/lib.rs` | `fff6b05fb7663b5f2b7bb946617ac69eb25683168ba46c5b31dffacfe27804bf` |
| `src/message_size_hint.rs` | `af0d4e45bc74938c28c1a8ffe5e3943f05ac6e0fc1478dc2d1872fb29ee84adb` |
| `src/smithy.rs` | `ac3f01a0b503c42de35ea1dfda978f3a6a746be6bf130188d875cb739ec18a61` |
| `src/test_util.rs` | `f1c1ca28920abfc89d3734d4c75b4cc6bd17e2160b77c3eefcc5cd56e4733100` |

A triagem cobre esses cabeçalhos indexados, não todos os avisos da crate ou
eventuais notices upstream; não determina obrigações por artefato. Na extração
Rust `8avAZthl`, `cargo tree --offline --locked --workspace --all-features
--target x86_64-unknown-linux-gnu -i aws-smithy-eventstream --depth 2`
selecionou a crate via `aws-sdk-s3`, `aws-sigv4` e outras dependências, incluindo
arestas de desenvolvimento. O grafo não prova inclusão dos arquivos ou do
benchmark em binário; a crate não é vendorizada na archive-fonte candidata.
O candidato Go não foi examinado nesta etapa.

### Triagem adicional: aws-types

Os 12 arquivos indexados de `aws-types 1.7.0` tiveram SHA-256 locais
coincidentes. Seus cabeçalhos creditam Amazon.com, Inc. ou afiliadas e declaram
`SPDX-License-Identifier: Apache-2.0`:

| Arquivo | SHA-256 |
|---|---|
| `build.rs` | `77722f458243d7f9035d3bc53acc695f98bf9f3f40c2aa23a0ed50f12cdc873c` |
| `src/app_name.rs` | `42d2480c1d27fda0232abbee18e90ccafbacf6199da7b5c165980eb72b6ca0b5` |
| `src/build_metadata.rs` | `7140adf79acf104765713503d7980cdf80f53205c4971a88691bb5ec4065b49a` |
| `src/endpoint_config.rs` | `313d8bb5a87deb293c8e45741919b1662ed542f9587a2fde5dc1169383be0918` |
| `src/lib.rs` | `facc9b26cdf0803686456af0380b3fcaa8bf9ffcd6357230872d6ac4c9c1403f` |
| `src/origin.rs` | `ce360f4edecd62754cfab4119092b6325379f97b056de1306c619bd22c8a7683` |
| `src/os_shim_internal.rs` | `5df8ddf4c8f2d0f4a05d45f774c334c5f0d273364c0ce913e5053f08d5153c21` |
| `src/region.rs` | `f71010a2c4085f565332aeabbd27dccf7c091af2b27a68ab2bddb79bcd03e563` |
| `src/request_id.rs` | `04bba530172cd2b6dacdeaf682879aba1b82c94a67cfc09ed0104f84478091b3` |
| `src/sdk_config.rs` | `d8b458069718600f643116ae50ea7f5dc2a48b22b2153b8e3a5b7bc4ce3fe79b` |
| `src/sdk_ua_metadata.rs` | `b14f16191a063a14efcb39c909916be6853967ef878cbf82859d4e8ee060e767` |
| `src/service_config.rs` | `8b3d2b41a4e8c17b1bdfe4f1c0e1012fe5d83e8c4ec31cb5260f05ebcc52dbad` |

A triagem cobre esses cabeçalhos indexados, não todos os avisos da crate ou
eventuais notices upstream; não determina obrigações por artefato. Na extração
Rust `8avAZthl`, `cargo tree --offline --locked --workspace --all-features
--target x86_64-unknown-linux-gnu -i aws-types --depth 2` selecionou a crate
via `aws-runtime` e `aws-sdk-s3`, incluindo arestas de desenvolvimento. O grafo
não prova inclusão dos arquivos em binário; a crate não é vendorizada na
archive-fonte candidata. O candidato Go não foi examinado nesta etapa.

### Triagem adicional: aws-smithy-observability

Os nove arquivos indexados de `aws-smithy-observability 0.3.0` tiveram SHA-256
locais coincidentes. Seus cabeçalhos creditam Amazon.com, Inc. ou afiliadas e
declaram `SPDX-License-Identifier: Apache-2.0`:

| Arquivo sob `src/` | SHA-256 |
|---|---|
| `attributes.rs` | `a9f3b9dbe15c5b39c0f3f9fd6c48f53da6d9a4ae862560353c32d6d51eaf2820` |
| `context.rs` | `3899580a3c03589b3401da5b86a05cf30ed3ba861ac738b498ca7debc240de15` |
| `error.rs` | `c206071022d6194be894898b482c81960abbcb136b69022b10421614e72c566a` |
| `global.rs` | `29c9e345554bb936391a5a325ddebb6135d4c4941619e86ab02eaf5b6704b9bb` |
| `instruments.rs` | `7efc5e664e311529a8f5bad2a67a9a1ec6d19340dc7552fb6c54060489a9eb61` |
| `lib.rs` | `3cf8685b4ceb3949e490967fc828dc1dbbbdfa3b1fa8d494a1045977b0635e6f` |
| `meter.rs` | `575b2136a0fd51c467e14a67eacc57b36d3a9316dbcc851765a11427e35c4248` |
| `noop.rs` | `4d16117eaf4d2d9b7b441b2633a7c323cac09f3667f8db3332c64e0dbcc00272` |
| `provider.rs` | `7c284035f63978a06c60a2aec7f7e6cde10a408f6ec7fd81a6c90a45c3f3175a` |

A triagem cobre esses cabeçalhos indexados, não todos os avisos da crate ou
eventuais notices upstream; não determina obrigações por artefato. Na extração
Rust `8avAZthl`, `cargo tree --offline --locked --workspace --all-features
--target x86_64-unknown-linux-gnu -i aws-smithy-observability --depth 2`
selecionou a crate via `aws-sdk-s3` e `aws-smithy-runtime`, incluindo arestas de
desenvolvimento. O grafo não prova inclusão dos arquivos em binário; a crate
não é vendorizada na archive-fonte candidata. O Go não foi examinado nesta etapa.

### Triagem adicional: ryu

Os 15 arquivos indexados de `ryu 1.0.23` tiveram SHA-256 locais coincidentes
e as primeiras 19 linhas idênticas. Esse cabeçalho diz que foram
traduzidos de código C de [ulfjack/ryu](https://github.com/ulfjack/ryu),
credita copyright 2018 Ulf Adams e oferece Apache 2.0 ou Boost Software License
1.0, com disclaimer; não fixa revisão upstream.

| Arquivo | SHA-256 |
|---|---|
| `src/common.rs` | `cae347e97fc30c50a964f80425e8c3e69ece2b8ab81f9b81b9baa7fcec64a001` |
| `src/d2s.rs` | `f2612785ebe510c935b979dc5f66f6b8c818ca8a4cf0364ce1fe1d41fea39592` |
| `src/d2s_full_table.rs` | `9b0186acbc6d65dc55c17e16125be707a2bfb920d22b35d33234b4cc38566a36` |
| `src/d2s_intrinsics.rs` | `bbf15472f4299942312e80a992cbc2f47f85f17ed193f24084534434dbfb26e7` |
| `src/d2s_small_table.rs` | `db3bbe4002d816785b0ee233c330f19fa7002f31dab47dc6f67b266996fe3ae4` |
| `src/digit_table.rs` | `02351ca54cb8cb3679f635115dd094f32fd91750e9f66103c1ee9ec3db507072` |
| `src/f2s.rs` | `ace23f2d4459055bbb8544c2efc09753fec27ff3ce44648e8015e8aff5696b0c` |
| `src/f2s_intrinsics.rs` | `4667c34aab7637f2330df7bbd4c178c0ba1dd5854ff15b9a36a96fd5f527fb66` |
| `tests/common_test.rs` | `599781a637d9b9756858aabfe5c38a0734a550debd3d94774f33792b7b3c8240` |
| `tests/d2s_intrinsics_test.rs` | `15d11b70810bf04f33f8b185bf7f010a436a4edb47fa4648b1a036568c2c5d15` |
| `tests/d2s_table_test.rs` | `7e06679c3486feab200ebd82d090457f0cdae2503610d721ced3739dbc86c574` |
| `tests/d2s_test.rs` | `5cadb01403383bfef181d22cc0d8764e4f46f7a84655166e6b75081132498fc7` |
| `tests/f2s_test.rs` | `168f6e32d86fb98fda9afad0022e3b189e9e1f60756de8c9338913c0668ff768` |
| `tests/s2d_test.rs` | `75c3a1044881718db65e05f25c9f6e1d005392dddb2e8dafb799668bb6a9a5c3` |
| `tests/s2f_test.rs` | `ca44faee70f56f9ef51349418c41a67561fc46ae07c5d2bf0fe6f95ffd1a44f6` |

A conferência cobre esses cabeçalhos indexados, não todos os avisos ou textos
de licença da crate; não identifica a revisão C usada nem determina obrigações
por artefato. Na extração Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i ryu --depth 2`
selecionou `ryu 1.0.23` via `aws-smithy-types`, incluindo arestas de
desenvolvimento. O grafo não prova inclusão dos arquivos ou testes em binário;
a crate não é vendorizada na archive-fonte candidata. O Go não foi examinado.

### Triagem adicional: parking_lot_core

Os 15 arquivos indexados de `parking_lot_core 0.9.12` tiveram SHA-256 locais
coincidentes com o índice. Os cabeçalhos identificam copyright 2016 Amanieu
d'Antras e apontam Apache 2.0 ou MIT, à escolha do destinatário:

| Arquivo sob `src/` | SHA-256 |
|---|---|
| `lib.rs` | `c8c164deb6369857838b886423918383fb80ab4153e7eac5fffacbed12a533e7` |
| `parking_lot.rs` | `ead11d575a1930c241aeec20f0e5e504bcdad637e284449ef4efcb9f9f175777` |
| `spinwait.rs` | `0d73980c72e84fd75c73693f0b351e097353b46f4b8aaa67b3dde7c721817bf7` |
| `thread_parker/generic.rs` | `414bd3114b40f7665efe59fa4c5033d2401eafc58b57a9ba5803949d26cc0454` |
| `thread_parker/linux.rs` | `70bfe6c168fc431965374d921e33b3e04536cb68fde3e807a837aa38a65772b0` |
| `thread_parker/redox.rs` | `38fbede41817b6606a5612ee9456940eaf627540a4aa39de8452e355ca1df306` |
| `thread_parker/sgx.rs` | `969a706a15b96bfd27c1c4c14aa87c0cbfc7a104cdce667262768470035dc971` |
| `thread_parker/unix.rs` | `4cd468fcee83f29e9c923c5a574d0f5cbff0207741dae2eba99b16509fdb6ffa` |
| `thread_parker/wasm.rs` | `86e954d009d57d2dfdbca71e5c0f116a99a182af83b7e51ed5b14fc55b2f7b01` |
| `thread_parker/wasm_atomic.rs` | `6fbc09accc9df9d42444cec6f55d3eed2c7078a474485fa269a9f503da2ee1c5` |
| `thread_parker/windows/keyed_event.rs` | `12b72b70ad9d58d943b51246621ac73223debb0b37abff990fa0f895c575f7c7` |
| `thread_parker/windows/mod.rs` | `ff86c7fc19eb139b16d3dd0452a512e3ff3e2f18d1034303682c0b45aebd2528` |
| `thread_parker/windows/waitaddress.rs` | `9ab190cd9aaaf89bed4535f8a0ccb81102b7abb64d4f94b31aa98f8a2acaf8de` |
| `util.rs` | `285e6133150645525f2ca1ece41f6d35bad4e7c5e08b42b20c99d2a97e04a974` |
| `word_lock.rs` | `57133a9da052d8e86eb38ff1968129149838918d69f52761dc8757b541f9d044` |

A triagem cobre os cabeçalhos indexados, não todos os avisos ou textos de
licença da crate. Na extração do exemplo Rust de telemetria `8avAZthl`, `cargo
tree --offline --locked --all-features --target x86_64-unknown-linux-gnu -i
parking_lot_core@0.9.12 --depth 2` selecionou a crate via `parking_lot` e
`prometheus`. O grafo não prova quais arquivos para plataformas distintas ou
objetos compõem um binário; a crate não é vendorizada na archive-fonte candidata.
Os avisos por artefato seguem abertos; o Go não foi examinado nesta etapa.

### Triagem adicional: parking_lot

Os 12 arquivos indexados de `parking_lot 0.12.5` tiveram SHA-256 locais
coincidentes. Os cabeçalhos creditam copyright 2016 Amanieu d'Antras e declaram
Apache 2.0 ou MIT, à escolha do destinatário:

| Arquivo sob `src/` | SHA-256 |
|---|---|
| `condvar.rs` | `182e5a2f6c06cf665bb7723c2780580647a73d9de5ff319f26c0fbbf5e7280ea` |
| `elision.rs` | `7070a29080f4c7d3b2c9b6e51a0d8230ebc8e56e4790aaeda1747b6c073a7438` |
| `fair_mutex.rs` | `2d0c5c2b779492653cad99825180628af95ce5f4bdbcfed57e7a5f2138a55fc0` |
| `lib.rs` | `0ad0d7a416c12f99db0033f22c98baf201878ab40b3cd34fe74e7bd0e5b87cca` |
| `mutex.rs` | `b7ee456958a3b0dc208dd39be195641ce3998f7906d101ccf284fa0f84c58d14` |
| `once.rs` | `58c78dba952e217c17bd8d5cf3dce32bb78e3e1c38a86860c8edc8c308caeca7` |
| `raw_fair_mutex.rs` | `316f954d9673ac5b8d6bf4c19f2444800f63daf801c224d986e2d6dac810643c` |
| `raw_mutex.rs` | `4a79255673f47e167ce28d331c3b51f245017c7023e36c1c9b09baf00cc121c6` |
| `raw_rwlock.rs` | `bdbb06e1433ea2bc1dbbfd69e66755bcad588f48c6552c0fe125d98419035791` |
| `remutex.rs` | `f0c0711ed46be67f7736a8500689e37a301ccf0dacb7706e1087513d3e87a432` |
| `rwlock.rs` | `7edd9dd320858a628388c1ebca1ed89cc2984abc30157c4b0c03194ebba7eade` |
| `util.rs` | `37a2c8b5c9254df83e8f3a5cd831558c1045061a76c2571bdc4d78eb86e467f2` |

A conferência cobre esses cabeçalhos indexados, não todos os avisos ou textos
de licença da crate; não determina obrigações por artefato. Na extração do
exemplo Rust separado de telemetria do candidato `8avAZthl`, `cargo tree
--offline --locked --all-features --target x86_64-unknown-linux-gnu -i
parking_lot@0.12.5 --depth 2` selecionou a crate via `prometheus`. O grafo não
prova inclusão dos arquivos em binário; a crate não é vendorizada na archive-fonte
candidata. O candidato Go não foi examinado nesta etapa.

### Triagem adicional: futures-channel

O índice registra avisos em `futures-channel 0.3.34/src/mpsc/queue.rs`. O
SHA-256 local `22034085dc22050b708a37854e215fc7cbb16d65edc60370cb5d8f4b7faca18e`
coincide com o índice. O cabeçalho credita copyright 2010–2011 Dmitry Vyukov e
contém permissão de redistribuição com condições para fonte e binário, disclaimer
e um parágrafo adicional sobre opiniões e conclusões. O comentário seguinte diz
que a implementação da fila é essencialmente a mesma usada nos canais MPSC da
biblioteca padrão; não identifica uma revisão. O texto incorporado completo
deve ser considerado na avaliação apropriada, sem presumir que o manifesto da
crate substitua esse aviso. Esta triagem não esgota os demais arquivos.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i futures-channel --depth 2`
selecionou `futures-channel 0.3.34` via `hyper` e `hyper-util`. O grafo não
prova inclusão do arquivo em binário; a crate não é vendorizada na archive-fonte
candidata. Os avisos por artefato permanecem abertos; o Go não foi examinado.

### Triagem adicional: num-integer

O índice registra o cabeçalho de `num-integer 0.1.47/src/lib.rs`. O SHA-256
local `d4485e6d50208695aa575427ef7309500973192839ca8f7bd8506d77c1b88e9a`
coincide com o índice. O cabeçalho credita copyright 2013–2014 The Rust Project
Developers e declara Apache 2.0 ou MIT, à escolha do destinatário. A análise
cobre apenas o cabeçalho indexado, não todos os avisos ou textos de licença da
crate; não determina obrigações por artefato.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i num-integer --depth 2`
selecionou `num-integer 0.1.47` via `aws-smithy-types`, incluindo arestas de
desenvolvimento. O grafo não prova inclusão do arquivo em binário; a crate não
é vendorizada na archive-fonte candidata. O Go não foi examinado nesta etapa.

### Triagem adicional: cpubits

O índice registra o aviso em `cpubits 0.1.1/src/lib.rs:318`. O SHA-256 local
`f7171de13cd85967b2ff1dd189b19f224d28b9695e7591fb8d92621a9e2f439b`
coincide com o índice. O comentário descreve o macro `cfg_if` como cópia
vendorizada de `cfg_if::cfg_if`, credita copyright 2014 Alex Crichton e declara
licenciamento duplo Apache 2.0 e MIT; não identifica revisão de origem. A
verificação cobre esse trecho indexado, não todos os avisos da crate nem a
aplicabilidade a qualquer distribuição.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i cpubits --depth 2`
selecionou `cpubits 0.1.1` via `crypto-bigint`. A cópia de `cfg_if` está na
fonte de `cpubits` no cache; isso não significa que a crate esteja vendorizada
na archive-fonte candidata. O grafo não prova inclusão do macro em binário;
os avisos por artefato permanecem abertos. O Go não foi examinado nesta etapa.

### Triagem adicional: rustc_version

O índice registra o cabeçalho de `rustc_version 0.4.1/src/lib.rs`. O SHA-256
local `6d0dbf7bd12ba7121e6767841a0cee23b8282cc4d7c4b778a1e33129abdd7d72`
coincide com o índice. O cabeçalho credita copyright 2016 aos desenvolvedores
de rustc-version-rs e declara Apache 2.0 ou MIT, à escolha do destinatário.
A conferência cobre apenas o cabeçalho indexado, não todos os avisos ou textos
de licença da crate; não determina obrigações por artefato.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i rustc_version --depth 2`
selecionou `rustc_version 0.4.1` como dependência de compilação de `aws-types`,
com arestas de desenvolvimento no grafo. Isso não prova inclusão da crate ou
do arquivo em binário; ela não é vendorizada na archive-fonte candidata. O
candidato Go não foi examinado nesta etapa.

### Triagem adicional: subtle

O índice registra o cabeçalho de `subtle 2.6.1/src/lib.rs`. O SHA-256 local
`ced6ebd3622af527cbbbb09feb57a2894751ff07e7ca8f97f22bad89401e5c8d`
coincide com o índice. O cabeçalho credita isis lovecruft e Henry de Valence,
2016–2018, e remete ao arquivo `LICENSE`. O `LICENSE` local examinado tem
SHA-256 `d1fc1bc0d155df60b2e7705b6b2ae02a05c96f948e1cec6e2fb86360b09f346b`;
contém texto BSD de três cláusulas, dois avisos de copyright, condições para
redistribuição de fonte e binário e disclaimer. A análise não cobre outros
avisos da crate nem determina condições aplicáveis por artefato.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i subtle --depth 2`
selecionou `subtle 2.6.1` via `aws-sigv4`, `rustls` e outras dependências,
incluindo arestas de desenvolvimento. O grafo não prova inclusão do arquivo em
binário; a crate não é vendorizada na archive-fonte candidata. O candidato Go não
foi examinado nesta etapa.

### Triagem adicional: lock_api

Os quatro arquivos `lock_api 0.4.14` registrados no índice tiveram SHA-256
locais coincidentes. Seus cabeçalhos creditam Amanieu d'Antras e declaram
Apache 2.0 ou MIT, à escolha do destinatário:

| Arquivo | SHA-256 | Ano no cabeçalho |
|---|---|---|
| `src/lib.rs` | `f6dfe0a321e47b5ea9f01a59711e5d3a0442949a829eb4957fa1491fbce63e5c` | 2018 |
| `src/mutex.rs` | `1487130ba159c9158623d58204b52531c0feaabcdea8e58c0740a29d4f33e5af` | 2018 |
| `src/remutex.rs` | `a0cc36a3ac9ba2a399f83667ca53670afbec0d0bae6e9c103323918594873eeb` | 2018 |
| `src/rwlock.rs` | `3094177475cf67c8dc34e68b3fdff2d4133ffb453860b72037477734cb87aefc` | 2016 |

A conferência cobre apenas os cabeçalhos indexados, não todos os avisos ou
textos de licença da crate; não determina obrigações por artefato. Na extração
do exemplo Rust separado de telemetria do candidato `8avAZthl`, `cargo tree
--offline --locked --all-features --target x86_64-unknown-linux-gnu -i lock_api
--depth 2` selecionou `lock_api 0.4.14` via `parking_lot` e `prometheus`. O
grafo não prova inclusão dos arquivos em binário; a crate não é vendorizada na
archive-fonte candidata. Esta etapa não examina o candidato Go.

### Triagem adicional: lazy_static

Os três arquivos `lazy_static 1.5.1` registrados no índice tiveram SHA-256
locais coincidentes. Seus cabeçalhos creditam copyright 2016 aos desenvolvedores
de lazy-static.rs e declaram Apache 2.0 ou MIT, à escolha do destinatário:

| Arquivo | SHA-256 |
|---|---|
| `src/core_lazy.rs` | `dfb195f44a62b19a32542297edfd281bc40837e48905603dfec20d62b12711d0` |
| `src/inline_lazy.rs` | `cd8a9babcc3af7d0d39f9874dc575a1da000f0d6048a66296f3ca197c2c00b46` |
| `src/lib.rs` | `9478f4130a4dca55df412267b892717047555e3da4d23ea4eae24c5e40b5e027` |

A conferência cobre os cabeçalhos indexados, não todos os avisos ou textos de
licença da crate; não determina obrigações por artefato. Na extração do exemplo
Rust separado de telemetria do candidato `8avAZthl`, `cargo tree --offline
--locked --all-features --target x86_64-unknown-linux-gnu -i lazy_static --depth 2`
selecionou a versão `1.5.1` via `prometheus 0.14.0`. O grafo não prova inclusão
dos arquivos em binário; a crate não é vendorizada na archive-fonte candidata.
Esta etapa não examina o candidato Go.

### Triagem adicional: form_urlencoded

O índice registra o cabeçalho de `form_urlencoded 1.2.2/src/lib.rs`. O SHA-256
local `766b5d679064e01f7e6cce6f127a23885f79806ce3bccc49e4fe41933b83fd8d`
coincide com o índice. O cabeçalho credita copyright 2013–2016 aos
desenvolvedores de rust-url e declara Apache 2.0 ou MIT, à escolha do destinatário.
Essa conferência cobre somente o cabeçalho indexado, não todos os avisos ou
textos de licença da crate; não determina obrigações por artefato.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i form_urlencoded --depth 2`
selecionou `form_urlencoded 1.2.2` via `aws-sigv4` e `url`, incluindo arestas
de desenvolvimento. O grafo não prova inclusão do arquivo em binário; a crate
não é vendorizada na archive-fonte candidata. O Go não foi examinado nesta etapa.

### Triagem adicional: percent-encoding

Os dois arquivos de `percent-encoding 2.3.2` registrados no índice tiveram
SHA-256 locais coincidentes. Seus cabeçalhos creditam copyright 2013–2016 aos
desenvolvedores de rust-url e declaram Apache 2.0 ou MIT, à escolha do destinatário:

| Arquivo | SHA-256 |
|---|---|
| `src/ascii_set.rs` | `f75552e4f446ed240cef64c1367e848cf3ae65f77602085459566d835a1d8587` |
| `src/lib.rs` | `641903d385581e76f31ef8c991d83c2a5d7fbbc2879d93909c539037e4e1e3e2` |

A conferência cobre os cabeçalhos indexados, não todos os avisos ou textos de
licença da crate; não seleciona licença nem determina obrigações por artefato.
Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i percent-encoding --depth 2`
selecionou a versão `2.3.2` via `aws-sdk-s3`, `url` e outras dependências,
incluindo arestas de desenvolvimento. O grafo não prova inclusão dos arquivos
em binário; a crate não é vendorizada na archive-fonte candidata. Esta etapa
não examina o candidato Go.

### Triagem adicional: stable_deref_trait

O índice registra o cabeçalho de `stable_deref_trait 1.2.1/src/lib.rs`. O
SHA-256 local `4736941fa6c790367158082cb55e5fc89aba7c7f0a85405aada8271fbb5c87ad`
coincide com o índice. O cabeçalho credita copyright 2017 Robert Grosse e
declara Apache 2.0 ou MIT, à escolha do destinatário. A conferência cobre apenas
o cabeçalho indexado, não todos os avisos ou textos de licença da crate; não
seleciona licença nem determina obrigações por artefato.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i stable_deref_trait --depth 2`
selecionou `stable_deref_trait 1.2.1` via `yoke` e dependências ICU. O grafo
não prova inclusão do arquivo em binário; a crate não é vendorizada na
archive-fonte candidata. Esta etapa não examina o candidato Go.

### Triagem adicional: untrusted

Os quatro arquivos de `untrusted 0.9.0` registrados no índice tiveram hashes
locais coincidentes. Os cabeçalhos preservam copyright de Brian Smith e texto
de permissão com condição de inclusão dos avisos em cópias e disclaimer:

| Arquivo | SHA-256 | Anos no cabeçalho |
|---|---|---|
| `src/input.rs` | `aaa007b7051f03b32f6c71fa8164780435e637bb2242df7792942b699d2d9cbd` | 2015–2021 |
| `src/lib.rs` | `860cdbec448f82196aa2242ad52945dfc60f29fcfbc28d244877fca0c0331fcc` | 2015–2021 |
| `src/reader.rs` | `3826d4de44a970ed42e3f7dead6f4b51b60ccf02c03b9ac7140c80a9e57b104d` | 2015–2021 |
| `tests/tests.rs` | `cefcc4543e3d49e1af0e6e6efebebbdfb61cad131d0ea2d69c14ed11f398cca5` | 2015–2019 |

Essa verificação cobre os cabeçalhos indexados, não todos os avisos da crate ou
sua aplicabilidade a uma distribuição. Na extração Rust `8avAZthl`, `cargo tree
--offline --locked --workspace --all-features --target x86_64-unknown-linux-gnu
-i untrusted --depth 2` selecionou `untrusted 0.9.0` via `rustls-webpki` e
`rustls`. O grafo não prova inclusão dos arquivos ou testes em binário; a crate
não é vendorizada na archive-fonte candidata. O Go não foi examinado nesta etapa.

### Triagem adicional: ChaCha AWS-LC

O índice registra uma ocorrência `origin` em `aws-lc-sys 0.47.0`,
`aws-lc/crypto/chacha/chacha.c:4`. O SHA-256 da cópia local,
`6fd983e25d701ae54a817da48babef8262652c2e555d2bc0154df425ee134e73`,
coincide com o índice. O cabeçalho credita Google Inc. em 2014, declara SPDX
`ISC` e afirma que o arquivo foi adaptado de código eSTREAM de D. Bernstein,
descrito no comentário como domínio público. O comentário não identifica arquivo
ou revisão da fonte eSTREAM; a declaração não substitui a verificação da fonte
original ou a avaliação dos avisos aplicáveis. A triagem cobre apenas esta
ocorrência indexada, não o restante da crate nem os arquivos assembly já
examinados em `s3-aws-lc-transitive-notices.md`.

O grafo Linux x86-64 já documentado para a extração Rust `8avAZthl` selecionou
`aws-lc-sys` via `aws-lc-rs`; isso não prova compilação deste arquivo ou sua
inclusão em binário. A crate não é vendorizada na archive-fonte candidata; o
candidato Go não foi examinado nesta etapa. Notices por artefato seguem abertos.

Investigação adicional: a [página ChaCha de D. J. Bernstein](https://cr.yp.to/chacha.html)
identifica várias implementações de ChaCha no diretório Salsa20 da suíte de
benchmark eSTREAM versão 2008-01-20. O [arquivo de referência ChaCha8](https://cr.yp.to/streamciphers/timings/estreambench/submissions/salsa20/chacha8/ref/chacha.c)
consultado retornou HTTP 200 e declara no cabeçalho `chacha-ref.c version
20080118`, D. J. Bernstein e `Public domain.`. Seu macro `QUARTERROUND` usa
rotações 16, 12, 8 e 7, como o macro no arquivo AWS-LC local. Essa semelhança
limitada não identifica o arquivo ou revisão efetivamente usados na adaptação,
nem constitui comparação integral. A página lista também outras variantes;
seus arquivos e condições não foram examinados nesta etapa. A declaração do
arquivo de referência e sua relevância jurídica para qualquer distribuição
permanecem sujeitas à avaliação apropriada; o grafo não demonstra inclusão do
código em artefatos finais.

### Triagem adicional: P-224 e EVP AWS-LC

Dois arquivos de `aws-lc-sys 0.47.0` sob `aws-lc/crypto/fipsmodule/` tiveram
SHA-256 locais coincidentes com o índice:

| Arquivo | SHA-256 | Contexto das ocorrências |
|---|---|---|
| `ec/p224-64.c` | `ab3ea7d17787f35a6236e66cdf82fe445f220cf778af49efe0486eaca05c3f21` | Linha 476 atribui uma cadeia de cálculo copiada a código de `djb`, sem identificar arquivo ou revisão. O cabeçalho credita Google, 2015, declara ISC e menciona como inspiração implementações de Daniel J. Bernstein e Adam Langley descritas ali como domínio público. |
| `evp/internal.h` | `63d76d49409c546d6d369b974521a3b9162a23d964e5a010779f403b84334559` | Linha 23 descreve um membro que não deve ser copiado de uma estrutura, não uma atribuição de origem de código. O cabeçalho credita Eric Young e declara Apache-2.0. |

As declarações dos cabeçalhos e comentários não identificam por si a revisão da
cadeia copiada nem determinam os avisos de distribuição; a fonte anterior e sua
relação com o trecho permanecem para avaliação. Esta triagem cobre apenas as
ocorrências indexadas. Conforme o grafo Linux x86-64 já registrado para a
extração Rust `8avAZthl`, `aws-lc-sys` é selecionada via `aws-lc-rs`, mas o
grafo não prova compilação desses arquivos ou sua presença em binário. A crate
não é vendorizada na archive-fonte candidata; o Go não foi examinado nesta etapa.

Investigação histórica da cadeia P-224: no histórico público de BoringSSL, o
commit [`f0258fe956843e4efc7c502e44d09b21668b096e`](https://github.com/google/boringssl/blob/f0258fe956843e4efc7c502e44d09b21668b096e/crypto/ec/p224-64.c),
de 2015-10-08, adicionou `crypto/ec/p224-64.c`; o arquivo nessa revisão já
continha o comentário `Computation chain copied from djb's code` junto a
`felem_inv`. Seu cabeçalho credita Google Inc. em 2015 e contém permissão e
disclaimer em formato ISC; também descreve como inspiração implementações de
Bernstein e Langley. O commit
[`aacb72c1b7ffe74520cf59a50525df79542b36d0`](https://github.com/google/boringssl/commit/aacb72c1b7ffe74520cf59a50525df79542b36d0)
renomeou o caminho para `crypto/fipsmodule/ec/p224-64.c` em 2017. Isso
documenta uma fonte histórica comparável, não demonstra que o AWS-LC usou
exatamente essa revisão. O comentário não identifica o arquivo ou versão do
código de `djb`; a referência no cabeçalho não prova, por si, qual fonte
forneceu a cadeia. Não foi realizada comparação integral nem estabelecida a
aplicabilidade dos avisos às tabelas ou aos binários distribuídos.

### Triagem adicional: capacidade de CPU e cifra AWS-LC

Dois arquivos de `aws-lc-sys 0.47.0` tiveram SHA-256 locais coincidentes com
o índice:

| Arquivo sob `aws-lc/crypto/` | SHA-256 | Contexto das ocorrências |
|---|---|---|
| `cipher_extra/e_aesgcmsiv.c` | `23175bfe25722b6bc356ba656b38226481b401e314aed1dfbb48242e31ec7322` | Linhas 107 e 294 descrevem valores calculados a partir de endereço e tag, não origem de código. Cabeçalho: copyright 2017 Google Inc., SPDX `ISC`. |
| `fipsmodule/cpucap/cpucap.c` | `302ca6040731b8510c7688bdbb0c95a89a7f32d73d31622f4487f687b7519192` | Linha 4 diz que o conteúdo foi copiado de `crypto/crypto.c` para mover a lógica de capacidade de CPU; não fixa revisão nem estabelece a proveniência anterior desse conteúdo. Cabeçalho: copyright Amazon, SPDX `Apache-2.0 OR ISC`. |

Esta classificação cobre apenas as ocorrências indexadas desses arquivos; não
substitui a revisão dos avisos nem a comparação do histórico da fonte copiada.
Como registrado na triagem das tabelas abaixo, o grafo Linux x86-64 da extração
Rust `8avAZthl` selecionou `aws-lc-sys` via `aws-lc-rs`; isso não comprova
compilação dos arquivos ou sua inclusão em binário. A crate não é vendorizada
na archive-fonte candidata; o candidato Go não foi examinado nesta etapa.

### Triagem adicional: socket2

Os quatro arquivos de `socket2 0.6.5` registrados no índice têm hashes locais
coincidentes. Cada cabeçalho credita copyright 2015 The Rust Project Developers
e declara Apache 2.0 ou MIT, à escolha do destinatário:

| Arquivo | SHA-256 |
|---|---|
| `src/lib.rs` | `f3782bc62e5a17e25e8868c55a14fa1da89f564bd0db9ad234dff429094386e1` |
| `src/socket.rs` | `14d718fc14286e1a0c35d785d1ece2096c5af12188c248a7493851d0d87f824d` |
| `src/sys/unix.rs` | `5193fd8c91103f8c73c03c0237c2a493aba911a2995c3750ffb13b054f5c9433` |
| `src/sys/windows.rs` | `ea941638399a4e3f7cf0ac2f21d5eaf44d62d26c760d97b1660ba653853101dd` |

A conferência cobre os cabeçalhos indexados, não todos os avisos ou textos de
licença da crate; não seleciona licença nem determina obrigações por artefato.
Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i socket2 --depth 2`
selecionou `socket2 0.6.5` via `hyper-util` e `tokio`. O grafo não prova quais
arquivos ou objetos compõem um binário, e a crate não é vendorizada na
archive-fonte candidata. O candidato Go não foi examinado nesta etapa.

### Triagem adicional: tabelas aws-lc-sys

Quatro arquivos de `aws-lc-sys 0.47.0` sob `aws-lc/crypto/fipsmodule/` com
ocorrências `origin` tiveram hashes locais coincidentes com o índice:

| Arquivo | SHA-256 | Contexto examinado |
|---|---|---|
| `ec/p384_table.h` | `d3e60fb216ca5225947ff61c86db8b5d84ec7a2c264ba12468a6920d439295e2` | Tabela e uso atribuídos ao projeto ECCKiila; geração atribuída a método em `main.py` numa branch móvel. |
| `ec/p521_table.h` | `8291ca4bb5407314ced1b92c613addbdb4f6bdd5365c32818973b83b5a36cb67` | Mesma atribuição a ECCKiila e ao método de geração. |
| `ec/ec_key.c` | `324f47e0d67617dc68fe5d80a561c9224094a5555d635077366bb64bfe9a0c92` | Comentário descreve uma chave pública derivada de uma chave privada, não origem de código. |
| `evp/digestsign.c` | `221027c46868e49db6020fdf80d1d952c03fc50a7cbe791bf3afdbbf207c469d` | Comentário descreve cópia de um valor entre campos, não origem de código. |

Os dois headers de tabela declaram geração por `make_tables.go`, copyright
Amazon e `Apache-2.0 OR ISC`; referem o projeto ECCKiila e um método de geração
sem fixar revisão. A fonte original e seus avisos não foram revisados
integralmente, e a menção não determina permissões nem prova reprodução dos
arquivos gerados. Esta triagem cobre somente as ocorrências indexadas.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i aws-lc-sys --depth 2`
selecionou `aws-lc-sys 0.47.0` via `aws-lc-rs` e `rustls`. O grafo não prova
compilação dos arquivos ou inclusão em binário; a crate não é vendorizada na
archive-fonte candidata. A revisão por artefato permanece aberta; esta etapa
não abrange o candidato Go.

Investigação adicional ECCKiila: a API GitLab identificou a branch `master`
no commit `74f3d38df9224689d2c74ff7d7832b6c45070ce6`. Consultados pela API
com essa revisão fixa, `LICENSE` e `main.py` retornaram HTTP 200 e os SHA-256
informados pela API foram, respectivamente,
`3bc1b24c38c946fb38621261f40d5c065d19495581f7cb353ddafa268d8b870e` e
`be1269a0bba1d375e399ccf76e3d96c794cad72d5fd931902a7cac9fb9cbd991`.
O `LICENSE` nessa revisão contém licença MIT e copyright 2020 Luis
Rivera-Zamarripa, Jesús-Javier Chi-Domínguez e Billy Bob Brumley; exige
preservação do copyright e do aviso de permissão em cópias ou partes
substanciais e inclui disclaimer. A consulta das URLs raw fixadas recebeu
HTTP 403; o resultado bem-sucedido acima veio da API de arquivos, não dessas
URLs raw. Links: [LICENSE](https://gitlab.com/nisec/ecckiila/-/blob/74f3d38df9224689d2c74ff7d7832b6c45070ce6/LICENSE)
e [main.py](https://gitlab.com/nisec/ecckiila/-/blob/74f3d38df9224689d2c74ff7d7832b6c45070ce6/main.py).

Os cabeçalhos AWS-LC citam `master`, sem identificar a revisão ECCKiila usada;
o commit acima é apenas um estado observado da branch, não prova de origem das
tabelas. Não foi estabelecida a relação dos avisos com as tabelas geradas ou sua
inclusão em artefatos finais; a avaliação de notices permanece aberta.

### Triagem adicional: aritmética aws-lc-sys

Dois arquivos `aws-lc-sys 0.47.0` sob `aws-lc/crypto/fipsmodule/bn/` marcados
como `origin` foram conferidos; os SHA-256 locais coincidem com o índice:

| Arquivo | SHA-256 | Referência examinada |
|---|---|---|
| `gcd_extra.c:177` | `b3b168ba049cd4fded6f83382d44bdd8166cb799f7501e770cb188ff84abd700` | Atribui adaptação ao algoritmo 14.51 da seção 14.4.3 do *Handbook of Applied Cryptography*, com modificações; o cabeçalho credita Google, 2018, e declara ISC. |
| `jacobi.c:47` | `a305089f3aaccd684fa90d2ba02bcd25d7a947196ae8b1d194cc169cfdde3392` | Atribui lógica ao algoritmo 1.4.10 de *A Course in Computational Algebraic Number Theory*, de Henri Cohen; o cabeçalho credita The OpenSSL Project e declara Apache-2.0. |

As referências são declarações dos arquivos; não foi feita comparação com as
obras nem determinada a aplicação de direitos ou avisos dos materiais
originais. A revisão apropriada depende dos componentes efetivamente
distribuídos. Esta triagem cobre apenas os dois trechos indexados.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i aws-lc-sys --depth 2`
selecionou `aws-lc-sys 0.47.0` via `aws-lc-rs` e `rustls`. O grafo não prova
compilação desses arquivos ou inclusão em binário; a crate não é vendorizada na
archive-fonte candidata. A revisão por artefato permanece aberta; esta etapa não
abrange o candidato Go.

### Triagem adicional: testes aws-lc-sys

Quatro arquivos de teste de `aws-lc-sys 0.47.0` marcados como `origin` no
índice tiveram SHA-256 locais coincidentes com os registros:

| Arquivo sob `aws-lc/crypto/` | SHA-256 | Atribuição examinada |
|---|---|---|
| `bytestring/bytestring_test.cc` | `a9384ae43f6b97e6fb8e5faaf8a8296fb4aa9c0346f447ebec88b383b935f0ee` | Entradas inválidas adaptadas de `UTF-8-test.txt` na página de Markus Kuhn; o comentário não fixa revisão. |
| `cipher_extra/cts_test.cc` | `9500c4a43f9090bb01c2d5381230c21775b36ed6633bf42fffbdef2162f43a75` | Vetores descritos como derivados do apêndice A da RFC 8009. |
| `ecdh_extra/ecdh_test.cc` | `8b1c565909c36d0ee1f28dc1468b0ae83dfce29d9d577af0b972fc18a75b2e41` | Teste atribuído a `ECTest.LargeXCoordinateVectors`, sem identificar arquivo ou revisão no comentário. |
| `evp_extra/evp_extra_test.cc` | `2307454b0b778ba1ae6ee9f526ffc2c90b2fbe07cdc7ffb3f5756083273f3576` | Exemplo de chave de teste atribuído a um arquivo do repositório `aws/s2n-tls`, com commit `c4d90e34fbd2ba64bb17a95628622ccc1d0c6807` no link. |

As fontes originais e seus avisos não foram revisados integralmente nesta etapa;
as referências não determinam, por si, permissões ou obrigações. Esta triagem
cobre quatro arquivos, não as demais ocorrências ou avisos de `aws-lc-sys`.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i aws-lc-sys --depth 2`
selecionou `aws-lc-sys 0.47.0` via `aws-lc-rs` e `rustls`. O grafo não prova
compilação ou inclusão desses testes em binário; a crate não é vendorizada na
archive-fonte candidata. A revisão por artefato permanece aberta; esta etapa não
abrange o candidato Go.

Investigação adicional de `bytestring_test.cc`: o [arquivo UTF-8 de Markus
Kuhn atualmente disponível](https://www.cl.cam.ac.uk/~mgk25/ucs/examples/UTF-8-test.txt)
identifica o autor, a data 2015-08-28 e `CC BY 4.0` no cabeçalho. A resposta
HTTP obtida tem 22.781 bytes e SHA-256
`b51cfe9a8d2689c90b10a13a3624092d546e0837c6ff835b6e5d713c5749c8c6`;
o servidor informou `Last-Modified: Fri, 28 Aug 2015 17:41:21 GMT`. O teste
AWS-LC atribui algumas entradas inválidas a essa fonte e identifica seções do
documento, mas não fixa revisão. Não foi feita comparação integral nem
estabelecido que a versão atual era a utilizada na adaptação. O aviso de
licença observado deve ser considerado na avaliação apropriada dos materiais
efetivamente distribuídos, sem concluir automaticamente sua aplicabilidade.
Os testes estão na crate no cache, não vendorizados na archive-fonte candidata;
o grafo de dependências não comprova sua inclusão em binário.

### Triagem adicional: aws-lc-rs

Além de `src/cmac.rs`, já classificado adiante, os outros cinco arquivos de
`aws-lc-rs 1.19.0` marcados como `origin` foram conferidos; todos os SHA-256
locais coincidem com o índice:

| Arquivo | SHA-256 | Contexto das ocorrências |
|---|---|---|
| `src/aead/poly1305.rs` | `ec024b662d3290c60f69ca5eae897c3a394eece5be42cff78055bcc032f214be` | Em `#[cfg(test)]`, atribui um teste a `crypto/poly1305/poly1305_test.cc` do BoringSSL, sem fixar revisão. |
| `src/ed25519.rs` | `7c46a8dd37b06df72bdb62829c0a08a17bb34ed4c819bc8f5e8bd60e6ca63513` | Descreve chave pública derivada de chave privada. |
| `src/hmac.rs` | `52daf4d704f764f6844d5b1d06ab65de7d95fcb60ed57aea58ad48279360221c` | Descreve valor de chave derivado de outra chave. |
| `src/lib.rs` | `fd8e5a1bc7387dd7ac1aaa237b35ca18bf781541a8737513b243a098049a40f5` | Descreve chave pública derivada de chave privada. |
| `src/pbkdf2.rs` | `0535bf250ace4c903670033eb0a3964407d291f54ea194ce5d489e912b5fff75` | Descreve valores PBKDF2 derivados de entradas. |

Os quatro últimos contextos não atribuem origem ou cópia de código. A revisão
da fonte BoringSSL e dos eventuais avisos do teste permanece aberta; esta
triagem cobre somente as ocorrências indexadas, não todos os avisos da crate.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i aws-lc-rs --depth 2`
selecionou `aws-lc-rs 1.19.0` via `rustls` e `rustls-webpki`. O grafo não prova
inclusão dos trechos ou testes em binário; a crate não é vendorizada na
archive-fonte candidata. Esta etapa não abrange o candidato Go nem conclui os
notices aplicáveis por artefato.

Investigação adicional da referência BoringSSL: `.cargo_vcs_info.json` de
`aws-lc-rs 1.19.0` aponta ao commit `5d496f88b40f327e28315deecc1a5a1fd1e81e95`.
Seguindo o histórico do arquivo e duas renomeações de diretório até o caminho
`src/aead/poly1305.rs`, o commit mais antigo retornado para esse caminho foi
[`b10fe33158544a60653e7dc4ef7853ed4b5caac6`](https://github.com/aws/aws-lc-rs/blob/b10fe33158544a60653e7dc4ef7853ed4b5caac6/src/aead/poly1305.rs),
de 2022-08-30. Nele a atribuição ao teste BoringSSL já estava presente e o
cabeçalho creditava Brian Smith, 2015–2016, e partes a Google Inc., 2014–2015,
com aviso de permissão e disclaimer. A cópia `1.19.0` examinada atualmente
identifica Brian Smith e modificações Amazon, com declarações SPDX `ISC` e
`Apache-2.0 OR ISC`; essa diferença de cabeçalhos requer avaliação apropriada,
sem presumir alteração indevida ou determinar obrigações por si só.

Como referência histórica, não como revisão comprovadamente usada, a última
alteração do caminho BoringSSL anterior à data desse commit foi
`056035edcc2004c710baaee849c7b690a64a4b62`, de 2020-01-06. O [arquivo nessa
revisão](https://github.com/google/boringssl/blob/056035edcc2004c710baaee849c7b690a64a4b62/crypto/poly1305/poly1305_test.cc)
contém copyright 2015 Google Inc., permissão de uso e redistribuição com
preservação dos avisos e disclaimer no formato ISC. Não houve comparação
integral nem foi identificada a revisão BoringSSL efetivamente usada; os textos
e avisos pertinentes permanecem para avaliação por artefato. O teste está em
`#[cfg(test)]`; o grafo não prova inclusão em binário, e a crate não é
vendorizada na archive-fonte candidata Rust.

### Triagem adicional: prometheus

O índice registra uma ocorrência `origin` em `prometheus 0.14.0`,
`src/encoder/text.rs:258`. O SHA-256 local
`979e07b201bdd9cad92c992a02f2ecbea205678c7c22f6043974e6a4b92155c0`
coincide com o índice. O comentário atribui a implementação de `escape_string`
ao artigo `https://lise-henry.github.io/articles/optimising_strings.html`,
que retornou HTTP 404 na consulta desta etapa. Não foi identificada uma revisão
ou licença do material citado; origem e eventuais permissões devem ser
investigadas sem presumir cópia literal ou termos de distribuição. A triagem
prévia da licença da crate consta em `telemetry-examples/upstream-notices-review.md`;
esta classificação cobre somente a ocorrência indexada.

Na extração do exemplo separado de telemetria do candidato Rust `8avAZthl`,
`cargo tree --offline --locked --all-features --target x86_64-unknown-linux-gnu
-i prometheus --depth 2` selecionou `prometheus 0.14.0` diretamente. O grafo
não comprova inclusão do trecho em binário; a crate não é vendorizada na
archive-fonte candidata. A revisão por artefato permanece aberta; esta etapa
não abrange o candidato Go.

Investigação adicional: o endereço original do artigo redireciona na API do
GitHub ao repositório `crowdagger/lise-henry.github.io`. O arquivo
[`articles/optimising_strings.html` no commit `f98b049e2d784cb51edd0649d2394af8bd25b3c3`](https://github.com/crowdagger/lise-henry.github.io/blob/f98b049e2d784cb51edd0649d2394af8bd25b3c3/articles/optimising_strings.html),
de 2016-10-07, foi obtido com 269.848 bytes e SHA-256
`992f048fe7e2dcfe53d2c1a6f402d7fea38047f9580c4436f44b27123381053d`.
Seu título é *Optimising string processing in Rust* e o HTML identifica Lise
Henry como autora. O índice CDX da Internet Archive registra um snapshot HTTP
200 de 2017-01-09 para o endereço original; a resposta desse snapshot também
apresenta o artigo. Não foi comprovado qual versão foi consultada pelo autor de
`prometheus` nem feita comparação integral dos trechos de implementação.

Uma busca sem distinção de maiúsculas no HTML fixado por `licence`, `license`,
`copyright` e `creativecommons` não encontrou ocorrências; a consulta da API
de licença do repositório nessa revisão retornou HTTP 404. Isso limita-se aos
marcadores e fontes consultados, não demonstra inexistência de licença ou
permissão em outro lugar. Os avisos aplicáveis continuam para avaliação por
artefato; a crate não é vendorizada na archive-fonte candidata.

### Triagem adicional: smallvec

O índice registra uma ocorrência `origin` em `smallvec 1.16.4/src/lib.rs:2588`.
O SHA-256 local `c95ee991a165908b650df4a0e6473d357294137baa4bfff415e58f9bfacccec0`
coincide com o índice. O comentário em `SetLenOnDrop` atribui cópia ao [PR Rust
#36355](https://github.com/rust-lang/rust/pull/36355), integrado em 2016-09-12;
a API upstream identifica o commit de integração
`765700ba7a3743b9af5cb12092ea1293dbe07068`. Não foi feita comparação integral
do trecho nem concluída a revisão dos avisos aplicáveis. A classificação cobre
somente a ocorrência indexada; a triagem anterior dos textos de licença da crate
consta em `sqlite-rust-transitive-notices.md`.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i smallvec --depth 2`
selecionou `smallvec 1.16.4` via `rusqlite`, `hyper`, `icu_normalizer` e `idna`,
incluindo arestas de desenvolvimento. O grafo não prova inclusão do trecho em
binário; a crate não é vendorizada na archive-fonte candidata. A revisão por
artefato permanece aberta; esta etapa não abrange o candidato Go.

### Triagem adicional: opentelemetry

O índice registra uma ocorrência `origin` em `opentelemetry 0.33.1`,
`src/context.rs:1637`. O SHA-256 local
`fd9ab76c44c38aa9490cc29993eef51a990b40fcf4c12ebe17299f7cf562816f`
coincide com o índice. O comentário de teste descreve uma visualização derivada
do valor de um contexto, não origem ou cópia de código. A classificação cobre
somente a ocorrência indexada; os avisos upstream examinados anteriormente
constam em `telemetry-examples/upstream-notices-review.md`.

Na extração do exemplo Rust separado de telemetria do candidato `8avAZthl`,
`cargo tree --offline --locked --all-features --target x86_64-unknown-linux-gnu
-i opentelemetry --depth 2` selecionou `opentelemetry 0.33.1` diretamente e via
`opentelemetry_sdk`, esta última como dependência de desenvolvimento. O grafo
não prova inclusão do teste em binário; a crate não é vendorizada na archive-fonte
candidata. Esta etapa não examina o candidato Go nem conclui notices por artefato.

### Triagem adicional: typenum

O índice registra duas ocorrências `origin` em `typenum 1.20.1`,
`src/type_operators.rs`, linhas 105 e 146. O SHA-256 local
`c3ea12ffc00d1d438db095d48abae94af9dd46adf0b25c90871c1c4d6f15ae7b`
coincide com o índice. Ambas atribuem código copiado a `num::pow::pow`, sem
identificar versão ou revisão. A fonte específica e os avisos aplicáveis
permanecem para investigação; a classificação cobre somente os trechos
indexados e não esgota os demais arquivos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i typenum --depth 2`
selecionou `typenum 1.20.1` via `crypto-common`, `generic-array` e
`hybrid-array`. O grafo não prova inclusão dos trechos em binário; a crate não é
vendorizada na archive-fonte candidata. A revisão por artefato permanece aberta;
esta etapa não abrange o candidato Go.

Investigação adicional: `.cargo_vcs_info.json` de `typenum 1.20.1` aponta ao
commit `0db9a0f731981f29266b63586c29fa07e4477b1a`. No histórico de
`src/type_operators.rs`, o [commit
`37b20d6d680a03240cc94db0a996495d23ef7912`](https://github.com/paholg/typenum/commit/37b20d6d680a03240cc94db0a996495d23ef7912),
de 2016-10-30, adiciona um comentário `copied from num::pow::pow`; o [commit
`33175142ad8da2b84ba120a49a241ced8438bb81`](https://github.com/paholg/typenum/commit/33175142ad8da2b84ba120a49a241ced8438bb81)
de 2016-11-08 amplia as implementações e adiciona outra ocorrência. Nenhum dos
comentários examinados fixa a revisão da fonte `num`.

Como referência histórica, não como versão comprovadamente usada, a archive
`num 0.1.36`, publicada em 2016-09-30, tem SHA-256
`bde7c03b09e7c6a301ee81f6ddf66d7a28ec305699e3d3b056d2fc56470e3120`,
declara `MIT/Apache-2.0` em seu manifesto, inclui `LICENSE-MIT` e
`LICENSE-APACHE` e reexporta `pow` de `num-traits`. Seu manifesto referencia
`num-traits 0.1.36`, archive de SHA-256
`a16a42856a256b39c6d3484f097f6713e14feacd9bfb02290917904fae46c81c`,
igual ao checksum informado pela [API crates.io](https://crates.io/api/v1/crates/num-traits/0.1.36).
O manifesto declara `MIT/Apache-2.0`; `src/pow.rs` contém implementação por
quadrados sucessivos semelhante à atribuída em `typenum`. Não foi feita
comparação integral nem comprovado que essas versões foram consultadas ou são
as únicas possíveis. A aplicabilidade e preservação dos avisos devem ser
avaliadas conforme os componentes efetivamente distribuídos; isso não altera
os limites do grafo ou da archive-fonte candidata descritos acima.

### Triagem adicional: icu_normalizer

O índice registra quatro ocorrências `origin` em `icu_normalizer 2.4.0`,
`src/lib.rs`, linhas 2144, 2160, 2179 e 2819. O SHA-256 local
`0c44d2a692cc7d526121f62e86dfdfb14d31471d9155a34e3ff5a4c8aebc1106`
coincide com o índice. Os comentários descrevem ponteiros derivados da mesma
alocação, não origem ou cópia de código. A classificação cobre apenas essas
linhas indexadas e não esgota os arquivos ou avisos da crate.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i icu_normalizer --depth 2`
selecionou `icu_normalizer 2.4.0` via `idna_adapter` e `idna`. O grafo não prova
inclusão do arquivo em binário; a crate não é vendorizada na archive-fonte
candidata. A revisão de notices por artefato permanece aberta; esta etapa não
abrange o candidato Go.

### Triagem adicional: icu_provider

O índice registra uma ocorrência `origin` em `icu_provider 2.4.0`,
`src/marker.rs:251`. O SHA-256 local
`aa9a396e0daa9b7bb537ecb30a2f5a2a8dc82c645140fac02f26e04cb64a0e1f`
coincide com o índice. O comentário atribui adaptação a `rust-lang/rustc-hash`
sem identificar revisão; o trecho preserva copyright 2015 The Rust Project
Developers e uma declaração de Apache 2.0 ou MIT. A revisão original e a
aplicabilidade dos avisos ao artefato permanecem para avaliação. Essa triagem
cobre somente a ocorrência indexada, não todos os avisos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i icu_provider
--depth 2` selecionou `icu_provider 2.4.0` via `icu_normalizer` e
`icu_properties`. O grafo não prova inclusão do trecho em binário; a crate não
é vendorizada na archive-fonte candidata. Esta etapa não abrange o candidato Go.

### Triagem adicional: aws-smithy-json

O índice registra duas ocorrências `origin` em um arquivo de teste de
`aws-smithy-json 0.65.1`, `tests/sep_document_round_trip.rs`. O SHA-256 local
`5eaffa5c4606c09611014e68ec8ed438d869b795155dce2455c0966a20ea690c`
coincide com o índice. As linhas 460 e 690 descrevem documentos derivados de
formas tipadas, não origem ou cópia de código. A classificação cobre apenas
esses trechos indexados e não esgota os arquivos ou avisos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i aws-smithy-json
--depth 2` selecionou `aws-smithy-json 0.65.1` via `aws-sdk-s3`, incluindo
aresta de desenvolvimento. O grafo não comprova inclusão do teste em binário;
a crate não é vendorizada na archive-fonte candidata. A revisão de notices por
artefato permanece aberta; esta etapa não abrange o candidato Go.

### Triagem adicional: aws-smithy-http-client

O índice registra uma ocorrência `origin` em `aws-smithy-http-client 1.5.2`,
`src/client/pool/admission.rs:675`. O SHA-256 local
`6fcedcf6f6dcb5a413db9c6a4da097b132a445e3b2294b1f5113c465a7602dd9`
coincide com o índice. O comentário descreve índices de protocolo derivados de
uma programação de demanda, não origem ou cópia de código. Essa classificação
cobre apenas a ocorrência indexada e não esgota os avisos da crate.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i aws-smithy-http-client
--depth 2` selecionou a versão `1.5.2` via `aws-smithy-runtime` e `aws-sdk-s3`,
incluindo arestas de desenvolvimento. O grafo não prova que o arquivo integre
um binário; a crate não é vendorizada na archive-fonte candidata. A revisão
por artefato permanece aberta; o candidato Go não foi examinado nesta etapa.

### Triagem adicional: aws-sigv4

O índice registra uma ocorrência `origin` em `aws-sigv4 1.6.4/src/sign/v4a.rs`,
linha 376. O SHA-256 local
`6cb2384c584729d4d22e68d6da8765be38263b0faf27e9049524b789ab6a0f4d`
coincide com o índice. O trecho descreve uma chave derivada de outro valor em
testes, não atribuição de origem ou cópia de código. Essa classificação cobre
somente a ocorrência indexada e não esgota os arquivos ou avisos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i aws-sigv4
--depth 2` selecionou `aws-sigv4 1.6.4` via `aws-runtime` e `aws-sdk-s3`,
incluindo arestas de desenvolvimento. O grafo não comprova inclusão do trecho
em binário; a crate não é vendorizada na archive-fonte. A revisão por artefato
continua aberta; esta etapa não abrange o candidato Go.

### Triagem adicional: cc

O índice registra duas ocorrências `origin` em `cc 1.11.0/src/lib.rs`. O
SHA-256 local `8d99d321bc3e5b6738677c7d1bcf64d9352d93b353ff07019a112b9e5aa01616`
coincide com o índice. O comentário na linha 2811 aponta código copiado do
[compilador Rust no commit `5db81020006d2920fc9c62ffc0f4322f90bffa04`](https://github.com/rust-lang/rust/blob/5db81020006d2920fc9c62ffc0f4322f90bffa04/compiler/rustc_codegen_ssa/src/back/linker.rs#L27-L38).
O comentário na linha 5522 atribui outro trecho a um arquivo de `rustc`, sem
identificar revisão. São declarações de origem nos fontes; não foram comparados
integralmente os trechos ou concluída a análise de seus avisos. A evidência
anterior dos textos de licença da crate consta em `sqlite-rust-transitive-notices.md`.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i cc --depth 2` selecionou
`cc 1.11.0` como dependência de compilação de `aws-lc-sys` e `libsqlite3-sys`,
inclusive via `cmake`. Uso na compilação não prova inclusão da crate ou dos
trechos citados no binário final; ela não é vendorizada na archive-fonte
candidata. Esta etapa não cobre o candidato Go nem conclui notices por artefato.

### Triagem adicional: rustls-webpki

O índice registra duas ocorrências `origin` em um arquivo de
`rustls-webpki 0.103.16`, `src/subject_name/dns_name.rs`. O SHA-256 local
`c5989e0e46973d8d6a3e94608aad7964089f25599c1f326a6ac30f85204c8328`
coincide com o índice. Em um módulo `#[cfg(test)]`, o comentário na linha 633
atribui casos adaptados a `x509_certificate_unittest.cc` do Chromium e descreve
modificações; o da linha 769 atribui exemplos à RFC 6125, seção 6.4.3. Os
comentários não identificam revisão do Chromium. As fontes originais e os avisos
aplicáveis aos testes não foram revisados integralmente nesta etapa; esta
classificação cobre apenas as ocorrências indexadas.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i rustls-webpki
--depth 2` selecionou `rustls-webpki 0.103.16` via `rustls`. O grafo não prova
inclusão dos testes em binário; a crate não é vendorizada na archive-fonte
candidata. A revisão de notices por artefato permanece aberta; esta etapa não
abrange o candidato Go.

### Triagem adicional: zerovec

O índice registra uma ocorrência `origin` em `zerovec 0.11.9`,
`src/varzerovec/owned.rs:321`. O SHA-256 local
`6ff83f0541bf3afea39e55d46538f38e4aab31880a4383eba0186d0c6a183c6d`
coincide com o índice. O comentário descreve ponteiros derivados de uma referência
mutável, não uma atribuição de origem ou cópia de código. A classificação cobre
apenas a ocorrência indexada e não esgota os arquivos ou avisos da crate.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i zerovec --depth 2` selecionou
`zerovec 0.11.9` via dependências ICU e `tinystr`. O grafo não prova inclusão
do arquivo em binário; a crate não é vendorizada na archive-fonte candidata. A
revisão de notices por artefato permanece aberta; o candidato Go não foi
examinado nesta etapa.

### Triagem adicional: rand_core

O índice registra uma ocorrência `origin` em `rand_core 0.10.1/src/lib.rs:185`.
O SHA-256 local `b4df43985896bf2f38e595f7574c4d7e0df1c562e40c6a92133d9eef1cc01908`
coincide com o índice. O trecho descreve estado de um gerador derivado de uma
chave, não origem ou cópia de código. A classificação cobre somente a ocorrência
indexada; a evidência anterior sobre os textos de licença está registrada adiante.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i rand_core@0.10.1 --depth 2`
selecionou a crate via `crypto-bigint`, `elliptic-curve` e outras dependências,
incluindo arestas de desenvolvimento. O grafo não prova inclusão do arquivo em
binário; a crate não é vendorizada na archive-fonte candidata. A revisão dos
avisos por artefato permanece aberta; esta etapa não abrange o candidato Go.

### Triagem adicional: idna

O índice registra duas ocorrências `origin` em `idna 1.1.0/src/uts46.rs`, nas
linhas 663 e 717. O SHA-256 local
`9726647c01f315215eca25b7cb706929ff829269d4143de06b4f4718538b6640`
coincide com o índice. Os dois trechos descrevem cálculos derivados de
argumentos de uma função, não atribuições de origem ou cópia de código. A
classificação cobre somente essas ocorrências indexadas, não todos os arquivos
ou avisos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i idna --depth 2`
selecionou `idna 1.1.0` via `url 2.5.8` e `aws-sdk-s3`, incluindo aresta de
desenvolvimento. O grafo não prova inclusão do arquivo em binário; a crate não
é vendorizada na archive-fonte candidata. A revisão por artefato permanece
aberta; esta etapa não abrange o candidato Go.

### Triagem adicional: url

O índice registra uma ocorrência `origin` em `url 2.5.8/tests/wpt.rs:145`.
O SHA-256 local `2d083ab72ad8cc9b116dd164e63440dd120f07977ac306377633cfe214f34a62`
coincide com o índice. O comentário `Copied from` aponta ao diretório `url` do
repositório [Web Platform Tests](https://github.com/web-platform-tests/wpt/tree/master/url),
em uma branch móvel, imediatamente antes da leitura de `urltestdata.json`. O
trecho não identifica revisão ou a extensão exata do material copiado; os
dados e avisos upstream não foram comparados integralmente nesta etapa. O
manifesto da crate declara `MIT OR Apache-2.0`; essa declaração não substitui
a revisão das fontes e dos avisos aplicáveis aos testes e dados distribuídos.
A classificação cobre somente a ocorrência indexada.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i url --depth 2`
selecionou `url 2.5.8` via `aws-sdk-s3`, incluindo aresta de desenvolvimento.
O grafo não prova inclusão dos testes ou dados em binário; a crate não é
vendorizada na archive-fonte candidata. A revisão por artefato permanece aberta;
esta etapa não abrange o candidato Go.

Investigação adicional dos dados: `url 2.5.8/tests/urltestdata.json` identifica
na primeira entrada o [commit WPT
`befe66343e5f21dc464c8c772c6d20695936714f`](https://github.com/web-platform-tests/wpt/blob/befe66343e5f21dc464c8c772c6d20695936714f/url/resources/urltestdata.json).
O SHA-256 do JSON local é
`58d67bea710d5f46324fe6841df5fd82090fe4ec2d882bc0fc7c1784d4771884`;
o arquivo obtido desse commit tem SHA-256
`3f71e188f21f6cc0bbc25c7f6cc997fe2847de6893262c8bee539778e46117d2`.
Os arquivos não são idênticos: comparando somente objetos JSON como multiconjuntos
com chaves ordenadas, os 819 objetos locais constam entre os 820 upstream; há um
objeto upstream adicional. A comparação não cobre equivalência das entradas de
texto nem atribui origem a outros dados de teste.

O [`LICENSE.md` na mesma revisão WPT](https://github.com/web-platform-tests/wpt/blob/befe66343e5f21dc464c8c772c6d20695936714f/LICENSE.md)
contém BSD de três cláusulas e credita os contribuidores de web-platform-tests.
O texto inclui condições para redistribuição de fonte e binário e disclaimer;
sua aplicabilidade aos dados e aos artefatos efetivamente distribuídos requer
avaliação apropriada, inclusive de eventuais avisos específicos não examinados.
Esses dados pertencem à crate no cache, não à archive-fonte Rust candidata;
seleção no grafo não prova inclusão de testes ou dados em binário.

### Triagem adicional: tower

O índice registra uma ocorrência `origin` em `tower 0.5.3`,
`src/load/peak_ewma.rs:38`. O SHA-256 local
`24a5e76918466916009a981393383835bdf51081199705529e35a3f2c773222e`
coincide com o índice. O comentário atribui a derivação a Finagle, declara
Apache V2 e credita Twitter Inc., 2017; aponta para
[`PeakEwma.scala` no commit `9cc08d15216497bb03a1cafda96b7266cfbbcff1`](https://github.com/twitter/finagle/blob/9cc08d15216497bb03a1cafda96b7266cfbbcff1/finagle-core/src/main/scala/com/twitter/finagle/loadbalancer/PeakEwma.scala).
Na mesma revisão upstream, `LICENSE` contém Apache License 2.0 e `NOTICE` credita
Twitter Inc. e menciona terceiros. Não foi feita comparação integral dos
arquivos nem determinada a aplicabilidade desses avisos à distribuição; a
classificação cobre apenas a ocorrência indexada.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i tower --depth 3` selecionou
`tower 0.5.3` via `aws-smithy-http-client`. A consulta de features no exemplo
separado de telemetria para o mesmo alvo mostrou somente `tower/default`; isso
não estabelece as features de outras composições nem o conteúdo de um binário.
A crate não é vendorizada na archive-fonte candidata. A revisão de notices por
artefato permanece aberta; o candidato Go não foi examinado nesta etapa.

### Triagem adicional: base64ct

O índice registra uma ocorrência `origin` em `base64ct 1.8.3/src/lib.rs:66`.
O SHA-256 local `ebb6d9fe0d1199bb6aefaecb4e44524358c99d0db8d82e39b01a0b27f084b087`
coincide com o índice. O cabeçalho atribui adaptação à implementação C++
`ConstTimeEncoding/base64.cpp`, credita Steve "Sc00bz" Thomas em 2014 e afirma
que o código derivado é licenciado sob MIT e Apache 2 com permissão do autor.
O [arquivo público atualmente citado](https://github.com/Sc00bz/ConstTimeEncoding/blob/master/base64.cpp)
contém copyright do mesmo autor e texto de permissão MIT com condição de
preservação do aviso e disclaimer. A referência aponta a uma branch móvel e
não identifica a revisão usada; tampouco foi verificada independentemente a
permissão mencionada por `base64ct`. Essa evidência não conclui os avisos
aplicáveis por artefato, nem cobre todos os arquivos da crate.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i base64ct --depth 2`
selecionou `base64ct 1.8.3` via `pem-rfc7468`, com caminhos para `der` e
`elliptic-curve`. O grafo não prova inclusão do trecho num binário; a crate não
é vendorizada na archive-fonte. O candidato Go não foi examinado nesta etapa.

### Triagem adicional: rustls-pki-types

O índice registra duas ocorrências `origin` em `rustls-pki-types 1.15.1`,
`src/server_name.rs`. O SHA-256 local
`4e15ee1510f1514963b264eeeead70a8db27911e37157e5b52ab816f37208223`
coincide com o índice. Na linha 518, o comentário atribui adaptação a
`std::net::Ipv6Addr::new()` sem fixar revisão. Nas linhas 557–559, atribui o
parser a `core` e aponta ao [commit Rust
`fc13ca6d70f7381513c22443fc5aaee1d151ea45`](https://github.com/rust-lang/rust/blob/fc13ca6d70f7381513c22443fc5aaee1d151ea45/library/core/src/net/parser.rs#L34).
O comentário é evidência declarada de origem; não foi realizada comparação
integral nem revisão dos avisos aplicáveis. A classificação cobre apenas as
ocorrências indexadas, não todos os arquivos da crate.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i rustls-pki-types --depth 2`
selecionou `rustls-pki-types 1.15.1` via `rustls`, `aws-smithy-http-client`,
`rustls-native-certs` e `rustls-webpki`. O grafo não prova inclusão do arquivo
em binário; a crate não é vendorizada na archive-fonte candidata. A revisão por
artefato continua aberta; o candidato Go não foi examinado nesta etapa.

### Triagem adicional: time

O índice registra uma ocorrência `origin` em `time 0.3.55`,
`src/signed_duration.rs:110`. O SHA-256 local
`8b63e5a85f81c20ecd4f101d1501bb9829d13b1869fde2eacc38b43113a982e6`
coincide com o índice. O comentário atribui a macro `try_from_secs` à
implementação da biblioteca padrão Rust e informa que alterações estão marcadas
no código; aponta ao [arquivo no commit
`3a37c2f0523c87147b64f1b8099fc9df22e8c53e`](https://github.com/rust-lang/rust/blob/3a37c2f0523c87147b64f1b8099fc9df22e8c53e/library/core/src/time.rs#L1262-L1340).
O arquivo upstream consultado contém uma macro de mesmo nome no trecho citado;
não foi feita comparação integral nem concluída a revisão dos avisos aplicáveis.
Esta classificação cobre somente a ocorrência indexada, não toda a crate.

No workspace extraído do candidato Rust `8avAZthl`, `cargo tree --offline
--locked --workspace --all-features --target x86_64-unknown-linux-gnu -i time
--depth 2` selecionou `time 0.3.55` via `aws-sigv4` e `aws-smithy-types`,
incluindo arestas de desenvolvimento. O grafo não comprova inclusão do trecho
em binário; a crate não é vendorizada na archive-fonte. A avaliação de notices
por artefato permanece aberta; o candidato Go não foi examinado nesta etapa.

### Triagem adicional: group

O índice registra uma ocorrência `origin` em `group 0.14.0/src/lib.rs:91`. O
SHA-256 local `254cc0466b709dffedd01c0c0eb59bdae220e8fd1c9a9612d1a71f65d531c2b3`
coincide com o índice. O comentário descreve um método derivado de outro método
da mesma interface, não uma atribuição de origem ou cópia de código de terceiros.
Essa classificação cobre somente a linha indexada; os textos de licença e o
arquivo `COPYRIGHT` já examinados estão registrados adiante neste documento.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i group --depth 2`
selecionou `group 0.14.0` via `elliptic-curve` e `wnaf`. O grafo não prova
inclusão do arquivo em binário; a crate não é vendorizada na archive-fonte
candidata. Esta etapa não abrange o candidato Go nem conclui notices por artefato.

### Triagem adicional: ff

O índice registra uma ocorrência `origin` em `ff 0.14.0/src/lib.rs:91`. O
SHA-256 local `d67e055f5375cf86198e4947a0c8b15df117919924a8bfec790a2a7ae7d06eac`
coincide com o índice. O comentário descreve um método derivado de outro método
da mesma interface, não uma atribuição de origem ou cópia de código de terceiros.
Esta classificação cobre apenas a linha indexada e não esgota os avisos da crate;
os textos de licença examinados anteriormente estão registrados adiante.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i ff --depth 2`
selecionou `ff 0.14.0` via `elliptic-curve`, `group`, `primefield` e `wnaf`.
O grafo não prova inclusão do arquivo em binário; a crate não é vendorizada
na archive-fonte candidata. Esta etapa não abrange o candidato Go nem conclui
notices por artefato.

### Triagem adicional: der

O índice registra uma ocorrência `origin` em `der 0.8.3/src/datetime.rs:4`.
O SHA-256 local `51d74f979a2be2440c9f7cf1ce28c74154e91f02cf5f7552bc4b077144d3f3b7`
coincide com o índice. O cabeçalho atribui adaptação à crate `humantime`,
credita The humantime Developers em 2016 e declara MIT ou Apache 2.0; não
identifica a revisão original. A origem precisa e os textos e avisos aplicáveis
permanecem para revisão. Essa classificação cobre a ocorrência indexada, não
todos os arquivos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i der --depth 2`
selecionou `der 0.8.3` via `ecdsa`, `pkcs8`, `sec1` e `spki`. O grafo não
prova inclusão do arquivo em binário; a crate não é vendorizada na archive-fonte
candidata. Esta etapa não abrange o candidato Go nem conclui notices por artefato.

### Triagem adicional: pkcs8

O índice registra cinco ocorrências `origin` em dois arquivos de `pkcs8 0.11.0`.
Os hashes locais coincidem com os registros: `src/encrypted_private_key_info.rs`
tem SHA-256 `00091a4d02fe196fcc8fdfee5dc9bb7d6ec6fdc09940ff958033df2613cc3153`;
`src/private_key_info.rs` tem SHA-256
`9f4328fd93211c0b4393df175dba47a43f42dec9a643e0d6830b1ef72aebdf00`.
Os trechos descrevem chaves de criptografia derivadas de uma senha, não
declarações de origem ou cópia de código. Essa classificação cobre somente as
linhas indexadas e não esgota as fontes ou avisos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i pkcs8 --depth 2`
selecionou `pkcs8 0.11.0` via `elliptic-curve`, com caminhos para `p256`,
`ecdsa` e `primeorder`. O grafo não prova que os trechos integrem um binário;
a crate não é vendorizada na archive-fonte candidata. A revisão de notices por
artefato permanece aberta; esta etapa não abrange o candidato Go.

### Triagem adicional: spin

O índice registra duas ocorrências `origin` em `spin 0.10.1`; os hashes locais
coincidem com os registrados:

| Arquivo | SHA-256 | Referência examinada |
|---|---|---|
| `src/barrier.rs:4` | `f8569a601d870a713344747eb355e6d840db510a599709173ebd0ccba5fc4028` | Adaptação declarada de `std::sync::Barrier`; o cabeçalho credita The Rust Project Developers, 2014, e oferece Apache 2.0 ou MIT. |
| `src/lazy.rs:3` | `97cb5ad2c11c5c7402f001233b091a9c4e3c450565a1f17fb73cf8114d08480e` | Adaptação declarada de `SyncLazy` da biblioteca padrão Rust. |

Os comentários examinados não identificam revisões das fontes Rust. O manifesto
de `spin` declara MIT; isso não substitui revisão dos avisos preservados nos
arquivos e de sua aplicabilidade aos artefatos escolhidos. A análise cobre apenas
as ocorrências indexadas, não os demais arquivos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -e features -i
spin --depth 3` selecionou `spin 0.10.1` via `crc-fast`, mostrando features
`mutex`, `once`, `rwlock` e `spin_mutex`, não `barrier` nem `lazy`. Os módulos
citados são condicionados às respectivas features no `src/lib.rs`. Isso vale
somente para o grafo consultado e não prova conteúdo de binário; outro consumidor
pode selecionar outras features. A crate não é vendorizada na archive-fonte
candidata; esta etapa não abrange o candidato Go.

### Triagem adicional: sha2 Rust

O índice registra duas ocorrências `origin` em `sha2 0.11.1`. Os hashes das
cópias locais coincidem com os do índice:

| Arquivo | SHA-256 | Referência examinada |
|---|---|---|
| `src/sha256/aarch64_sha2.rs:3` | `4bddcedd5bb92a3b3768880ec3d077d7495b95c238aab3150ced25cc905a1826` | Implementação atribuída a adaptação de mbedTLS. |
| `src/sha512/aarch64_sha3.rs:1` | `8dcc738182d27f7b43dd3ec35487ca905119e978f920454921663582fe59c777` | Implementação atribuída a adaptação de mbedTLS. |

Os comentários não identificam arquivo ou revisão mbedTLS de origem. As
permissões e avisos aplicáveis aos trechos permanecem para investigação e
avaliação apropriada; a classificação abrange apenas os trechos indexados.
Ambos os arquivos contêm `compile_error!` para alvos que não sejam AArch64.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i sha2@0.11.1
--depth 2` selecionou a crate pelo core, adapter S3 e dependências como
`aws-sdk-s3`, incluindo arestas de desenvolvimento. O grafo não prova que os
backends AArch64 sejam compilados ou incluídos em binário; outras configurações
podem selecioná-los. A crate não é vendorizada na archive-fonte candidata, e
esta etapa não abrange o candidato Go.

Investigação histórica adicional: `.cargo_vcs_info.json` de `sha2 0.11.1`
aponta ao commit `138dc5a05b93f8e49cda177fc6cfd7cc43673e5d` do repositório
RustCrypto/hashes. O histórico dos caminhos, incluindo suas renomeações em
2025 e 2026, foi rastreado até o [commit
`3fa561ed2eb584c215cb4793b5c708b5ed238216`](https://github.com/RustCrypto/hashes/commit/3fa561ed2eb584c215cb4793b5c708b5ed238216),
de 2023-06-15. O diff desse commit adiciona a atribuição `Implementation adapted
from mbedtls` aos dois backends, então chamados `src/sha256/aarch64.rs` e
`src/sha512/aarch64.rs`. A descrição do [PR
#490](https://github.com/RustCrypto/hashes/pull/490) apresenta os backends, mas
não identifica arquivo ou revisão mbedTLS. A consulta aos comentários gerais do
PR não retornou menções a mbedTLS ou licença; a consulta aos comentários de
revisão recebeu HTTP 403 por limite da API e permanece incompleta. Não foi
identificada a revisão original nem confirmado o aviso aplicável a cada trecho.
O resultado não altera os limites do grafo Linux x86-64 e da archive-fonte
candidata descritos acima.

### Triagem adicional: p256 Rust

Os três arquivos `p256 0.14.0` marcados como `origin` no índice tiveram hashes
locais coincidentes com os registros:

| Arquivo | SHA-256 | Referência examinada |
|---|---|---|
| `src/arithmetic.rs:40` | `673d5015a4ee5edcd4c2c4bfa8d9c4ff29fc473218c148c678c1912ccddf0e60` | Parâmetros da curva atribuídos a NIST SP 800-186, seção G.1.2. |
| `src/ecdsa.rs:132` | `07c218c59e9a4ae4b8256cc3d69ac99e837472da82f3d8a45bfd228d332971f9` | Vetor em teste atribuído aos vetores FIPS 186-4 ECDSA, arquivo `SigGen.txt`. |
| `src/test_vectors/ecdsa.rs:8` | `607d74f05e1ed43d2aa387af6e6b665ec9b66a82e291630b18642ca1032af684` | Vetores atribuídos à mesma coleção FIPS 186-4; a documentação descreve uma transformação de prehash. |

O `src/lib.rs` condiciona o módulo `test_vectors` a `test` ou à feature
`test-vectors`. As referências são declarações dos fontes, não conferência
integral dos documentos NIST, dos vetores originais ou de seus eventuais avisos.
Esta classificação cobre apenas as ocorrências indexadas e não determina
obrigações aplicáveis a uma distribuição.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i p256 --depth 2`
selecionou `p256 0.14.0` via `aws-sigv4` e `aws-sdk-s3`, incluindo arestas de
desenvolvimento. O grafo não prova inclusão dos parâmetros ou vetores em um
binário; a crate não está vendorizada na archive-fonte. O candidato Go não foi
examinado nesta etapa, e a revisão por artefato permanece aberta.

### Triagem adicional: crypto-bigint

Os três arquivos `crypto-bigint 0.7.10` marcados como `origin` no índice foram
conferidos; seus SHA-256 locais coincidem com os registros:

| Arquivo | SHA-256 | Atribuição no trecho examinado |
|---|---|---|
| `src/modular/boxed_monty_form/mul.rs` | `d1803be45c8271640387c60ea2ab0a20fc798f866e37f240a79b2934e35e0133` | Partes adaptadas de `num-bigint` commit abreviado `2cea7f4`; credita The Rust Project Developers, 2014, e declara licenciamento Apache 2.0 e MIT. |
| `src/modular/safegcd.rs` | `87e43dfd7950acc9c710bd1596263f4d153f5b467888f2422646c3c99f4cda0b` | Atribui adaptação a implementações ligadas a dois PRs `halo2curves`, informa Apache 2.0 e MIT e credita Privacy Scaling Explorations Team, 2023. |
| `src/uint/ref_type/sqrt.rs` | `85e8670802b0db249f646e0ad68a11261777a295a9ac67e7bc5d84365775aa13` | Atribui adaptação à figura 11-4 de *Hacker's Delight*, 2ª edição, de Henry S. Warren Jr., baseada no algoritmo de Toepler. |

Estas são declarações dos fontes locais. As revisões e textos das fontes
originais, especialmente as permissões para qualquer material da obra citada,
não foram verificados nesta etapa. Não se infere cópia literal, autorização ou
obrigações por artefato apenas do comentário; os avisos requerem avaliação
apropriada. A classificação cobre as ocorrências indexadas, não toda a crate.

No workspace extraído do candidato Rust `8avAZthl`, `cargo tree --offline
--locked --workspace --all-features --target x86_64-unknown-linux-gnu -i
crypto-bigint --depth 2` selecionou `crypto-bigint 0.7.10` via `aws-sigv4`,
`elliptic-curve`, `primefield` e `rfc6979`, incluindo arestas de desenvolvimento.
O grafo não comprova a compilação dos trechos ou conteúdo de binário. A crate
não é vendorizada na archive-fonte; esta etapa não abrange o candidato Go.

Investigação adicional das fontes citadas: a API de `rust-num/num-bigint`
resolve o prefixo `2cea7f4` ao commit
[`2cea7f47938d2fc3708e6ad06db6b580c14c46a8`](https://github.com/rust-num/num-bigint/tree/2cea7f47938d2fc3708e6ad06db6b580c14c46a8);
seu `Cargo.toml` declara `MIT OR Apache-2.0` e identifica The Rust Project
Developers. Os PRs citados de `halo2curves`,
[#2](https://github.com/taikoxyz/halo2curves/pull/2) e
[#83](https://github.com/privacy-scaling-explorations/halo2curves/pull/83),
foram integrados respectivamente em
`ba112ff9c23093a4a87759bc82c413a5fd2a00dd` e
`a83509660676f252d8534684fcc718603e72b1ee`; os manifests dessas revisões
declaram literalmente `MIT/Apache-2.0`. A descrição do segundo PR identifica o
primeiro como origem da contribuição. Esses metadados não substituem a leitura
dos textos e avisos pertinentes, nem demonstram identidade integral do código.

Quanto a *Hacker's Delight*, o README do repositório público
[`hcs0/Hackers-Delight`](https://github.com/hcs0/Hackers-Delight) descreve seu
conteúdo como código acompanhante da segunda edição. O arquivo
[`isqrt.c.txt` no commit `bb02339a08c5ac2fc4da2a63e6439f2629d95774`](https://github.com/hcs0/Hackers-Delight/blob/bb02339a08c5ac2fc4da2a63e6439f2629d95774/isqrt.c.txt)
contém `isqrt4`, um algoritmo de hardware semelhante ao trecho Rust atribuído
à figura 11-4, com diferenças; não se comprovou que esse arquivo seja a fonte
usada. Não foi encontrada declaração de licença no README ou nesse arquivo
examinados; isso não prova inexistência de permissão em outro lugar. A revisão
da origem e dos direitos desse material permanece aberta. O grafo Rust não
prova inclusão dos trechos em binário, e a crate não é vendorizada no candidato.

### Triagem adicional: libc Rust

O índice contém duas ocorrências `origin` em `libc 0.2.191`. Os hashes locais
coincidem com os registros: `src/unix/linux_like/mod.rs`, SHA-256
`39afa8f72bdb7f6afa0dc9bed7e35ae0e78015dcbe520809a6324b8e58f3ca3c`,
e `src/unix/linux_like/linux/uclibc/arm/mod.rs`, SHA-256
`53adee09775166670dad2bfaff213c958f31363c605e5671911e69e48ae41836`.
O primeiro atribui código ioctl ao [arquivo do Linux no commit
`8a696a29c6905594e4abf78eaafcb62165ac61f1`](https://github.com/torvalds/linux/blob/8a696a29c6905594e4abf78eaafcb62165ac61f1/rust/kernel/ioctl.rs).
O cabeçalho do arquivo upstream declara literalmente `SPDX-License-Identifier:
GPL-2.0`; a função `_IOC` está presente em ambos, com diferenças. Não foi
feita comparação integral nem determinada a aplicação jurídica desse aviso à
adaptação. O segundo comentário diz que uma tabela de syscalls foi copiada de
um caminho `musl/b32/arm.rs`; não identifica revisão nem prova, por si, a
proveniência anterior dessa tabela. Estas observações cobrem somente as duas
ocorrências indexadas, não os demais avisos da crate.

No workspace extraído do candidato Rust `8avAZthl`, `cargo tree --offline
--locked --workspace --all-features --target x86_64-unknown-linux-gnu -i libc
--depth 1` selecionou `libc 0.2.191`, inclusive por `getrandom`, `tokio` e o
core, com dependências de compilação e desenvolvimento. O grafo não prova
compilação dos trechos citados, especialmente o caminho ARM/uClibc, ou conteúdo
de binário. A crate não é vendorizada na archive-fonte candidata. As condições
aplicáveis exigem revisão apropriada por artefato; o Go não foi examinado.

### Triagem adicional: hyper

O índice registra uma ocorrência `origin` em `hyper 1.12.0`,
`src/proto/h2/client.rs:56`. O SHA-256 da cópia local,
`56d715d09cdeddb91606dd0a9cfaa3de360ef31c07179ab9df6d24d9da0868af`,
coincide com o índice. O comentário descreve um valor padrão derivado de uma
recomendação da especificação HTTP/2; não afirma cópia de código. Esta
classificação limita-se ao trecho indexado e não esgota os avisos da crate.

No workspace extraído do candidato Rust `8avAZthl`, `cargo tree --offline
--locked --workspace --all-features --target x86_64-unknown-linux-gnu -i hyper
--depth 2` selecionou `hyper 1.12.0` via `aws-smithy-http-client`,
`hyper-rustls` e `hyper-util`. A crate não é vendorizada na archive-fonte; o
grafo não comprova a inclusão do arquivo em binário. A revisão de notices por
artefato permanece aberta; o candidato Go não foi examinado nesta etapa.

### Triagem adicional: hyper-util

O índice registra uma ocorrência `origin` em `hyper-util 0.1.21`,
`src/client/legacy/client.rs:1603`. O SHA-256 da cópia local,
`bc854508f0bee926f93adf5c00b0ecd4387d3047d8f748b3571a2a72dcbbb8a1`,
coincide com o índice. O trecho documenta um cabeçalho HTTP `Host` derivado da
autoridade da URI; não é declaração de origem ou cópia de código. Esta
classificação cobre apenas a ocorrência indexada e não esgota os avisos da crate.

No workspace extraído do candidato Rust `8avAZthl`, `cargo tree --offline
--locked --workspace --all-features --target x86_64-unknown-linux-gnu -i
hyper-util --depth 2` selecionou `hyper-util 0.1.21` via
`aws-smithy-http-client 1.5.2` e `hyper-rustls 0.27.10`. A crate não é
vendorizada na archive-fonte e o grafo não prova que o arquivo integre um
binário. A revisão dos avisos depende da composição efetiva; o candidato Go não
foi examinado nesta etapa.

### Triagem adicional: hyper-rustls

O índice registra uma ocorrência `origin` em `hyper-rustls 0.27.10`,
`src/stream.rs:1`, cujo SHA-256 local
`e81a1d449d1b63728d33d81aa58bfa83fb39195f4c4fe4f35f7ada60e4706c78`
coincide com o índice. O comentário declara cópia de `hyperium/hyper-tls` em
`62e3376/src/stream.rs`. A API upstream resolve esse prefixo ao commit
`62e3376736a9d3693c916641e80fdd7e164d97e8`, de 2017-04-24. O
[arquivo nesse commit](https://github.com/hyperium/hyper-tls/blob/62e3376736a9d3693c916641e80fdd7e164d97e8/src/stream.rs)
contém `MaybeHttpsStream`; o arquivo atual possui diferenças de implementação.
O [manifesto histórico](https://github.com/hyperium/hyper-tls/blob/62e3376736a9d3693c916641e80fdd7e164d97e8/Cargo.toml)
declara literalmente `MIT/Apache-2.0`. Não foi feita comparação integral nem
revisão completa dos avisos do arquivo histórico ou da crate; não se escolheu
licença ou aprovou distribuição.

No workspace extraído do candidato Rust `8avAZthl`, `cargo tree --offline
--locked --workspace --all-features --target x86_64-unknown-linux-gnu -i
hyper-rustls --depth 2` selecionou `hyper-rustls 0.27.10` via
`aws-smithy-http-client 1.5.2`. A crate não é vendorizada na archive-fonte e o
grafo não prova que `stream.rs` integre um binário. A revisão de avisos depende
do artefato efetivo; o candidato Go não foi examinado nesta etapa.

### Triagem adicional: getrandom

As duas ocorrências `origin` do índice em `getrandom` estão em
`0.2.17/src/rdrand.rs:38` e `0.4.3/src/backends/rdrand.rs:53`. Os SHA-256
locais coincidem com os do índice, respectivamente
`ffbe1bfb8f5b30a95f462fa85db07e251f63248c6c0daf3b5f586034cedfa976` e
`013c90b04f882ea82a73cc82b8d282227e2cfe9fe7d4f8eae9020a1b17aeb3b1`.
Ambos os comentários atribuem a adaptação de um autoteste ao arquivo Linux
`arch/x86/kernel/cpu/rdrand.c`, sem especificar revisão. A origem exata e os
avisos aplicáveis ao trecho permanecem para revisão; isso não determina uma
obrigação ou licença por si só. A análise cobre apenas as ocorrências indexadas.

No workspace extraído do candidato Rust `8avAZthl`, consultas inversas
`cargo tree --offline --locked --workspace --all-features --target
x86_64-unknown-linux-gnu -i getrandom@<versão> --depth 2` selecionaram
`0.4.3` via `tempfile`, `uuid` e outras dependências, incluindo arestas de
desenvolvimento; para `0.2.17`, a consulta não retornou dependentes. Isso não
prova seleção do backend RDRAND ou conteúdo de binário, nem exclui seleção em
outras configurações. As crates não estão vendorizadas na archive-fonte
candidata; o candidato Go não foi examinado nesta etapa.

Investigação histórica adicional: o diff do commit getrandom
[`82c3cb890175b37644a5259d2f91d30770ba5af3`](https://github.com/rust-random/getrandom/commit/82c3cb890175b37644a5259d2f91d30770ba5af3),
de 2023-02-02, introduz o autoteste e a atribuição ao arquivo Linux no caminho
`src/rdrand.rs`. O histórico foi consultado até o commit
`b625985d852600a3eeb68556811e59e7c9a6a098`, indicado pela cópia `0.2.17` em
`.cargo_vcs_info.json`. Esse diff não especifica a revisão do kernel usada.

A consulta ao histórico do caminho Linux até a data desse commit retornou como
última alteração anterior o commit `049f9ae93d033be6758ad865a9b89650f9f075ec`,
de 2022-07-18. O [arquivo nessa revisão](https://github.com/torvalds/linux/blob/049f9ae93d033be6758ad865a9b89650f9f075ec/arch/x86/kernel/cpu/rdrand.c)
contém um teste de repetição com oito amostras; seu cabeçalho declara
`SPDX-License-Identifier: GPL-2.0-only` e copyright 2011 Intel Corporation.
Essa é uma referência histórica comparável, **não** prova de qual revisão foi
consultada ou de identidade integral dos trechos. O comentário em ambas as
versões de `getrandom` e esse aviso devem ser considerados na análise apropriada
de proveniência e distribuição; não se conclui aqui quais termos se aplicam à
adaptação ou ao artefato. A seleção de `getrandom 0.4.3` no grafo Linux não
estabelece compilação do backend RDRAND nem conteúdo do binário, e nenhuma das
duas crates está vendorizada na archive-fonte candidata.

### Triagem adicional: httparse

O índice `rust-embedded-notice-index.json` registra uma ocorrência `origin`
em `httparse 1.10.1/src/simd/neon.rs:82`. O SHA-256 da cópia local,
`fed66a5d37873dbf901ad2b69e9d08711c570a4a5fc958e13539808501ec9ac3`,
coincide com o índice. O comentário descreve adaptação de uma versão de
256 bits, com operações superiores comentadas; não identifica a fonte ou
revisão dessa versão. Não se conclui, só pelo comentário, que seja código
próprio ou de terceiros. A análise cobre apenas a ocorrência indexada.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i httparse --depth 2`
selecionou `httparse 1.10.1` via `hyper 1.12.0` e `hyper-util 0.1.21`.
Isso não prova compilação do arquivo NEON nem conteúdo de binário. A crate não
é vendorizada na archive-fonte candidata. A origem da versão de 256 bits e os
avisos aplicáveis permanecem para revisão conforme o artefato; esta análise não
abrange o candidato Go.

Investigação adicional: `.cargo_vcs_info.json` de `httparse 1.10.1` aponta
ao commit `9f29e79f9832dbd0ae5220acb17c1866745bdecd`. A consulta do
[histórico de `src/simd/neon.rs`](https://api.github.com/repos/seanmonstar/httparse/commits?path=src/simd/neon.rs&sha=9f29e79f9832dbd0ae5220acb17c1866745bdecd&per_page=100)
retornou sete commits; o mais antigo, `cdabb70b9ff20793c6c8629e6d65e18b44f741e4`,
adicionou o arquivo em 2023-04-25 já com o comentário. Na discussão de revisão
do [PR #133](https://github.com/seanmonstar/httparse/pull/133), uma resposta
explica que o validador de bitset de 128 bits resulta do truncamento de um
validador de bitset de 256 bits; outra diz que código inativo foi removido.
Isso esclarece o significado do comentário, mas não identifica uma revisão ou
autoria anterior do validador completo nem prova sua origem independente. O
escopo permanece limitado à consulta desse histórico e discussão; não encerra
a revisão dos avisos ou determina inclusão em qualquer artefato final.

### Consolidação dos candidatos locais examinados

Esta tabela reúne evidências já detalhadas neste documento e em
`native-notices-review.md`; não é uma lista completa de obrigações nem uma
aprovação de notices. Os caminhos identificam cópias de investigação, não os
artefatos finais de distribuição.

| Cópia e conteúdo verificado | Dependências selecionadas / avisos examinados externamente | Pendências específicas |
|---|---|---|
| Go `local-candidate-current-JIr2oOeH/go-copy`: fontes e fixtures; o manifesto passou. A busca por nomes relacionados a licença ou notice no manifesto não encontrou caminhos, o que não exclui avisos no conteúdo. | No Linux amd64, `modernc.org/sqlite v1.61.1` seleciona código gerado com atribuição a `rsc/fpfmt`; `modernc.org/libc v1.80.1` tem licença principal BSD de três condições e avisos de terceiros examinados separadamente. Essas dependências não são vendorizadas na cópia. | A revisão de origem e dos avisos da adaptação `fpfmt` permanece aberta; confrontar os textos e componentes efetivamente distribuídos, inclusive os avisos de `libc`. A seleção de fontes não comprova vinculação num binário. |
| Rust `local-candidate-current-8avAZthl/rust.tar.gz`: workspace, fixtures e documentação preliminar de SBOM/notices; o manifesto passou. O aviso incluído afirma que não há dependências de terceiros vendorizadas nem licença de projeto aprovada. A busca por nomes relacionados a notices encontrou oito caminhos de documentação SBOM, não uma certificação de completude. | Para `x86_64-unknown-linux-gnu`, os grafos examinados selecionam `rusqlite 0.41.1` → `libsqlite3-sys 0.39.1` no adapter SQLite, e `aws-smithy-types 1.9.1` → `base64-simd 0.8.0` → `vsimd 0.8.0` no adapter S3. Os manifests das duas crates SIMD declaram MIT; o SQLite examinado atribui uma adaptação a `fpfmt`. Esses textos e dependências foram examinados fora da archive candidata. | A revisão de `fpfmt` permanece aberta; o gist citado como inspiração por `vsimd` não apresentou licença explícita na revisão examinada, sem que isso prove cópia. Revisar os avisos segundo os componentes efetivos e demais terceiros; grafo de dependências não demonstra conteúdo de binário. |

Os resultados de testes e hashes correspondentes constam nas seções seguintes.
As anotações mais recentes desta matriz não estão automaticamente incluídas na
archive Rust já gerada. A definição de fontes, crates, exemplos, binários,
targets e features finais e a aprovação apropriada continuam necessárias antes
de concluir a revisão de distribuição; nenhum desses passos é autorizado pela
existência dos candidatos.

### Decisões e verificações necessárias para o conjunto final

O mantenedor precisa definir quais dos seguintes itens são efetivamente
candidatos: fonte Go, workspace Rust completo ou crates independentes, binários,
exemplos e fixtures; para binários ou crates, também os targets e features.
Esta escolha é separada de qualquer autorização para publicar ou distribuir.

Após essa definição, as verificações dependentes incluem listar e verificar os
arquivos de cada pacote extraído sem checkout adjacente; resolver dependências
locais e fixtures para crates independentes; gerar SBOMs do estado e alvo finais;
revisar os textos completos e avisos dos componentes efetivamente incluídos,
incluindo a questão de proveniência de `vsimd` quando aplicável; e obter a
aprovação apropriada para notices e distribuição. Binários exigem identificar
separadamente seus componentes nativos e características de build. Não se
infere conclusão desses itens dos testes ou inventários preliminares aqui.

### Comparação posterior com o checkout

No checkout em `22c8ba0`, conferiram-se os hashes dos arquivos registrados
nas cópias isoladas e compararam-se os nomes com `git ls-files --cached --others
--exclude-standard` para os mesmos escopos. Os 94 arquivos de `rust/` e
`conformance/fixtures/` da archive `f8313da0…b495131` e os 127 arquivos de
`go/` e fixtures da cópia Go coincidiram com o checkout; não havia nomes extras
ou ausentes nesses escopos. A comparação inclui alterações rastreadas e novos
arquivos não ignorados, não verifica arquivos ignorados e não abrange conteúdo
gerado, dependências do cache ou outros diretórios. Os documentos em
`docs/sbom/2026-11-05/` mudaram após a geração da archive Rust, portanto esta
comparação **não** afirma que a archive completa seja idêntica ao checkout atual.
Essas cópias continuam candidatas locais de investigação, sem aprovação de
notices ou distribuição.

## Candidato Rust local posterior

### Candidato Rust após teste público de contenção

Uma nova archive local, sem substituir as anteriores, foi criada em
`~/git-worktrees/sqlite-replication/sbom-refresh-20261106-evidence/local-candidate-current-8avAZthl/rust.tar.gz`.
Seu SHA-256 é `ea607522eb35a5962847bf829c5b3cec9ba3893977388647dbd5be662adc552f`.
Foram verificados 124 membros regulares, sem caminhos inseguros ou duplicados;
o manifesto passou após extração isolada. O teste offline do workspace com target
novo passou: core 69 aprovados e 8 ignorados, API 12 aprovados, coordinator 32
aprovados e spool 12 aprovados e 2 ignorados. O módulo separado de telemetria
passou quatro testes offline. O manifesto passou novamente após ambos os testes.
Os testes ignorados incluem drills CP-01 e cenários que exigem Floci ou binários
Go separados; o teste offline utilizou o cache de dependências local.
Esta anotação é posterior à archive e não integra seu
conteúdo; o candidato não é aprovação de notices ou distribuição.
Na extração desse candidato, `cargo tree --offline --locked --workspace
--all-features -i vsimd` confirmou `vsimd 0.8.0` via `base64-simd 0.8.0` e
`aws-smithy-types 1.9.1`, chegando a `sqlite-replication-storage-s3` e
`sqlite-replication-core`. A consulta equivalente com `-i libsqlite3-sys`
confirmou `libsqlite3-sys 0.39.1` via `rusqlite 0.41.1` até o adapter SQLite
e core; o grafo inclui também arestas de desenvolvimento. Estas consultas
descrevem dependências resolvidas da extração e não provam incorporação dos
trechos ou avisos num binário específico. A archive não vendoriza essas crates;
proveniência de `vsimd` e da adaptação `rsc/fpfmt` e os notices aplicáveis por
artefato permanecem pendentes. Nenhuma distribuição foi aprovada.
Para delimitar os componentes no candidato `8avAZthl`, consultas adicionais
`cargo tree --offline --locked -p <pacote> -e normal,build --target
x86_64-unknown-linux-gnu -i <dependência>` selecionaram `libsqlite3-sys
0.39.1` via `rusqlite 0.41.1` para `sqlite-replication-sqlite`, e `vsimd
0.8.0` via `base64-simd 0.8.0` e `aws-smithy-types 1.9.1` para
`sqlite-replication-storage-s3`. Essas consultas excluem arestas de teste e
são limitadas ao alvo e à configuração selecionados; não comprovam os objetos
presentes em binários futuros nem resolvem as atribuições de terceiros.
Na nova conferência dos manifests dos candidatos `8avAZthl` (Rust) e
`JIr2oOeH/go-copy` (Go), `sha256sum -c --quiet SHA256SUMS` passou em ambas as
extrações. Uma busca insensível a maiúsculas por `license`, `notice`, `copying`,
`copyright`, `fpfmt` ou `vsimd` nos caminhos dos manifests retornou oito
caminhos, todos sob `docs/sbom/2026-11-05/`, no candidato Rust, e nenhum na
cópia Go. São evidências de nomes de arquivos, não busca no conteúdo: não
estabelecem ausência de avisos embutidos, completude de notices ou aprovação.
As anotações neste documento posteriores à archive Rust não estão nela; a cópia
Go é composta por fontes Go e fixtures, não por um pacote final de distribuição.

### Nova geração local de investigação

Em `~/git-worktrees/sqlite-replication/sbom-refresh-20261106-evidence/local-candidate-current-JIr2oOeH/`
foram criados uma archive Rust e uma cópia Go sem substituir evidências
anteriores. A archive Rust tem SHA-256
`68e7975d1aff722c0448f2fd27aa7fdcd2192be7d674806a2d94abc2467a2cb3`;
124 membros regulares foram verificados antes da extração, sem duplicatas nem
caminhos absolutos ou `..`, e o manifesto passou. A cópia Go tem 133 arquivos
de fontes e fixtures e seu manifesto também passou. Testes isolados do workspace
Rust (offline, target novo) passou: core 69 aprovados e 8 ignorados, API 12,
coordinator 32 e spool 12 aprovados e 2 ignorados. O módulo separado de
telemetria Rust passou quatro testes offline; o manifesto da extração passou
novamente depois de ambos os testes. A cópia Go passou em
`go test -race -count=1 ./...`, assim como seu módulo separado do exemplo de
telemetria; o manifesto passou novamente depois dos testes. A cópia Go não preserva
necessariamente permissões de execução. Esta anotação é posterior à geração;
nenhum candidato constitui aprovação de notices ou distribuição.

O `make test` posterior aos testes concorrentes passou formatação, vet, Clippy,
testes, fuzz e conformance (96 restores), mas terminou com erro em `test-floci`:
o Docker Hub encerrou a conexão ao solicitar o token anônimo para a imagem fixada.
Alguns testes Go usaram cache e os drills CP-01 opt-in não foram executados.
Execuções anteriores do Floci passaram, mas não substituem esse gate incompleto.
Uma repetição posterior e separada de `make test-floci` passou, incluindo
conformance, restore e persistência após reinício do Floci. Isso recupera a
verificação local da etapa que falhou, mas a execução original de `make test`
continua registrada como falha; não equivale a homologação AWS S3/R2 real.

### Candidato local atual — verificação iniciada

Foi gerada uma nova archive sem substituir candidatos anteriores em
`~/git-worktrees/sqlite-replication/sbom-refresh-20261106-evidence/local-candidate-current-9EFwBxQk/rust.tar.gz`,
SHA-256 `2e7de7969aa7008a4e37136631d0821c82d58de2c927da47d35963fc0bc4f1fb`.
A inspeção verificou 124 membros regulares, sem nomes duplicados, caminhos
absolutos ou componentes `..`; após extração isolada, `sha256sum -c --quiet
SHA256SUMS` passou após os testes do workspace. O `cargo test --offline --locked
--workspace --all-features` passou com target novo: core 66 testes aprovados e
8 ignorados, API 9 aprovados, coordinator 32 aprovados e spool 12 aprovados e
2 ignorados. O exemplo de telemetria, workspace separado, também passou com
quatro testes de integração usando o mesmo target e cache local offline. Os
drills CP-01 e testes que exigem Floci ou binários Go separados foram ignorados
na execução isolada; isso não prova instalação offline em outro host.
Esta anotação é posterior à archive e não
faz parte dela. O candidato é provisório, não é aprovação de notices nem de
distribuição e poderá ficar desatualizado com alterações posteriores.

No mesmo diretório foi criada uma cópia isolada `go-copy/` com 130 arquivos
Go e fixtures não ignorados, incluindo alterações locais; seu manifesto
`SHA256SUMS` passou na verificação inicial. A cópia de conteúdo não é uma
archive de distribuição e não preserva necessariamente permissões de execução.
`go test -race -count=1 ./...` passou na cópia e, separadamente, no módulo do
exemplo Go de telemetria. O manifesto passou novamente após os testes. Esses
resultados não autorizam distribuição e não verificam instalação em outro host.

### Candidatos da retomada atual — validação em andamento

Foram geradas duas archives locais idênticas em
`~/git-worktrees/sqlite-replication/sbom-refresh-20261106-evidence/local-candidate-XbYv27Rk/`;
SHA-256 de ambas: `e672541c2c48fdf43208b1ad52813aa03c17baaf1c2d38ab5bb542f3d32e5a86`.
Antes da extração verificaram-se 122 hashes e 124 membros regulares, sem
duplicatas, caminhos absolutos, `..` ou membros extras. Uma cópia separada de
129 arquivos Go e fixtures foi conferida por SHA-256 e seu manifesto preservado
no mesmo diretório. `go test -race -count=1 ./...` passou na cópia Go isolada
e, separadamente, em seu módulo do exemplo Go de telemetria.
Na extração Rust, `cargo test --offline --locked --workspace --all-features`
passou com target novo: 66 testes do core e 32 do coordenador, com testes opt-in
ignorados conforme a saída. `sha256sum -c --quiet SHA256SUMS` passou após a
execução. O exemplo Rust, workspace separado, também passou em `cargo test
--offline --locked --all-features`, com quatro testes de integração, usando o
mesmo target recém-criado. As cópias
não contêm automaticamente alterações de documentação feitas depois de sua
criação e não constituem artefatos finais nem aprovação de notices.
Comparação posterior dos hashes dos membros registrou zero diferenças para os
94 arquivos Rust/fixtures da archive e os 129 arquivos Go/fixtures da cópia
frente ao checkout. Esta comparação não procura nomes adicionais nem cobre
documentação posterior, arquivos ignorados ou conteúdo gerado.
Posteriormente, os arquivos Go de recovery/captura foram alterados para limitar
a imagem temporária de recovery; portanto a cópia Go acima não valida essas
alterações posteriores e precisa ser regenerada para abrangê-las.
Os fontes Rust de spool e captura também foram alterados posteriormente para o
limite de imagem de recovery; a archive Rust não inclui essa alteração.

Duas gerações do checkout modificado produziram archives idênticas de SHA-256
`f8313da0cdb1c14015a3cf9410d55edd453a7d75790bd1e5a15653f78b495131`
em `/tmp/opencode/sqlite-notice-audit-Sm6HsqcO/`. Inspeção sem extração verificou
122 hashes, dois arquivos de controle, ausência de nomes duplicados, caminhos
absolutos ou `..`, e de membros não regulares. Três arquivos com `quarantined`
no nome estão presentes. Nenhum membro tem basename exatamente `LICENSE`,
`NOTICE` ou `COPYING`; essa busca não identifica avisos embutidos ou outros nomes.
Posteriormente, a archive foi verificada e extraída em diretório isolado em
`~/git-worktrees/sqlite-replication/candidate-f8313da-validation-20261106/`.
Com Cargo 1.99.1, cache local offline e target novo, `cargo test --offline
--locked --workspace --all-features` passou nos testes não ignorados: 63 do
core e 30 do coordinator, entre os demais. Os testes opt-in de Floci, CP-01 e
interoperabilidade entre processos ficaram ignorados nessa execução. O exemplo
de telemetria é workspace separado e não está coberto por esse comando. Isso
não demonstra instalação offline em outro host nem aprova notices ou distribuição.
Esta anotação é posterior à geração e não está na archive.

### Avisos Prometheus do exemplo Go

No grafo normal do exemplo Go isolado em `linux/amd64`, foram selecionados
`client_golang v1.25.0`, `client_model v0.6.3`, `common v0.72.0` e
`procfs v0.22.0`. Os quatro arquivos `NOTICE` na raiz dos respectivos módulos
foram lidos integralmente no cache local; seus SHA-256 são:

| Módulo | SHA-256 do `NOTICE` |
|---|---|
| `client_golang` | `8548bfb2bc1913396df38c548a6ec60b8dbcac3b0800485a40569c8bdd184471` |
| `client_model` | `6c79faa15168885fb88a316ae0df18f486deafddefdc16826cdc56dfbd421e24` |
| `common` | `600244c8052c8c1d307043fafacbc83b429c171dd8e08f5a537c0a54014585ee` |
| `procfs` | `497fcdb9fae55924c12f55f05cbe83a096662e5c591d79b74e4a8f23bab5ffee` |

Os quatro creditam The Prometheus Authors e mencionam SoundCloud; o `NOTICE`
de `client_golang` também identifica perks e suporte Go a Protocol Buffers. A
aplicabilidade dos avisos e dos textos de licença depende dos arquivos e da
forma de distribuição efetivos; seleção pelo compilador não comprova inclusão
no artefato final. Os avisos não foram copiados para um pacote de distribuição,
e outros avisos embutidos e transitivos continuam sujeitos a revisão.

O exemplo Rust de telemetria incluído nessa mesma archive foi testado
separadamente com `cargo test --offline --locked --all-features`: passaram os
quatro testes de integração. Usou o mesmo `fresh-target` criado para esta
extração e o cache local de dependências; não é prova de instalação offline em
outro host ou de aprovação de seus notices.

Os SHA-256 de `go.mod` e `go.sum` do exemplo Go isolado e de `Cargo.toml` do
exemplo Rust extraído coincidem com as entradas da triagem
`telemetry-examples/README.md`. Porém o `Cargo.lock` Rust extraído tem SHA-256
`fc290e4d1bed61333112badf073e327d82b4a4c20d761c6f9d1e2d0daaf90c5f`,
diferente do hash `2cd20995…147d85f` registrado naquela triagem. A comparação
específica mostrou que as versões e checksums ali examinados de
`opentelemetry 0.33.1`, `opentelemetry_sdk 0.33.1`, `prometheus 0.14.0`,
`protobuf 3.7.2` e `protobuf-support 3.7.2` permanecem iguais; `opentelemetry_sdk`
não figura no SBOM Rust anterior, conforme já documentado para dependências de
teste. Isso não estabelece que o restante dos grafos ou avisos seja igual: gerar
e revisar inventário atualizado para qualquer artefato escolhido antes de
considerar sua revisão concluída.
Comparando pares nome/versão, o lockfile tem 252 entradas de registry e 39
delas não aparecem entre os componentes do SBOM histórico. A comparação é de
escopos diferentes: lockfile inclui dependências de teste e outros targets;
os 39 não são, por isso, automaticamente componentes distribuídos ou prova
individual de deriva. O inventário atualizado deve selecionar o alvo e o
artefato específicos e tratar separadamente testes e demais plataformas.

### Seleção atual dos exemplos de telemetria

Foi gerado um novo CycloneDX para o exemplo Rust extraído da archive
`f8313da0…b495131`, com `cargo-cyclonedx 0.5.9`, `--format json
--all-features --target x86_64-unknown-linux-gnu --spec-version 1.5` e
dependências locais offline. A archive do gerador teve SHA-256
`9bd3e599314f50810c9d98b8b68a617ff9d3cc20873968d90b29d121f6b226ff`,
igual ao valor anteriormente registrado. O resultado novo está na cópia
isolada, em `rust/examples/coordinator-telemetry/rust-telemetry-refresh-20261106.json`,
SHA-256 `375564f263a10bdceed2df7152f19dbe901d3269115b8171dec63d862fc85004`.
O JSON foi lido e informa CycloneDX 1.5 e 220 componentes; os pares
nome/versão coincidem com os 220 componentes do SBOM histórico. Essa
comparação não verifica relações, licenças, checksums ou schema CycloneDX;
a validação estrutural foi posteriormente realizada: a CLI CycloneDX 0.33.1,
binário SHA-256 `bfc8b2538da86fe239bc53658bbb63c1c8c510a293c1e6891aa5bea5d3c58746`,
confirmado contra o release upstream e a evidência anterior, retornou
`BOM validated successfully` para `validate --input-format json
--input-version v1_5 --fail-on-errors`. Isso não valida cobertura ou notices. O inventário não
inclui automaticamente dependências de teste nem comprova conteúdo de binário.
O SBOM original e sua quarentena não foram modificados. O arquivo gerado é
posterior à extração e não integra a archive original.

Comparando os 220 componentes por nome e versão com o SBOM histórico do exemplo
Rust, não houve diferenças nos campos `licenses` nem `evidence`; todos os
componentes novos têm licença ou evidência de licença. Esses campos não são
prova da correção da declaração nem abrangem o componente raiz ou todos os
avisos embutidos. A revisão anterior dos textos upstream continua aplicável
apenas às versões e aos escopos que examinou.

Para conferir a cadeia de telemetria, o inventário novo declara Apache-2.0
para `prometheus 0.14.0` e MIT para `protobuf 3.7.2` e `protobuf-support 3.7.2`.
As archives dessas três dependências no cache foram novamente verificadas
contra o lockfile da extração, com hashes respectivamente
`3ca5326d8d0b950a9acd87e6a3f94745394f62e4dae1b1ee22b2bc0c394af43a`,
`d65a1d4ddae7d8b5de68153b48f6aa3bba8cb002b243dbdbc55a5afbc98f99f4` e
`3e36c2f31e0a47f9280fb347ef5e461ffcd2c52dd520d8e216b52f93b0b0d7d6`.
Esses valores coincidem com `telemetry-examples/upstream-notices-review.md`,
que registra os textos e limitações da triagem anterior. Continua necessária a
revisão dos avisos embutidos, dependências transitivas e composição efetiva de
cada artefato escolhido; nenhuma licença final ou distribuição foi aprovada.

Uma comparação textual adicional confrontou o campo `licenses` dos 220
componentes com `package.license` dos `Cargo.toml` normalizados presentes no
cache local do registry: 204 são idênticos; em nove, o gerador converteu o
separador antigo `/` em `OR` (`bytes-utils`, `fallible-iterator`,
`fallible-streaming-iterator`, `ff`, `fnv`, `group`, `vcpkg`, `version_check`
e `xmlparser`). Os outros sete são crates locais `sqlite-replication-*`, com
PURLs `file://`, não entradas desse registry. A comparação é textual: não lê
os arquivos de licença, não verifica notices embutidos, nem garante que a
normalização corresponda a uma interpretação jurídica ou à distribuição
efetiva. Os caminhos locais dos PURLs identificam a extração de investigação,
não um pacote publicado. A revisão de textos e dos artefatos finais permanece
pendente.

Nas nove crates acima, as archives `.crate` do cache foram conferidas contra os
checksums do `Cargo.lock` do exemplo extraído: todas coincidiram. Uma busca em
todos os membros regulares por basename iniciado por `LICENSE`, `NOTICE` ou
`COPYING` encontrou `LICENSE-MIT` e `LICENSE-APACHE` em cada archive e nenhum
outro nome nesse conjunto. As versões são `bytes-utils 0.1.4`,
`fallible-iterator 0.3.0`, `fallible-streaming-iterator 0.1.9`, `ff 0.14.0`,
`fnv 1.0.7`, `group 0.14.0`, `vcpkg 0.2.15`, `version_check 0.9.5` e
`xmlparser 0.13.6`. Essa inspeção não lê integralmente os 18 textos, nem cobre
avisos embutidos ou com outros nomes. Uma busca por linhas de copyright nos
textos MIT encontrou créditos nas oito archives exceto `group`, cujo arquivo
MIT não apresenta linha explícita de titular de copyright; verificar os fontes
e a proveniência antes de preparar atribuições. A presença dos dois arquivos
não seleciona alternativa de licença nem comprova inclusão em um binário.

Posteriormente os nove arquivos `LICENSE-MIT` foram lidos integralmente de
suas archives verificadas contra o lockfile:

| Crate | SHA-256 do `LICENSE-MIT` | Copyright indicado no texto |
|---|---|---|
| `bytes-utils 0.1.4` | `ff3f1cd12af8866d9bde961d6cc40df774cd131d484de31d3170c4b02b21a7b5` | 2017 arc-swap developers |
| `fallible-iterator 0.3.0` | `0816e154b159ba255c563f7c8c7df5bbb8cc5fc96f5ab8cf9f4743b4f41fe7eb` | 2015 The rust-openssl-verify Developers |
| `fallible-streaming-iterator 0.1.9` | `8dcec5569a9be5b0e086c80faed6f1aefa670af0ec29cecc2f714303096887e0` | 2016 The fallible-streaming-iterator Developers |
| `ff 0.14.0` | `8d0c1d1d4b2bebf5b9211f0e883d5a482b7808f3ed31320da14ddee95811f4ef` | 2017 Sean Bowe |
| `fnv 1.0.7` | `65fdb6c76cd61612070c066eec9ecdb30ee74fb27859d0d9af58b9f499fd0c3e` | 2017 Contributors |
| `group 0.14.0` | `23f18e03dc49df91622fe2a76176497404e46ced8a715d9d2b67a7446571cca3` | Nenhum titular explícito neste arquivo; ver `COPYRIGHT` abaixo. |
| `vcpkg 0.2.15` | `016d20f335060a70e79d9fcf8dfaa6201114d65d592211f4bdfb8ae9ca2bc1dc` | 2017 Jim McGrath |
| `version_check 0.9.5` | `b7e650f3fce5c53249d1cdc608b54df156a97edd636cf9d23498d0cfe7aec63e` | 2017–2018 Sergio Benitez |
| `xmlparser 0.13.6` | `0b5f24524360e15bcf9fb79bca875f5aa62f01f94f2988e23238c7ec0e6e8784` | 2018 Reizner Evgeniy |

Os nove textos incluem condição de inclusão do aviso de copyright e permissão
em cópias ou porções substanciais e disclaimer; a tabela não substitui os
textos completos. Suas alternativas Apache, avisos em outros arquivos e
aplicabilidade ao artefato escolhido continuam por revisar. Os componentes
são dependências do grafo, não fontes vendorizadas na archive candidata.

Os nove arquivos `LICENSE-APACHE` também foram conferidos contra as archives
do cache, que correspondem ao lockfile. Os nove arquivos formam quatro grupos
de bytes idênticos; um texto de cada grupo foi lido integralmente:

| Crates | SHA-256 do `LICENSE-APACHE` |
|---|---|
| `bytes-utils 0.1.4`, `fnv 1.0.7`, `group 0.14.0`, `version_check 0.9.5`, `xmlparser 0.13.6` | `a60eea817514531668d7e00765731449fe14d059d3249e0bc93b36de45f759f2` |
| `fallible-iterator 0.3.0`, `fallible-streaming-iterator 0.1.9` | `c6596eb7be8581c18be736c846fb9173b69eccf6ef94c5135893ec56bd92ba08` |
| `ff 0.14.0` | `3708458dee7f359ac6c9c5558023ed481be87e5372c43ebd7f2ca7ad23c12026` |
| `vcpkg 0.2.15` | `60c93a31f490375aadf64098b75f10715010379541021e00261045d7800611d3` |

Os textos contêm Apache 2.0, incluindo condições de redistribuição e avisos
aplicáveis; seus apêndices apresentam campos exemplificativos, não atribuições
preenchidas. A identificação por hash não revisa avisos em fontes individuais,
não seleciona a alternativa Apache e não determina a composição de um binário.
Revisar tais avisos e a aplicabilidade dos textos ao artefato final permanece
necessário; esta tabela não substitui as licenças completas.

Busca textual nas nove archives verificadas pelo lockfile examinou 38 arquivos
`.rs` por `copyright`, `SPDX-License-Identifier`, `licensed under`,
`derived from`, `adapted from`, `copied from` e `based on`, sem distinção de
maiúsculas. As 16 linhas retornadas usam as últimas expressões em contexto
técnico, sem aviso explícito de autoria ou licença; por exemplo, o comentário
`based on` em `vcpkg/src/lib.rs` trata de compatibilidade de versões. A busca
não examina outros tipos de arquivo, outros idiomas ou avisos expressos de
outra forma; não comprova autoria integral, cobertura dos notices ou conteúdo
de binários. A inclusão dessas dependências em cada artefato e os avisos
aplicáveis continuam a depender da composição e configuração final escolhida.

Uma busca complementar nas mesmas nove archives verificadas examinou 893
arquivos não `.rs` decodificáveis como UTF-8, excluindo arquivos cujos basenames
começam por `LICENSE`, `LICENCE`, `COPYRIGHT`, `NOTICE` ou `COPYING`, que são
tratados separadamente. Os termos foram `copyright`, `SPDX-License-Identifier`,
`licensed under`, `derived from`, `adapted from` e `copied from`. Além de sete
linhas em READMEs que descrevem alternativas de licença, houve 285 linhas em
`vcpkg 0.2.15/test-data/`, principalmente listas que citam caminhos de arquivos
`copyright` de pacotes externos. Nenhum arquivo regular com basename exatamente
`copyright` foi encontrado na própria archive de `vcpkg`; os caminhos listados
não são, por si, prova de que seus conteúdos estejam incluídos. A revisão não
resolve todos os avisos possíveis nem determina que dados de teste ou esses
pacotes externos integrem qualquer artefato final. A composição de fontes,
testes e ferramentas de build deve ser decidida antes da revisão final.

Na busca posterior por basename `COPYRIGHT*` nessas nove archives, somente
`group 0.14.0` retornou resultado: `group-0.14.0/COPYRIGHT`, SHA-256
`7429802d3e67e3308471b91fbea341e484c4e0a23d0fb230a8636c38184cec79`.
O arquivo completo foi lido na cópia extraída do cache: informa que os direitos
permanecem com os contribuidores e apresenta Apache-2.0 ou MIT como alternativas.
A archive corresponde ao lockfile e o hash do arquivo extraído coincide com o
do membro da archive. Esse aviso é um exemplo que a
busca anterior, restrita a `LICENSE*`, `NOTICE*` e `COPYING*`, não identificava;
outros nomes ou avisos embutidos ainda podem existir. Os contribuidores não
foram individualmente identificados e a aplicabilidade a cada artefato segue
pendente.

#### Busca ampliada de avisos `COPYRIGHT` no exemplo Rust

Uma busca adicional nas 213 archives de registry vinculadas ao lockfile do
SBOM Rust de telemetria não encontrou nenhum membro regular cujo basename
comece, sem distinção de maiúsculas, por `LICENSE`, `LICENCE`, `COPYING`,
`COPYRIGHT` ou `NOTICE` em apenas três delas: `base64-simd 0.8.0`,
`vsimd 0.8.0` e `opentelemetry 0.33.1`. A última já está examinada em
`telemetry-examples/upstream-notices-review.md`. Os manifests das duas primeiras
declaram MIT e apontam ao repositório `Nugine/simd`; os respectivos arquivos
`.cargo_vcs_info.json` nas archives indicam o commit
`d74c030d9dc4f3cae02146d1f497ff62726ef09a`. No
[LICENSE upstream desse commit](https://github.com/Nugine/simd/blob/d74c030d9dc4f3cae02146d1f497ff62726ef09a/LICENSE),
o texto MIT credita copyright 2021 Nugine; o SHA-256 do texto obtido foi
`71674605ec4c087fe9eb534e3e4f9e26eb2e4aabcd76a29fd156c6a844d44b3d`.
As archives de `base64-simd` e `vsimd` têm respectivamente SHA-256
`339abbe78e73178762e23bea9dfd08e697eb3f3301cd4be981c0f78ba5859195` e
`5c3082ca00d5a5ef149bb8b555a72ae84c9c59f7250f013ac822ac2e49b19c64`,
conferidos com o lockfile. A referência ao commit não comprova equivalência
integral de fontes nem exclui avisos embutidos ou com outros nomes. Essas
dependências não são vendorizadas na archive-fonte do candidato; textos e
atribuições para eventual binário ou fonte distribuído requerem revisão por
artefato. Não houve escolha ou aprovação de licença para distribuição.

Nas cópias locais de `base64-simd 0.8.0` e `vsimd 0.8.0`, os 15 e 26
arquivos regulares respectivamente coincidem byte a byte com suas archives,
cujos checksums correspondem ao lockfile. Uma busca insensível a maiúsculas por
`copyright`, `SPDX`, `license`/`licence`, `derived from`, `adapted from`,
`based on` e `copied from` retornou apenas os manifests e READMEs declarando
MIT; ambos os READMEs apontam a `../../LICENSE`, que não integra essas archives.
Busca adicional por URLs nos fontes encontrou referências à especificação
WHATWG em `base64-simd/src/forgiving.rs` e, em `vsimd`, referências técnicas,
incluindo `src/alsw.rs` que afirma ser inspirado por um gist externo. Essa
afirmação de inspiração não prova cópia nem resolve proveniência ou termos de
terceiros; verificar sua relevância e os avisos para qualquer artefato final.
As buscas são limitadas aos termos e arquivos examinados, não substituem
revisão integral dos fontes nem determinam o conteúdo de um binário.

Uma comparação adicional por bytes do arquivo específico `src/alsw.rs` da
cópia local `vsimd 0.8.0` com o arquivo no commit upstream
`d74c030d9dc4f3cae02146d1f497ff62726ef09a`, indicado pelo
`.cargo_vcs_info.json` da crate, encontrou 5.194 bytes idênticos e SHA-256
`0160b74a177404e7c548224ade7b0a1112754aac7665672a3d16710a08f2747f`
em ambas as cópias. Fonte upstream fixada:
`https://raw.githubusercontent.com/Nugine/simd/d74c030d9dc4f3cae02146d1f497ff62726ef09a/crates/vsimd/src/alsw.rs`.
Isso associa o arquivo examinado ao commit apontado pelos metadados, mas não
prova identidade do restante da árvore, não resolve direitos sobre o gist citado
e não prova inclusão no binário S3. A archive-fonte Rust candidata não vendoriza
a crate; os avisos de qualquer binário dependem da composição efetiva e da
revisão de terceiros. A cópia Go candidata não contém essa crate Rust.
Uma comparação posterior, limitada aos arquivos `*.rs` sob `src/` presentes
nas cópias das duas crates `0.8.0`, consultou os caminhos correspondentes no
mesmo commit upstream fixado `d74c030d9dc4f3cae02146d1f497ff62726ef09a`:
21 de 21 arquivos de `vsimd` e 10 de 10 de `base64-simd` coincidiram byte a
byte, sem erros de consulta. Os arquivos foram obtidos no padrão
`https://raw.githubusercontent.com/Nugine/simd/<commit>/crates/<crate>/<caminho>`.
Não foram comparados manifests, arquivos não Rust ou possíveis arquivos extras
na árvore upstream; a correspondência não comprova direitos sobre o gist nem
inclusão em binário. A composição S3 seleciona essas dependências, mas avisos
de distribuição continuam sujeitos à composição efetiva e revisão. Esta
anotação é posterior à archive-fonte candidata e não integra seus bytes.

O gist citado em `vsimd/src/alsw.rs` foi consultado na revisão
`5ce93a2bb7512cab656a2f57e7d5b22bd595068e`: a página apresentou um arquivo
`base64_sse3_test.cpp` publicado por `aqrit`. O conteúdo bruto consultado tem
SHA-256 `164c54fa6799caa7d6c6344adaa6cc0762874ea2d0581f2874b7a186eab4614e`;
nele não foi identificada declaração explícita de licença. O arquivo `alsw.rs`
usa a palavra "inspired"; o gist contém uma função `hash` com operações
semelhantes às descritas na implementação Rust. Isso não estabelece cópia,
direitos ou condições de distribuição, nem afirma que o gist inteiro esteja
incorporado. A proveniência e eventual necessidade de autorização ou atribuição
continuam pendentes de revisão pelo responsável pelos artefatos finais.
Consulta adicional à API pública do gist
`https://api.github.com/gists/a2ccea48d7cac7e9d4d99f19d4759666` retornou
criação em 2017-09-18, um único arquivo `base64_sse3_test.cpp` e uma entrada
de histórico: a revisão `5ce93a2bb7512cab656a2f57e7d5b22bd595068e`, também de
2017-09-18. O campo de descrição veio vazio; a resposta consultada não fornece
uma declaração de licença. A data de atualização do gist na API é posterior,
mas não demonstra alteração de conteúdo ou concessão de licença. Isso não
substitui confirmação pelo titular nem determina os notices de um binário que
eventualmente incorpore `vsimd`; a archive-fonte candidata não o vendoriza.

O histórico público do caminho `crates/vsimd/src/alsw.rs` em `Nugine/simd`
mostra que o arquivo foi adicionado no commit
`330ed8158e9ce2d7d1f60cc1b245f0bba91169b8`, em 2022-09-17, já com a
referência ao gist e a expressão “Inspired by”. A API desse commit classifica o
arquivo como `added`; a consulta ao arquivo fixado confirma o comentário.
Fontes:
`https://api.github.com/repos/Nugine/simd/commits/330ed8158e9ce2d7d1f60cc1b245f0bba91169b8`
e `https://raw.githubusercontent.com/Nugine/simd/330ed8158e9ce2d7d1f60cc1b245f0bba91169b8/crates/vsimd/src/alsw.rs`.
Isso documenta a referência desde a introdução nesse caminho, não comprova
origem completa do algoritmo, direitos do gist ou licença de eventual material
copiado. A archive-fonte Rust candidata não vendoriza a crate `vsimd`; seu uso
em binários e os notices correspondentes dependem da composição efetiva, ainda
por revisar. Nenhuma autorização de distribuição é inferida.

Na composição Rust selecionada descrita para o candidato `8avAZthl`, a cópia
de `base64-simd 0.8.0` do registry declara `mod alsw` em `src/lib.rs:55`.
Os fontes `src/check.rs:68–70` e `src/decode.rs:282–285` referenciam,
respectivamente, `vsimd::alsw::check_ascii_xn` e `decode_ascii_xn`; a cópia de
`vsimd 0.8.0` exporta `alsw` em `src/lib.rs:77`. A correspondência dessas
cópias com as archives verificadas pelo lockfile está registrada acima.
Essas referências delimitam a investigação dos avisos na cadeia selecionada
para S3, mas não provam execução, retenção pelo linker ou presença em binário
específico. A archive-fonte candidata não vendoriza essas dependências; a
proveniência do gist e os avisos aplicáveis por artefato permanecem pendentes.

O exemplo Rust seleciona também `outref 0.5.2`, dependência de `base64-simd`.
O SBOM e o manifesto declaram MIT. A archive do registry tem SHA-256
`1a80800c0488c3a21695ea981a54918fbb37abf04f4d0720c453632255e2ff0e`,
igual ao lockfile; seus 14 membros regulares coincidem com os arquivos da cópia
local examinada. O único basename iniciado por `LICENSE`, `LICENCE`, `COPYING`,
`COPYRIGHT` ou `NOTICE` é `LICENSE`, SHA-256
`a00511478c83ebb464ef19d840f061c7444adc4636bb561900e75c5808cfc6cc`.
O texto MIT foi lido integralmente e credita copyright 2022 Nugine; contém
condição de preservação do aviso e da permissão e disclaimer. Uma busca por
marcadores de copyright, SPDX, licença e derivação nas cópias locais retornou
o LICENSE, os manifests e o README, não outros avisos explícitos sob os termos
consultados. Isso não exclui avisos redigidos de outro modo nem prova inclusão
de `outref` no artefato final. A dependência não integra a archive-fonte do
candidato; sua aplicabilidade e os textos finais dependem da composição escolhida.

Nos 220 componentes do SBOM, os sete crates locais foram excluídos desta
consulta. As 213 archives de registry restantes foram encontradas no cache,
e seus SHA-256 coincidiram com o lockfile extraído. Uma busca em membros
regulares por basename iniciado por `COPYRIGHT` encontrou seis arquivos:

| Crate | SHA-256 do membro `COPYRIGHT` |
|---|---|
| `group 0.14.0` | `7429802d3e67e3308471b91fbea341e484c4e0a23d0fb230a8636c38184cec79` |
| `linux-raw-sys 0.12.1` | `3290ae0fbc9ddb77d2239121d710f0bb9d31b3b4744e6d97fe01e652b4c1870b` |
| `rand_core 0.10.1` | `92b81db30f7ab6d693e0af5661a40d273ca1947e891123ed823924192c10cbc7` |
| `rustix 1.1.6` | `377c2e7c53250cc5905c0b0532d35973392af16ffb9596a41d99d202cf3617c9` |
| `utf8_iter 1.1.0` | `c30152c94a6d75e021adbc52b3a52470366a46edb917e17deae3259251af244c` |
| `utf_types 1.0.0` | `3210be7332b5bdf48eb24a945258b9f38616a2cceb0dfc06e3c3c7e9740475a0` |

Os seis textos foram lidos nas cópias extraídas do cache, cujos hashes
correspondem aos respectivos membros das archives. `linux-raw-sys` e `rustix`
informam direitos retidos pelos contribuidores e alertam para arquivos derivados
de `libstd`; `rand_core` aponta autoria no histórico de versões. Os avisos de
`utf8_iter` e `utf_types` creditam Mozilla Foundation e distinguem trechos de
teste designados como domínio público/CC0; `utf8_iter` também aponta adaptação
de código da biblioteca padrão Rust e sua revisão de origem. O aviso de `group`
está detalhado acima. Estes resumos não substituem textos nem examinam as
exceções em arquivos individuais. A busca não cobre outros nomes, fontes de
crates locais, dependências de desenvolvimento omitidas pelo SBOM, ou quais
objetos são vinculados em binários; aplicabilidade e revisão integral de notices
continuam pendentes.

#### Índice de avisos embutidos para revisão posterior

Nas 213 archives de registry do SBOM de telemetria Rust, conferidas contra o
lockfile extraído, uma triagem por linha procurou, sem distinção de maiúsculas,
`copyright` como palavra, `SPDX-License-Identifier`, `licensed under`,
`derived from`, `adapted from` e `copied from`. Foram examinados 9.764 arquivos
UTF-8 com extensões `.rs`, `.c`, `.h`, `.s`, `.cc`, `.cpp` ou `.inc` (extensão
comparada sem distinção de maiúsculas); 2.877 arquivos de 101 crates tiveram
correspondências. O índice registra crate, membro, SHA-256 do arquivo, categoria
e números de linha, não a interpretação dos resultados:
`~/git-worktrees/sqlite-replication/sbom-refresh-20261106-evidence/rust-embedded-notice-index.json`,
SHA-256 `1afaf4d4660be9b49b6e65e22a552b51e79872712dbffe8fd08039977ec23952`.

Revisão pontual adicional da ocorrência `origin` em
`aws-lc-rs 1.19.0/src/cmac.rs`, identificada no índice pelo SHA-256
`9d3eff1a07c856e1af094548944be05b184b2d7653d5e19b0e1e01f8e763ea84`:
a linha 266 diz que um valor de chave pode ser derivado de uma chave aleatória
por uma função de derivação. Trata-se de documentação técnica sobre chaves,
não de uma declaração de origem de código. O cabeçalho nas linhas 1–2 credita
Amazon.com ou afiliadas e declara `Apache-2.0 OR ISC`. A classificação cobre
somente essa ocorrência e não esgota os avisos da crate ou de AWS-LC. O grafo
Rust S3 seleciona `aws-lc-rs` conforme `s3-aws-lc-transitive-notices.md`, mas
isso não comprova inclusão de CMAC em binário; a dependência não é vendorizada
na archive-fonte candidata Rust. Esta observação não examina o candidato Go.

Revisão do conjunto de 26 arquivos de `serde_json 1.0.154` marcados como
`origin` nesse índice: os hashes das cópias locais coincidiram com os 26
registrados. São 19 arquivos sob `src/lexical/` e sete sob `tests/lexical/`.
Os cabeçalhos dos fontes apontam adaptação de `rust-lexical`; `src/lexical/mod.rs`
credita Alexander Huszagh e declara licenciamento MIT ou Apache-2.0 para esse
módulo. Os testes também citam casos e fontes técnicas adicionais; não foram
examinados integralmente nem se conclui que seus dados integrem um artefato.
O manifesto da crate declara `MIT OR Apache-2.0`; nenhum termo é escolhido aqui.

Na extração Rust `8avAZthl`, a consulta `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -e features -i
serde_json` retornou `serde_json 1.0.154` com features `default` e `std`,
inclusive arestas de desenvolvimento, mas não `float_roundtrip`. O arquivo
`src/lib.rs` condiciona `mod lexical` a essa feature. Isso descreve somente o
grafo consultado: `--all-features` do workspace não habilita necessariamente
todas as features transitivas e outro consumidor pode habilitá-la. A seleção
não prova conteúdo de binário; a crate não está vendorizada na archive-fonte
candidata. Preservar a atribuição para revisão conforme composição efetiva;
esta análise não abrange o candidato Go nem encerra os demais avisos.

Para `tokio 1.54.1`, foram examinados os seis arquivos classificados como
`origin` no mesmo índice, com hashes locais coincidentes com os registrados.
Em quatro arquivos de `src/io/`, as ocorrências descrevem cópia de dados entre
buffers ou fluxos; em `src/runtime/blocking/sharded.rs`, descrevem a escolha de
um índice a partir do identificador de worker. Em `src/runtime/builder.rs`, as
linhas 281 e 294 atribuem ao Go a possível origem do valor numérico `61`, não
afirmam cópia de uma implementação. Essas classificações cobrem apenas as
linhas indexadas e seu contexto imediato, não todos os avisos da crate.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i tokio` selecionou Tokio
diretamente no core e no adapter S3 e por dependências transitivas; a saída
inclui arestas de desenvolvimento. O grafo não prova vinculação dos arquivos
citados em um binário. Tokio não está vendorizado na archive-fonte candidata;
os avisos aplicáveis dependem da composição final. A revisão não abrange o
candidato Go nem resolve as questões abertas de `vsimd` e `fpfmt`.

Para `memchr 2.8.3`, os oito arquivos classificados como `origin` no índice
foram conferidos contra os hashes registrados, todos coincidentes. As linhas
indexadas usam `derived from` para descrever a relação entre ponteiros e objetos
alocados; não são declarações de origem do código. A conclusão cobre somente
essas ocorrências, não todos os arquivos, licenças ou avisos da crate. Inclui
arquivos específicos de AArch64, WebAssembly e x86-64, sem afirmar que foram
compilados ou vinculados no mesmo binário.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i memchr --depth 2` mostrou
`memchr 2.8.3` via `serde_json 1.0.154`, incluindo caminhos para spool e
storage-fs e arestas de desenvolvimento. A crate não é vendorizada na
archive-fonte candidata; o grafo não estabelece conteúdo de binário ou
completude de notices. Esta análise não abrange o candidato Go.

Complemento dos textos locais de `memchr 2.8.3`, sem repetir a triagem das
oito ocorrências acima: o manifesto declara `Unlicense OR MIT`. Foram lidos na
cópia do registry `COPYING`, `LICENSE-MIT` e `UNLICENSE`. `COPYING` afirma que
o código pode ser usado sob qualquer uma das duas alternativas; `LICENSE-MIT`
credita Andrew Gallant, 2015, e inclui condição de preservação do copyright e
aviso de permissão e disclaimer. `UNLICENSE` contém declaração de dedicação ao
domínio público e disclaimer. Os SHA-256 locais são, respectivamente:

| Arquivo | SHA-256 |
|---|---|
| `COPYING` | `01c266bced4a434da0051174d6bee16a4c82cf634e2679b6155d40d75012390f` |
| `LICENSE-MIT` | `0f96a83840e146e43c0ec96a22ec1f392e0680e6c1226e6f3ba87e0740af850f` |
| `UNLICENSE` | `7e12e5df4bae12cb21581ba157ced20e1986a0508dd10d0e8a4ab9a4cf94e85c` |

Esses textos não selecionam licença final nem esgotam eventuais avisos
incorporados. O grafo e os limites por candidato descritos acima permanecem;
as obrigações dependem dos artefatos e componentes efetivamente distribuídos.

Os seis arquivos `aws-smithy-schema 1.1.2` marcados como `origin` no índice
foram examinados nos trechos registrados; os hashes das cópias locais
coincidiram com os do índice. As ocorrências de `derived from` descrevem dados,
máscaras, referências, rotas ou tamanhos derivados de outros valores; não
identificam origem ou cópia de código de terceiros. Essa classificação cobre
somente as ocorrências indexadas e não esgota os avisos ou fontes da crate.
No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i aws-smithy-schema --depth 2`
mostrou a versão `1.1.2` via `aws-sdk-s3 1.157.0` para o adapter S3 e também
arestas de desenvolvimento. A dependência não é vendorizada na archive-fonte;
o grafo não prova o conteúdo de um binário nem completa a revisão de notices.
Esta verificação não abrange o candidato Go.

Em `indexmap 2.14.2`, os quatro arquivos marcados como `origin` no índice
foram conferidos contra os hashes registrados, todos coincidentes. Os
comentários em `src/lib.rs:258` e `src/util.rs:78` descrevem, respectivamente,
um tipo de erro e uma função de comparação de slices como copiados da biblioteca
padrão Rust; o segundo menciona uma adaptação para comparador próprio. Os
arquivos `src/map/tests.rs` e `src/set/tests.rs` também atribuem testes de
busca e particionamento a testes da biblioteca padrão. As ocorrências
examinadas não identificam revisões específicas dessas fontes. O manifesto da
crate declara `Apache-2.0 OR MIT`; essa declaração não substitui a análise dos
avisos aplicáveis às partes atribuídas à biblioteca padrão. A classificação
cobre somente os trechos indexados, não toda a crate.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i indexmap --depth 2`
selecionou `indexmap 2.14.2` via `h2 0.4.20` e seus consumidores
`aws-smithy-http-client` e `hyper`. A crate não é vendorizada na archive-fonte;
o grafo não prova que os trechos ou testes sejam incluídos num binário. A
revisão dos avisos por artefato permanece aberta; esta análise não abrange o Go.

Em `tokio-util 0.7.20`, os quatro arquivos classificados como `origin` no
índice foram conferidos contra os hashes registrados, todos coincidentes. Em
`src/io/write_all_vectored.rs:138–140`, o comentário declara que a função
`advance_slices` foi copiada de `std::IoSlice::advance_slices`, sem identificar
uma revisão da biblioteca padrão. `src/sync/cancellation_token.rs` descreve
tokens derivados de outro token, não origem de fonte. Nos dois arquivos de
teste, um comentário descreve um valor de execução derivado de outro e outro
atribui o valor `128` a um arquivo Tokio identificado por commit. O manifesto da
crate declara MIT; a origem e os textos aplicáveis à função copiada da biblioteca
padrão permanecem sujeitos a revisão, sem presumir que a declaração da crate
resolva todos os avisos. Não foram examinados todos os arquivos ou outros
marcadores de proveniência da dependência.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i tokio-util --depth 2`
selecionou `tokio-util 0.7.20` via `aws-smithy-types` e `h2`, incluindo arestas
de desenvolvimento. O grafo não prova que a função ou testes integrem um
binário; a crate não é vendorizada na archive-fonte. A revisão não abrange o
candidato Go nem encerra os notices dos demais componentes.

Para aprofundar essa atribuição, a crate `tokio-util 0.7.20` identifica em
`.cargo_vcs_info.json` o commit Tokio
`5ec9e6b8d5a7111fe9cd35a637bd8b4e4eb1e483`. A consulta ao histórico do caminho
`tokio-util/src/io/write_all_vectored.rs` até esse commit retornou quatro
alterações; o commit de introdução é
`8c980ea75a0f8dd2799403777db700c2e8f4cda4`, de 2026-02-27, cujo diff já
inclui o comentário sobre cópia de `std::IoSlice::advance_slices`. A descrição
do PR upstream [#7768](https://github.com/tokio-rs/tokio/pull/7768) afirma
que a função foi copiada com pequenos ajustes para a versão mínima do Rust;
nem a descrição nem o comentário identificam uma revisão específica da
biblioteca padrão. A consulta ao histórico está disponível em
`https://api.github.com/repos/tokio-rs/tokio/commits?path=tokio-util/src/io/write_all_vectored.rs&sha=5ec9e6b8d5a7111fe9cd35a637bd8b4e4eb1e483&per_page=100`.
Essas declarações upstream reforçam a proveniência a investigar, mas não
comprovam qual versão foi usada nem encerram a análise dos avisos aplicáveis.
O alcance para o candidato Rust e a ausência de análise do candidato Go são os
descritos acima; a crate não está vendorizada na archive-fonte candidata.

Em `aws-smithy-xml 0.64.1`, os três arquivos marcados como `origin` no índice
foram conferidos contra os hashes registrados, todos coincidentes. Em
`src/unescape.rs:140`, dentro de um módulo `#[cfg(test)]`, um comentário afirma
que casos de teste foram adaptados de `Apache Commons StringEscapeUtilsTest.java`,
sem identificar versão ou revisão. Em `src/codec/deserializer.rs:2616`, o
comentário aponta adaptação de testes de regressão a partir de comentários de
revisão de um PR; em `src/protocol/aws_rest_xml.rs:328`, descreve um código de
erro derivado de um status HTTP, não a origem do código-fonte. O manifesto da
crate declara `Apache-2.0`; isso não identifica a revisão dos testes Apache
Commons nem substitui a revisão de seus eventuais avisos quando esses testes
forem incluídos numa distribuição. A análise cobre os trechos indexados, não
todos os avisos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i aws-smithy-xml
--depth 2` retornou `aws-smithy-xml 0.64.1` via `aws-sdk-s3 1.157.0` para o
adapter S3, com aresta adicional de desenvolvimento no core. O grafo não prova
inclusão dos testes ou de outros objetos em binário; a crate não está
vendorizada na archive-fonte candidata. Esta análise não abrange o candidato Go.

No índice de `aws-sdk-s3 1.157.0`, foram examinadas 36 linhas `origin` em três
arquivos de `src/client/copy_object.rs` e `src/operation/copy_object/`;
os hashes das três cópias locais coincidiram com os registros do índice. As
ocorrências de `copied from` aparecem na documentação da operação S3 e tratam
de checksum, metadados ou tags copiados de um objeto para outro, não de origem
do código-fonte. Isso classifica apenas essas linhas; não esgota os avisos ou
outros arquivos do SDK. O adapter S3 do candidato Rust `8avAZthl` seleciona
`aws-sdk-s3 1.157.0` conforme o grafo registrado acima. A crate não está
vendorizada na archive-fonte candidata e o grafo não prova quais arquivos
integram um binário. Esta análise não abrange o candidato Go nem encerra a
revisão de notices por artefato.

No índice de `aws-smithy-types 1.9.1`, os dois arquivos marcados como
`origin`, `src/big_number.rs` e `src/retry.rs`, tiveram hashes locais iguais
aos registrados. As três ocorrências descrevem, respectivamente, valores
numéricos, uma ordenação e parâmetros derivados de outros valores; não são
declarações sobre origem ou cópia de código. A classificação é restrita às
linhas indexadas e não conclui a revisão dos demais avisos da crate.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i aws-smithy-types --depth 2`
selecionou a versão `1.9.1`, inclusive via `aws-sdk-s3` e
`aws-credential-types`, com arestas de desenvolvimento. A crate não é
vendorizada na archive-fonte candidata e o grafo não demonstra o conteúdo de um
binário. Esta verificação não abrange o candidato Go nem resolve os notices
de outros componentes ou a composição dos artefatos finais.

No índice de `aws-runtime 1.10.2`, os dois arquivos marcados como `origin`
foram conferidos contra os hashes registrados, ambos coincidentes. Em
`src/content_encoding/body/http_body_1_x.rs:680,735`, dois comentários dentro
de `#[cfg(test)] mod tests` atribuem cenários de teste à documentação pública
AWS S3 indicada pelos respectivos URLs. Em `src/env_config.rs:70`, a referência
descreve a origem de configuração em tempo de execução, não origem do código.
A classificação cobre os trechos indexados, não todos os avisos ou textos
eventualmente aplicáveis à crate e aos testes.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i aws-runtime --depth 2`
selecionou `aws-runtime 1.10.2` via `aws-sdk-s3 1.157.0`, com aresta de
desenvolvimento adicional. A crate não é vendorizada na archive-fonte; a
seleção no grafo não prova que os testes ou arquivos integrem um binário.
Esta revisão não cobre o candidato Go nem encerra os notices por artefato.

No índice de `aws-smithy-runtime 1.16.2`, os dois arquivos marcados como
`origin` foram conferidos contra os hashes registrados, ambos coincidentes.
Em `src/client/identity/cache/lazy.rs`, as três ocorrências descrevem um valor
de configuração derivado de outros valores, não origem de código. Em
`tests/stalled_stream_download.rs:202`, um comentário atribui um cenário de
teste aos passos de reprodução de uma issue pública do AWS SDK; essa referência
não identifica licença ou origem de código copiado. A classificação cobre
apenas os trechos indexados e não esgota os avisos da crate ou de seus testes.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i
aws-smithy-runtime --depth 2` selecionou `aws-smithy-runtime 1.16.2` via
`aws-runtime` e `aws-sdk-s3`, incluindo arestas de desenvolvimento. A crate não
é vendorizada na archive-fonte; o grafo não prova que o arquivo de teste ou
outros arquivos integrem um binário. A verificação não abrange o candidato Go
nem determina os notices dos artefatos finais.

No índice de `rustls 0.23.46`, foram conferidos os três arquivos marcados como
`origin`: `src/conn.rs`, `src/crypto/aws_lc_rs/hpke.rs` e `src/quic.rs`.
Os hashes das cópias locais coincidiram com os do índice; as ocorrências
descrevem valores derivados de outros valores, não origem de código-fonte.
Essa classificação cobre somente os trechos indexados e não substitui a
revisão dos demais avisos da crate ou de seus componentes nativos.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i rustls --depth 2` selecionou
`rustls 0.23.46` via `aws-smithy-http-client`, `hyper-rustls` e `tokio-rustls`.
A crate não é vendorizada na archive-fonte candidata; a seleção no grafo não
estabelece quais arquivos integram um binário ou quais avisos se aplicam a um
artefato final. Esta verificação não abrange o candidato Go.

No índice, os seis arquivos `syn` marcados como `origin` correspondem a
`src/discouraged.rs`, `src/parse.rs` e `tests/test_parse_buffer.rs` em cada uma
das versões `2.0.119` e `3.0.9`. Os hashes das seis cópias locais coincidiram
com os registros do índice. As ocorrências examinadas descrevem cursores ou
fluxos de análise derivados de outros cursores ou fluxos, inclusive em testes;
não são declarações de origem de código de terceiros. Isso classifica somente
as linhas indexadas e não completa a revisão dos avisos das duas versões.

Na extração do candidato Rust `8avAZthl`, consultas `cargo tree --offline
--locked --workspace --all-features --target x86_64-unknown-linux-gnu -i
syn@<versão> --depth 2` selecionaram ambas as versões: `2.0.119` por macros
como `aws-smithy-runtime-api-macros` e `tracing-attributes`, e `3.0.9` por
componentes como `serde_derive` e `tokio-macros`. As crates não são
vendorizadas na archive-fonte candidata; uso na compilação não prova inclusão
de seus códigos ou testes num binário final. A análise não cobre o candidato Go
nem determina os notices de outras dependências ou de artefatos finais.

No índice de `errno 0.3.14`, os três arquivos marcados como `origin`
(`src/unix.rs`, `src/windows.rs` e `src/wasi.rs`) foram conferidos contra os
hashes registrados, todos coincidentes. Os cabeçalhos Unix e Windows atribuem
adaptação a arquivos da distribuição Rust e creditam The Rust Project
Developers, respectivamente em 2015 e 2014; o cabeçalho WASI aponta adaptação de
`unix.rs` e credita 2015. Os três cabeçalhos referem licença MIT ou Apache-2.0;
o manifesto da crate declara `MIT OR Apache-2.0`. Os comentários não identificam
as revisões Rust de origem. Esta é uma revisão dos trechos indexados, não uma
conclusão sobre todos os avisos ou a escolha de licença por artefato.

Na extração do candidato Rust `8avAZthl`, as consultas inversas offline de
`errno` com `--target x86_64-unknown-linux-gnu` não retornaram dependentes nem
para o workspace nem para o exemplo separado de telemetria. Já `cargo tree
--offline --locked --all-features --target all -i errno --depth 3` no exemplo
selecionou `errno 0.3.14` via `rustix 1.1.6` e `tempfile 3.27.0`, incluindo
arestas de desenvolvimento. `--target all` é uma união de grafos, não prova
seleção num alvo específico ou inclusão num binário. A crate não está
vendorizada na archive-fonte candidata; os notices aplicáveis dependem da
composição efetiva. Esta revisão não abrange o candidato Go.

No índice de `tinystr 0.8.5`, os três arquivos marcados como `origin` foram
conferidos contra os hashes registrados, todos coincidentes. Os arquivos
`benches/common/mod.rs` e `benches/construct.rs` declaram adaptação de partes
de `https://github.com/zbraniecki/tinystr`, sem fixar revisão. O arquivo
`tests/serde.rs` atribui testes à revisão
`4e4eab55dd6bded7f29a18b41452c506c461716c` daquele repositório. O teste
[nesse commit](https://github.com/zbraniecki/tinystr/blob/4e4eab55dd6bded7f29a18b41452c506c461716c/tests/serde.rs)
foi consultado e contém testes de round-trip semelhantes; isso não estabelece
identidade integral ou resolve direitos das adaptações. O manifesto do commit
declara literalmente `Apache-2.0/MIT`, enquanto o manifesto de `tinystr 0.8.5`
declara `Unicode-3.0`. A aplicabilidade dos textos e avisos aos testes e
benchmarks requer revisão; essas declarações não substituem a análise por
artefato. Esta classificação não cobre todos os demais arquivos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i tinystr --depth 2`
selecionou `tinystr 0.8.5` via `icu_locale_core 2.4.0`. A crate não é
vendorizada na archive-fonte; o grafo não comprova inclusão de testes ou
benchmarks num binário. A revisão não abrange o candidato Go.

No índice de `icu_locale_core 2.4.0`, os três arquivos marcados como `origin`
foram conferidos contra os hashes registrados, todos coincidentes. As ocorrências
em `src/langid.rs:303` e `src/locale.rs:306` descrevem ordenações derivadas dos
campos de estruturas; a ocorrência em
`src/preferences/extensions/unicode/keywords/mod.rs:5` descreve preferências
derivadas de palavras-chave de locale. Elas não declaram origem de código de
terceiros. Esta classificação cobre somente os trechos indexados e não esgota os
textos, atribuições ou avisos da crate.

Na extração Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i icu_locale_core --depth 2`
selecionou `icu_locale_core 2.4.0` via `icu_properties` e `icu_provider`. A
crate não é vendorizada na archive-fonte candidata; a seleção no grafo não prova
quais arquivos integram um binário nem determina os notices finais. Esta
verificação não cobre o candidato Go.

No índice de `twox-hash 2.1.5`, os três arquivos marcados como `origin` foram
conferidos contra os hashes registrados, todos coincidentes. As ocorrências em
`src/xxhash3/streaming.rs`, `src/xxhash3_64.rs` e `src/xxhash3_128.rs`
descrevem valores derivados de uma semente, não atribuição de código de
terceiros. Esta classificação cobre somente os trechos indexados e não esgota
os avisos ou outros arquivos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -e features -i
twox-hash --depth 3` mostrou `twox-hash 2.1.5` com `xxhash32` via
`lz4_flex 0.14.0`, sem as features `xxhash3_64` ou `xxhash3_128` nessa consulta.
O `src/lib.rs` da crate condiciona os módulos xxhash3 a essas features. Isso
descreve apenas o grafo e a configuração examinados, não outras configurações
ou o conteúdo efetivo de um binário; a crate não está vendorizada na
archive-fonte candidata. Esta análise não abrange o candidato Go e não encerra
os avisos aplicáveis por artefato.

No índice de `bytes 1.12.1`, o único arquivo marcado como `origin`,
`tests/test_bytes.rs`, foi conferido contra o hash registrado e coincidiu. A
ocorrência na linha 971 descreve uma fatia derivada de um valor de dados, não
uma atribuição de código de terceiros. A classificação cobre apenas essa linha
indexada e não esgota os demais avisos ou arquivos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i bytes --depth 1`
selecionou `bytes 1.12.1`, inclusive por `aws-sdk-s3`, `http` e `tokio`, com
arestas de desenvolvimento. A crate não é vendorizada na archive-fonte; o grafo
não prova que o teste ou outros arquivos integrem um binário. Esta verificação
não abrange o candidato Go nem determina os notices dos artefatos finais.

No índice de `quote 1.0.47`, o único arquivo marcado como `origin`,
`src/format.rs`, foi conferido contra o hash registrado e coincidiu. O comentário
na linha 7 afirma que a sintaxe foi copiada da macro Rust `format!`; o trecho
examinado não afirma que a implementação tenha sido copiada. Esta classificação
é restrita à ocorrência indexada e não esgota as fontes ou avisos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i quote --depth 2`
selecionou `quote 1.0.47` por dependências de compilação, incluindo macros como
`serde_derive` e `tokio-macros`. A crate não é vendorizada na archive-fonte; o
grafo não prova que seu código integre um binário final. Esta revisão não cobre
o candidato Go nem determina os notices aplicáveis por artefato.

No índice de `once_cell 1.21.4`, o único arquivo marcado como `origin`,
`src/imp_std.rs`, foi conferido contra o hash registrado e coincidiu. Os
comentários nas linhas 1–4 e 172–173 informam que o código foi copiado de
`std::sync::Once` e posteriormente alterado; os trechos examinados não
identificam a revisão da biblioteca padrão Rust. O manifesto da crate declara
`MIT OR Apache-2.0`, e a cópia local contém `LICENSE-MIT` e `LICENSE-APACHE`.
Essas evidências não encerram a revisão dos avisos aplicáveis ao código
atribuído à biblioteca padrão, nem dos outros arquivos da crate.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i once_cell
--depth 2` selecionou `once_cell 1.21.4` via `rustls`, `tempfile` e
`tracing-core`, incluindo arestas de desenvolvimento. A crate não é vendorizada
na archive-fonte e o grafo não prova que o arquivo citado integre um binário.
Esta revisão não abrange o candidato Go nem determina notices finais.

No índice de `fastrand 2.5.0`, o único arquivo marcado como `origin`,
`src/lib.rs`, teve hash local coincidente com o registrado. Os comentários
`Adapted from` nas linhas 170, 188 e 206 apontam para um artigo de Daniel Lemire;
na linha 237 apontam para a resposta Stack Overflow `28904636`; e na linha 438
para a documentação `rand::seq::IteratorRandom::choose_multiple`. Essas
referências exigem revisão das fontes e dos avisos pertinentes; não identificam
por si só versões ou termos aplicáveis da adaptação.

A [API de revisões da resposta 28904636](https://api.stackexchange.com/2.3/posts/28904636/revisions?site=stackoverflow&filter=withbody&pagesize=100)
retornou duas revisões: a primeira, de 2015-03-06, com metadado `CC BY-SA 3.0`,
e a segunda, de 2021-06-25, com `CC BY-SA 4.0`. A API atual identifica o autor
como `craigster0`. Não foi estabelecida a revisão consultada por `fastrand`,
nem a relação desses metadados com a adaptação e seus notices; não se conclui
automaticamente uma obrigação ou autorização de distribuição. A revisão das
outras fontes citadas e dos demais arquivos da crate permanece pendente.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i fastrand
--depth 3` selecionou `fastrand 2.5.0` via `aws-sdk-s3` e `tempfile`, entre
outros, incluindo arestas de desenvolvimento. A crate não é vendorizada na
archive-fonte; o grafo não prova que os trechos citados integrem um binário.
Esta revisão não cobre o candidato Go nem determina os notices finais.

No índice de `regex-lite 0.1.9`, os dois arquivos marcados como `origin` foram
conferidos contra os hashes registrados, ambos coincidentes. Em `src/pikevm.rs`,
as ocorrências descrevem valores e estados da execução, não atribuição de
fontes de terceiros. Em `src/utf8.rs:102–103`, o comentário atribui uma máquina
de estados a `bstr`, por sua vez adaptada da
[página de Björn Höhrmann sobre decodificação UTF-8](https://bjoern.hoehrmann.de/utf-8/decoder/dfa/).
A página consultada apresenta copyright 2008–2009 Bjoern Hoehrmann e um aviso de
permissão no estilo MIT, que pede preservação do copyright e do aviso em cópias
ou partes substanciais. A página é móvel; não foram identificadas as revisões
específicas da página ou de `bstr` usadas na adaptação. A declaração da página
não determina por si só os termos ou notices aplicáveis a qualquer artefato;
essa análise e a revisão dos demais arquivos da crate permanecem pendentes.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i regex-lite
--depth 2` selecionou `regex-lite 0.1.9` via `aws-sdk-s3 1.157.0`, incluindo
aresta de desenvolvimento. A crate não é vendorizada na archive-fonte; o grafo
não prova que o arquivo citado integre um binário. O candidato Go não foi
examinado nesta etapa.

No índice de `protobuf 3.7.2`, os dois arquivos marcados como `origin`,
`src/descriptor.rs` e `src/well_known_types/api.rs`, foram conferidos contra os
hashes registrados, ambos coincidentes. As ocorrências descrevem, respectivamente,
nome de pacote Go e versão principal derivados de metadados de pacote; não
atribuem código a terceiros. Essa classificação cobre apenas os trechos
indexados e não substitui a revisão das demais fontes e avisos da crate.

No exemplo Rust separado de telemetria extraído do candidato `8avAZthl`,
`cargo tree --offline --locked --all-features --target x86_64-unknown-linux-gnu
-i protobuf --depth 3` selecionou `protobuf 3.7.2` via `prometheus 0.14.0`.
Esta é evidência do grafo desse exemplo, não dos demais pacotes ou do conteúdo
de um binário; a dependência não é vendorizada na archive-fonte candidata.
Os avisos finais dependem dos artefatos escolhidos. A análise não abrange o Go.

No índice de `http 1.5.0`, os dois arquivos marcados como `origin` foram
conferidos contra os hashes registrados, ambos coincidentes. Em
`src/extensions.rs:33`, a ocorrência descreve dados derivados de um protocolo,
não origem de código. Em `src/uri/mod.rs:53–67`, a ocorrência está dentro de
uma citação de texto e gramática que corresponde à seção 5.3 da
[RFC 7230](https://www.rfc-editor.org/rfc/rfc7230.txt). O aviso da RFC credita
IETF Trust e autores e remete a BCP 78 e às disposições legais do IETF Trust;
a aplicabilidade e eventuais avisos para a citação e gramática devem ser
avaliados por artefato, sem presumir que o manifesto da crate os resolva. O
manifesto de `http 1.5.0` declara `MIT OR Apache-2.0`. Esta revisão cobre apenas
os trechos indexados, não todos os textos de terceiros da crate.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i http --depth 2` selecionou
`http 1.5.0` por dependências como `aws-sdk-s3` e `h2`, incluindo arestas de
desenvolvimento. A crate não é vendorizada na archive-fonte candidata; o grafo
não comprova que a documentação citada ou outros arquivos integrem um binário.
Esta análise não cobre o candidato Go nem encerra os notices por artefato.

No índice de `hashbrown 0.17.1`, os três arquivos marcados como `origin`
foram conferidos contra os hashes registrados, todos coincidentes. Em
`src/control/group/generic.rs:106–107`, o comentário atribui um algoritmo à
página [Bit Twiddling Hacks](https://graphics.stanford.edu/~seander/bithacks.html#ValueInWord).
A página consultada credita Sean Eron Anderson e declara os trechos individuais
de código em domínio público, salvo indicação em contrário, distinguindo-os da
coletânea e das descrições, às quais atribui copyright. Essa declaração da página
móvel não identifica a versão histórica consultada por `hashbrown`, nem decide
direitos, autoria ou avisos aplicáveis ao artefato. Os marcadores nos outros
dois arquivos (`src/raw.rs` e `src/table.rs`) descrevem relações internas, não
afirmações de origem de código de terceiros; a revisão cobre apenas as linhas
indexadas e não esgota os avisos da crate.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i hashbrown --depth 2`
selecionou `hashbrown 0.17.1` via `indexmap` e `lru`, incluindo arestas de
desenvolvimento. A crate não é vendorizada na archive-fonte candidata e o grafo
não prova inclusão do arquivo citado num binário. O candidato Go não foi
examinado nesta etapa; os notices por artefato permanecem pendentes.

No índice de `crc-fast 1.10.0`, os dois arquivos marcados como `origin` foram
conferidos contra os hashes registrados, ambos coincidentes. Em
`src/bin/checksum.rs:155–156`, a referência descreve uma semente de dados de
benchmark derivada do tamanho do buffer, não origem de código. Em
`src/combine.rs:3–34`, o cabeçalho atribui o código a Mark Adler, aponta para
a resposta Stack Overflow `29928573` e conserva um aviso de copyright 2015 e
permissão com três restrições sobre origem, alterações e preservação do aviso.

A [API pública da resposta](https://api.stackexchange.com/2.3/answers/29928573?site=stackoverflow&filter=withbody)
identificou Mark Adler como autor; a resposta atual contém também o copyright e
o texto de permissão e apresenta `content_license: CC BY-SA 4.0`. A API informou
última edição em 2022-06-08; não foi estabelecida a revisão efetivamente usada
por `crc-fast` nem a relação jurídica entre os textos. Ambas as evidências devem
ser consideradas na revisão apropriada, sem concluir automaticamente os termos
de distribuição ou eliminar o aviso preservado na fonte.

No candidato Rust `8avAZthl`, `cargo tree --offline --locked --workspace
--all-features --target x86_64-unknown-linux-gnu -i crc-fast --depth 2`
selecionou `crc-fast 1.10.0` via `aws-smithy-checksums` e `aws-sdk-s3`,
incluindo aresta de desenvolvimento. A crate não é vendorizada na archive-fonte;
o grafo não prova inclusão de `combine.rs` ou do benchmark num binário. Esta
verificação não abrange o candidato Go e não encerra os avisos por artefato.

Investigação posterior do histórico público: a
[API de revisões da resposta 29928573](https://api.stackexchange.com/2.3/posts/29928573/revisions?site=stackoverflow&filter=withbody&pagesize=100)
retornou sete revisões. A revisão 5, de 2015-04-29, contém o aviso de permissão
e identifica `crccomb.c` versão 1.0; a revisão 6, de 2015-04-30, contém o
aviso e identifica versão 1.1, como o cabeçalho da crate. Ambas apresentam o
metadado `CC BY-SA 3.0`. A revisão 7, de 2022-06-08, contém o aviso e versão
1.1 e apresenta `CC BY-SA 4.0`. Essas correspondências de cabeçalho não
estabelecem identidade integral do código ou qual revisão foi usada, nem
decidem a relação entre os metadados da plataforma e a permissão no código.

Os metadados `.cargo_vcs_info.json` de `crc-fast 1.10.0` apontam ao commit
`3a853cc7daf2cd47cc4466f198680cabdfb0b5fa` de
`https://github.com/awesomized/crc-fast-rust`. O
[histórico do caminho `src/combine.rs`](https://api.github.com/repos/awesomized/crc-fast-rust/commits?path=src/combine.rs&sha=3a853cc7daf2cd47cc4466f198680cabdfb0b5fa&per_page=100)
retornou quatro commits; no mais antigo retornado,
`e40895f9bf1ba7f8f9545cda566bf129ef091067`, de 2025-04-10, o arquivo já
continha a referência à resposta e o aviso de Mark Adler. O manifesto da crate
declara `MIT OR Apache-2.0`, mas a revisão do aviso incorporado e dos termos
aplicáveis permanece aberta. Essas evidências não provam inclusão do trecho em
um binário; a crate não é vendorizada na archive-fonte Rust candidata.

Investigação adicional da atribuição Apache Commons: o histórico público
registra o arquivo originalmente como `rust-runtime/smithy-xml/src/unescape.rs`,
adicionado no commit `6df93e093d74cb844eff9a289c5d65269a396577` em
2021-05-10, com o comentário sobre os testes já presente. O commit
`1aa59693eed10713dec0f3777db700c2e8f4cda4` o renomeou para o caminho
`rust-runtime/aws-smithy-xml/src/unescape.rs`. Fonte para a introdução:
`https://api.github.com/repos/smithy-lang/smithy-rs/commits/6df93e093d74cb844eff9a289c5d65269a396577`.
Esses marcos não identificam a versão Apache consultada pelo autor.

Duas fontes públicas com o nome citado, em revisões anteriores à introdução,
foram examinadas como candidatas, não como origem comprovada:
[`commons-lang`, commit `8e61b090ceaaf7b108f0df175e9edbeae8d37642`](https://github.com/apache/commons-lang/blob/8e61b090ceaaf7b108f0df175e9edbeae8d37642/src/test/java/org/apache/commons/lang3/StringEscapeUtilsTest.java)
e [`commons-text`, commit `7792db0f58dab20b93d43f9702c53c6351a27411`](https://github.com/apache/commons-text/blob/7792db0f58dab20b93d43f9702c53c6351a27411/src/test/java/org/apache/commons/text/StringEscapeUtilsTest.java).
Os dois cabeçalhos declaram Apache License 2.0 e remetem aos respectivos
`NOTICE.txt`; esses avisos identificam a Apache Software Foundation e os
produtos Commons Lang e Commons Text, respectivamente. Ambos contêm exemplos
de escape XML semelhantes; isso não prova qual foi usado nem identifica sua
revisão efetiva. A atribuição em Smithy está em código `#[cfg(test)]`; a crate
não é vendorizada no candidato Rust `8avAZthl`. Não se infere inclusão dos
testes em binários nem uma decisão definitiva sobre notices por artefato.

A triagem inicial pulou somente `libsqlite3-sys-0.39.1/sqlite3/sqlite3.c`, por
exceder 8 MiB. Ele foi examinado separadamente em streaming, após conferir a
archive contra o lockfile: o membro tem SHA-256
`0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52`, e foram
encontradas 144 linhas com `copyright` e 30 com as expressões de origem, sem
correspondências nos outros dois grupos. O índice separado
`sqlite-amalgamation-notice-index.json` no mesmo diretório tem SHA-256
`6d12e02f77a56285232f977cf8c17d6f0bac96ded6cfcd406d80267e32854ef7`.

Esses índices são triagem, não revisão completa: podem conter falsos positivos,
não cobrem outros padrões, tipos de arquivo, crates locais ou dependências de
teste ausentes do SBOM. Ainda é preciso revisar os trechos pertinentes e definir
quais fontes ou objetos efetivamente integram cada artefato final. A quantidade
de resultados não determina obrigações, segurança jurídica ou aprovação de
distribuição; os arquivos originais e candidatos foram preservados.

Na retomada, a contagem das categorias do próprio índice (não de linhas ou de
obrigações) identificou 2.704 arquivos com `copyright`, 2.301 com `spdx`,
211 com `license` e 298 com `origin`, respectivamente em 61, 21, 30 e 66
crates. Um mesmo arquivo pode aparecer em mais de uma categoria. `origin`
inclui, entre outras, `aws-lc-sys`, `ring`, `libsqlite3-sys`, `rustix` e
`lz4_flex`; as contagens incluem resultados técnicos sem relação com direitos.
Não foram revisadas todas as 298 ocorrências por esta contagem, que tampouco
inclui a amalgamação SQLite tratada separadamente. A revisão integral não deve
ser marcada como concluída com base apenas nesse agrupamento.

As dez linhas `origin` em nove arquivos de `ring 0.17.14` foram verificadas
contra os membros da archive do registry, SHA-256
`a4689e6c2294d81e88dc6261c768b63bc4fcdb852be6d1352498b114f61383b7`,
igual ao lockfile de telemetria. Três linhas (`chacha.rs`, `hmac.rs` e
`pbkdf2.rs`) descrevem derivação de valores ou chaves, não proveniência de
fontes. `montgomery_inv.c` referencia algoritmo de Henry S. Warren Jr.; seu
cabeçalho contém copyright 2016 Brian Smith e texto de permissão ISC.
`src/aead/chacha/fallback.rs` referencia código estream de D. Bernstein e
BoringSSL, com aviso ISC e copyrights Brian Smith e Google no cabeçalho.
`src/aead/poly1305.rs` aponta adaptação de teste BoringSSL. Três arquivos
pregenerados ARM64 dizem ser gerados de script BoringSSL e descrevem adaptação
de `gcm_init_v8`; sua inclusão depende do alvo e build. O `LICENSE` da crate
encaminha também a `LICENSE-BoringSSL`, `LICENSE-other-bits` e licenças de
`once_cell`. Esses achados são triagem limitada às linhas indexadas; não
substituem os textos, não estabelecem a composição de um binário e não encerram
a revisão de outros avisos, código gerado ou proveniência upstream.
No índice principal, os quatro marcadores `origin` de `libsqlite3-sys 0.39.1`
estão todos em `sqlite3/sqlite3.h`: descrevem SQL derivado de schema, nome de
entry point derivado de nome de arquivo e dados copiados para BLOB, não
declarações de proveniência. O membro SHA-256
`d6c84557261893439b341d2e6c35187ae3ea24fbfd843ad94616f72d89242b8c`
foi conferido contra a archive SHA-256
`14a5f01b87b92705d07b433c06f96cbc764637d81a0bffb5734ce17cf0f9a2b3`.
Essa classificação não cobre a amalgamação, indexada separadamente, e não
resolve sua referência a `rsc/fpfmt` nem outros avisos possíveis.

#### Referência `fpfmt` na composição Go

Na cópia Go do candidato `local-candidate-current-JIr2oOeH`, o módulo
`modernc.org/sqlite v1.61.1` é dependência declarada. Para o ambiente local
Linux amd64, `GOPROXY=off go list -f '{{join .GoFiles "\n"}}'
modernc.org/sqlite/lib` selecionou 126 arquivos; uma busca nos arquivos
selecionados encontrou a referência `github.com/rsc/fpfmt` e a definição de
`_sqlite3Fp10Convert2` em `sqlite_g_000000000001fea8.go` (linhas 12860–12863).
O arquivo consultado no cache tem SHA-256
`decab2056381635ad950b4dd449196607d72a938dfdb9075737a91ae62cdd84d`.
O comentário identifica uma adaptação de trabalho de Ross Cox, mas não a
revisão de origem ou os termos aplicáveis. O módulo de terceiros não está
vendorizado na cópia candidata. A seleção de arquivos pelo Go não é um mapa de
vinculação e não prova presença no binário final; revisar composição e notices
de qualquer artefato Go distribuído, além da análise Rust acima. Essa anotação
não altera nem aprova o candidato.

Na mesma dependência, `CHANGELOG.md` atribui a atualização para SQLite 3.54.1
à versão `modernc.org/sqlite v1.61.0`, anterior à `v1.61.1` selecionada. O
`vendor.json` de `v1.61.1` declara `lib/` proveniente de
`modernc.org/libsqlite3` commit `6124339364eb60da198c6d046265cc490f07fffa`,
tag `v1.15.2`, e informa o uso de `modernc.org/ccgo/v4 v4.37.0` na geração.
Esses são metadados do fornecedor, não uma comparação independente de todos os
bytes gerados com o commit de origem ou identificação do commit `fpfmt` usado.
O `vendor.json` consultado tem SHA-256
`28f9283081c7599e09eed30e6cf75c59aab5a0ec4455410acf1b224511000fa6`.

O `LICENSE-3RD-PARTY.md` da mesma cópia, SHA-256
`4194f8e7119be72babd499db4f6f1f7547e36e8a7bffcb2b9c8df241b8f1576a`,
descreve SQLite 3.54.1 traduzido em `lib/` e aponta para `LICENSE-SQLITE`;
busca literal por `fpfmt` ou `Ross Cox` nesse inventário não retornou linhas.
Isso não estabelece ausência de outros avisos ou incompatibilidade jurídica.
O próprio inventário registra `modernc.org/libc v1.79.0`, enquanto o `go.mod`
do candidato seleciona `v1.80.1`; portanto o inventário do fornecedor não deve
ser tratado automaticamente como lista completa da composição do candidato.
Na cópia isolada, `GOPROXY=off go mod verify` retornou `all modules verified`.
Os avisos efetivos exigem revisão por artefato e a proveniência específica da
adaptação continua pendente; nenhum candidato foi aprovado para distribuição.

O endpoint público `https://proxy.golang.org/modernc.org/libsqlite3/@v/v1.15.2.info`
associa a tag `v1.15.2` ao mesmo commit
`6124339364eb60da198c6d046265cc490f07fffa` declarado em `vendor.json`. A
archive obtida do proxy em `.../@v/v1.15.2.zip` tem 1.455 membros e SHA-256
`922664f0e30ce14347eeb30afc850ddfc67f3f7564d88e1cfa2d2c9bffedd660`;
isso é uma impressão digital do download, não verificação independente de
assinatura. Os dois membros cujo basename começa por `LICENSE`, `NOTICE`,
`COPYING` ou `COPYRIGHT` são `LICENSE` (SHA-256
`3ec5937b53ad5e87ebae86650ca26ccede7dee8e3aca6e169677b5ae3b31a758`, texto
BSD de três condições creditado aos autores de libsqlite3-go) e
`LICENSE-SQLITE.md` (SHA-256
`ee6af51062b30d532991face5164136ae6f84e265ecf8abe89dc69dac45ca1e7`,
informações de licença SQLite). O arquivo gerado selecionado no candidato,
`sqlite_g_000000000001fea8.go`, não aparece com esse basename na archive
consultada; não foi estabelecida equivalência byte a byte com a origem. A
presença desses textos e a declaração upstream sobre SQLite não resolvem por si
sós o aviso específico `fpfmt` ou direitos de terceiros. A cópia candidata não
vendoriza esse módulo; os avisos dos binários dependem de composição e revisão.
Uma busca posterior na mesma archive do proxy identificou a definição e a
referência a `fpfmt` no membro `ccgo_linux_amd64.go`. Extraindo somente a
definição de `_sqlite3Fp10Convert2`, da linha `func` até a chave de fechamento
na primeira coluna, obtiveram-se 2.100 bytes tanto nesse membro quanto em
`modernc.org/sqlite v1.61.1/lib/sqlite_g_000000000001fea8.go`; os fragmentos
são byte a byte idênticos, SHA-256
`7e2d61ec58f30cb4b8da71c96d022350763e9a720b585b629552dad91bba5d98`.
O bloco de comentário imediatamente anterior também coincide e menciona a
adaptação de trabalho de Ross Cox. O membro inteiro upstream tem SHA-256
`7eee83baa24a7d406fc969fe27cd6df7e0e60d7f0030e235b3c535b9d6946cbf`.
Essa verificação cobre apenas o fragmento selecionado e o comentário, não a
equivalência dos arquivos completos, o código SQLite anterior à tradução, a
revisão de `fpfmt`, sua licença ou a inclusão num binário distribuído. O Go
proxy identifica a tag e o commit como registrado acima; não foi feita
verificação independente de assinatura. Notices seguem pendentes por artefato.
Na archive do proxy de `modernc.org/libsqlite3 v1.15.2`, o changelog informa
SQLite 3.54.1 e o `Makefile` aponta para
`https://www.sqlite.org/2026/sqlite-amalgamation-3540100.zip`; o gerador
registra também transformações e patches, que não foram reproduzidos nesta
verificação. A archive obtida dessa URL tem SHA-256
`0a9c8e34352dad46006aa82836d5383bff9dc6bc9ce5a60553c05aa30e6ad1f9`; seu
único membro `sqlite3.c` tem SHA-256
`0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52`,
coincidente com o membro local de `libsqlite3-sys 0.39.1` examinado no
candidato Rust. Esse arquivo contém nas linhas 37619–37620 a atribuição a
Ross Cox e `rsc/fpfmt`. A igualdade de SHA-256 associa os bytes examinados ao
download, mas não substitui verificação independente de assinatura nem prova
equivalência integral dos arquivos Go gerados ou os termos da adaptação. Os
avisos por composição Go e Rust, e a revisão do `fpfmt`, continuam pendentes.
Na archive de `ring 0.17.14` foram identificados seis membros com `LICENSE` no basename:
`LICENSE` (`b3d73400…75d078a`), `LICENSE-BoringSSL` (`005fc765…f6c414d`),
`LICENSE-other-bits` (`f025ccfb…9989680f`), os dois arquivos
`src/polyfill/once_cell/LICENSE-APACHE` (`a60eea81…f759f2`) e `LICENSE-MIT`
(`6ee2ed6c…198deafc`), e `third_party/fiat/LICENSE` (`9eacbcb8…499fdd60f`).
Os hashes aqui são prefixos/sufixos para identificação; os textos completos e
sua aplicabilidade não foram revisados por esta listagem.

Na revisão das 30 linhas classificadas como possível origem em `sqlite3.c`,
várias se referem a cópias de valores ou a código interno SQLite, não a
atribuições. Uma ocorrência material, na linha 37619, informa que o algoritmo
`sqlite3Fp10Convert2` é adaptado de trabalho de Ross Cox e referencia
`https://github.com/rsc/fpfmt`. O `LICENSE` da branch `main` desse repositório,
consultado posteriormente, contém BSD de três condições com copyright 2009
The Go Authors; o SHA-256 do texto obtido foi
`2d36597f7117c38b006835ae7f537487207d8ec407aa9d9980794b2030cbc067`.
Essa branch é móvel e não foi identificado o commit consultado pelos autores
SQLite: a referência e a licença atual não demonstram a versão de origem ou
quais termos se aplicam à adaptação. É necessário revisar proveniência e textos
pertinentes para qualquer artefato que incorpore a amalgamação; não se conclui
que todos os 30 resultados sejam avisos de terceiros nem que a atribuição esteja
resolvida. O membro SQLite examinado está vinculado ao hash registrado acima.

Consulta adicional à API GitHub de histórico de `rsc/fpfmt/LICENSE` retornou
uma entrada para esse caminho: commit inicial
`b6f0da96e4722c144a8c24fab801fdee32860df2`, datado de 2025-12-20,
sem pais. O arquivo `LICENSE` obtido por esse commit contém copyright 2009
The Go Authors e as três condições já descritas. Fontes consultadas:
`https://api.github.com/repos/rsc/fpfmt/commits?path=LICENSE&per_page=10` e
`https://raw.githubusercontent.com/rsc/fpfmt/b6f0da96e4722c144a8c24fab801fdee32860df2/LICENSE`.
Isso fixa uma versão histórica do texto, mas não identifica o código nem o
commit adaptado pelo SQLite, nem decide a aplicabilidade dos termos. A archive
fonte Rust candidata não vendoriza `libsqlite3-sys`; a análise de uma eventual
distribuição com a amalgamação ou um binário que a incorpore permanece pendente.
Uma consulta à árvore desse commit inicial mostra apenas o arquivo `LICENSE`:
`https://api.github.com/repos/rsc/fpfmt/contents?ref=b6f0da96e4722c144a8c24fab801fdee32860df2`.
Portanto, esse commit fixa o texto da licença, mas não é uma versão contendo o
algoritmo adaptado. Ainda é necessário identificar uma revisão do código usada
pela adaptação SQLite antes de atribuir termos específicos a esse trecho.
O histórico GitHub de `src/util.c` identifica a introdução de
`sqlite3Fp10Convert2` no commit SQLite
`db84cdb747ac926d3f272933386fc6196c40cb19` (2026-02-14), cujo registro
Fossil é `64d9b7becf2da8927024adda278e837a46837e46af3cff51bd54a183d1716750`.
O histórico de `fpfmt.go` retornou cinco commits anteriores a essa data, de
`128895b583063db1aebf59b0605e27ecb5a2df8c` (2025-12-20, adição de código)
até `68ac29e874d5f34efe94c9d1b5a1463f81ffccd3` (2026-01-19). Fontes:
`https://api.github.com/repos/sqlite/sqlite/commits/db84cdb747ac926d3f272933386fc6196c40cb19`
e `https://api.github.com/repos/rsc/fpfmt/commits?path=fpfmt.go&per_page=100`.
São evidências cronológicas, não identificação da revisão efetivamente usada;
outros arquivos ou fontes também podem ser relevantes. A ressalva sobre o
candidato-fonte sem dependências vendorizadas e a revisão por binário permanece.
**Correção de escopo cronológico:** a introdução da função em fevereiro não
fixa a data da adaptação atual. O commit SQLite
`dfd31e158cc6cb0011036f03b8230eee485d5df0`, de 2026-03-22, adicionou o
comentário “The (current) algorithm is adapted from the work of Ross Cox” e a
URL de `fpfmt`; seu pai `cc19c1dafdc923dfc9ecc752899ad287479bc12f` não
contém essa referência em `src/util.c`. A mudança também altera a função;
o commit registra FossilOrigin
`8c3a1b450d6d5778004b9fb0e92b4f3f90d3e06ff87276370d01a45b7d607d23`.
Fonte: `https://api.github.com/repos/sqlite/sqlite/commits/dfd31e158cc6cb0011036f03b8230eee485d5df0`.
As cinco revisões anteriores a fevereiro são somente uma amostra, não o conjunto
comprovado de versões candidatas para a adaptação de março. O comentário não
informa um commit de `fpfmt`; direitos e notices continuam sem conclusão.
Consulta posterior ao histórico público de cada caminho em `rsc/fpfmt`, com
`until=2026-03-22T17:50:48Z` e `per_page=100`, retornou cinco commits para
`fpfmt.go` (o mais recente é `68ac29e874d5f34efe94c9d1b5a1463f81ffccd3`,
de 2026-01-19), dois para cada um de `pow10tab.go` e `pow10gen.go` (o mais
recente em ambos é `ad029f423a9fbbb4142416dd99eadaa7d6b329cc`, de
2026-01-14) e um para `LICENSE` (`b6f0da96e4722c144a8c24fab801fdee32860df2`).
Consultas no formato `https://api.github.com/repos/rsc/fpfmt/commits?path=<caminho>&until=2026-03-22T17:50:48Z&per_page=100`.
O filtro corrige a delimitação temporal da investigação desses quatro caminhos;
não identifica qual estado ou outra fonte os autores SQLite consultaram, não
comprova ancestralidade ou equivalência de um artefato, e não decide termos de
distribuição. A dependência não está vendorizada na archive-fonte candidata;
notices de binários exigem revisão da composição efetiva.
Comparação limitada dos fontes fixados mostra que a função `Parse` nas linhas
112–132 de `fpfmt.go` em `68ac29e874d5f34efe94c9d1b5a1463f81ffccd3` e
`sqlite3Fp10Convert2` nas linhas 1258–1304 do commit SQLite de introdução usam
etapas semelhantes de cálculo de expoente, escala e arredondamento. O comentário
no commit de introdução examinado descreve a operação, mas não identifica uma
revisão de `fpfmt`. Fontes fixas:
`https://raw.githubusercontent.com/rsc/fpfmt/68ac29e874d5f34efe94c9d1b5a1463f81ffccd3/fpfmt.go`
e `https://raw.githubusercontent.com/sqlite/sqlite/db84cdb747ac926d3f272933386fc6196c40cb19/src/util.c`.
A semelhança não prova que aquela revisão específica foi a fonte nem resolve os
termos aplicáveis; a revisão das outras versões e dos artefatos permanece aberta.
Uma comparação adicional extraiu somente a função `Parse` dos cinco arquivos
`fpfmt.go` fixados no histórico acima. Nas quatro revisões de 2026
(`bdca9fc333ad7932134c3ad4f7e5e83bd66ec9db`,
`5b224aeabcf8c67b18e2e79809c8ee9f9c96d83d`,
`4b2dc7ed54d659bb9add7d44edc210b66633680c` e
`68ac29e874d5f34efe94c9d1b5a1463f81ffccd3`) o texto extraído da função é
idêntico. Na revisão inicial `128895b583063db1aebf59b0605e27ecb5a2df8c`,
o texto difere, embora apresente as mesmas etapas gerais de cálculo. Foram
consultados os arquivos por URL fixada no padrão
`https://raw.githubusercontent.com/rsc/fpfmt/<commit>/fpfmt.go`.
Esta comparação não cobre as funções auxiliares ou tabelas e não distingue a
revisão efetivamente usada pela adaptação; não estabelece direitos ou licença
aplicável. A archive-fonte candidata não contém `libsqlite3-sys` vendorizado;
componentes em binários ou outras distribuições dependem do build escolhido.
Compararam-se também os textos completos de `pow10tab.go` nas mesmas cinco
revisões fixadas. As três últimas (`5b224aea`, `4b2dc7ed` e `68ac29e8`)
coincidem textualmente; as duas primeiras diferem da última em 28 linhas cada,
incluindo valores da tabela para potências de dez. Todos os cinco arquivos
foram obtidos com sucesso por URLs fixadas no padrão
`https://raw.githubusercontent.com/rsc/fpfmt/<commit>/pow10tab.go`. Isso não
compara a tabela com a amalgamação SQLite, não prova qual versão foi usada e
não decide a aplicabilidade de licença ou notice a qualquer artefato.
Comparação posterior, restrita à tabela do commit SQLite de introdução
`db84cdb747ac926d3f272933386fc6196c40cb19` em `src/util.c`, extraiu 696
constantes de `sqlite3PowerOfTen` e as comparou, na mesma ordem de expoentes
(-348 a 347), com a primeira palavra de cada uma das 696 entradas de
`pow10Tab` nas cinco revisões fixadas acima. Cada comparação encontrou 668
valores diferentes; no primeiro valor, SQLite tem `0xfa8fd5a0081c0288` e
`fpfmt` tem `0xfa8fd5a0081c0289`. O SQLite dessa revisão usa uma tabela de
64 bits, enquanto a tabela `fpfmt` examinada contém pares de palavras. Essa
comparação de valores não estabelece derivação, direitos ou revisão usada,
nem compara a amalgamação empacotada posteriormente. Os fontes fixados são
os mesmos URLs por commit registrados acima; a revisão por artefato continua
pendente, inclusive para a amalgamação e binários que a incorporem.
Na amalgamação `libsqlite3-sys 0.39.1` já identificada pelo hash acima,
`pwr10to2` e `pwr2to10` (linhas 37568–37569) usam respectivamente as
expressões `(p*108853) >> 15` e `(p*78913) >> 18`. As cinco revisões
fixadas de `fpfmt.go` contêm as mesmas constantes e deslocamentos nas
auxiliares correspondentes, como a consulta aos arquivos por commit mostrou.
Essa correspondência matemática não identifica a revisão usada nem prova
autoria, cópia ou termos jurídicos. A archive-fonte candidata não vendoriza a
amalgamação; a presença e os avisos em binários e outros artefatos devem ser
verificados por composição e alvo efetivos.
No commit SQLite de introdução `db84cdb747ac926d3f272933386fc6196c40cb19`,
`sqlite3Fp10Convert2` nas linhas 1278–1303 inclui a expressão de arredondamento
`(out + 1 + ((out>>2)&1)) >> 2` e o empacotamento da mantissa e expoente. Em
`fpfmt.go` no commit candidato `68ac29e874d5f34efe94c9d1b5a1463f81ffccd3`,
`round` (linha 64) e `pack64` (linhas 41–49) apresentam operações semelhantes;
`prescale` e `uscale` (linhas 292–309) usam uma tabela de duas palavras e
tratamento de bits adicionais. Essa comparação é parcial e envolve a versão
histórica de `src/util.c`, não uma prova de identidade com a amalgamação
empacotada posteriormente. Correspondências não identificam a revisão de origem
nem determinam direitos ou notices. Os URLs fixados das fontes constam acima.
Na amalgamação empacotada `libsqlite3-sys 0.39.1`, linhas 37642–37657,
`sqlite3Fp10Convert2` calcula produtos intermediários, um bit `sticky`, uma
subtração condicional e arredondamento. As funções `uscale` extraídas das cinco
revisões fixadas de `fpfmt.go` também contêm produtos intermediários e lógica
`sticky` semelhante. O texto integral de `uscale` é idêntico nas duas últimas
revisões (`4b2dc7ed` e `68ac29e8`); difere textualmente nas três anteriores.
A comparação usou o membro SQLite identificado pelo hash já registrado e as
URLs fixas de `fpfmt.go` acima. Ela não prova equivalência funcional, cópia,
origem de uma revisão específica ou direitos. A archive-fonte candidata não
vendoriza a dependência; notices em binários dependem do build efetivo.

As 144 linhas da categoria `copyright` do índice da amalgamação foram
conferidas contra o arquivo cujo hash está acima. Há apenas dois textos de
linha distintos: 143 ocorrências de `The author disclaims copyright to this
source code. In place of` e uma de `The author disclaims copyright to this
source code.`. O contexto da primeira no início de `sqliteInt.h` inclui a
bênção SQLite; a linha isolada aparece no cabeçalho `fts3_tokenizer.h`, dentro
de uma seção condicional FTS3. Essa contagem por palavra não analisa todos os
contextos ou os demais arquivos da crate, não elimina avisos sem a palavra
`copyright` e não resolve a referência a Ross Cox descrita acima. Tampouco
estabelece quais seções condicionais compõem um binário final ou substitui a
revisão dos textos completos e das obrigações aplicáveis por artefato.

Na archive `rustix 1.1.6` selecionada pelo exemplo Rust, foram conferidos os
cabeçalhos de três arquivos: `src/weak.rs` identifica derivação da biblioteca
padrão Rust na revisão `fd0cb0cdc21dd9c06025277d772108f8d42cb25f`;
`src/types.rs` identifica cópia de `rust-lang/libc` na revisão
`008f905256d27eb5dd416d0976678ee6c2855554`; e
`src/maybe_polyfill/no_std/mod.rs` declara que a subárvore deriva da biblioteca
padrão e é licenciada MIT ou Apache 2.0. O último arquivo descreve sua seleção
para builds sem `std`; essa afirmação não determina as features do consumidor.
Os três arquivos examinados no cache coincidem com os membros da archive, que
corresponde ao checksum do lockfile. A busca textual por declarações semelhantes
em `.rs` encontrou outros arquivos `rustix` e nenhuma correspondência nos
arquivos `.rs` de `linux-raw-sys`, sob os padrões consultados; isso não exclui
outras formulações ou tipos de arquivo. A composição de um binário e seus
avisos aplicáveis seguem por revisar.

Revisão adicional dos outros nove arquivos `rustix 1.1.6` classificados como
`origin` no índice: os hashes das cópias locais coincidem com os nove hashes
registrados no índice. `src/backend/libc/io/syscalls.rs` atribui funções à
biblioteca padrão Rust na revisão `326ef470a8b379a180d6dc4bbef08990698a737a`;
`src/fs/seek_from.rs` aponta a revisão
`dca3f1b786efd27be3b325ed1e01e247aa589c3b`. Os sete arquivos restantes em
`src/maybe_polyfill/no_std/` identificam fontes e revisões da biblioteca padrão
Rust, inclusive três arquivos sob `os/windows/`; os cabeçalhos examinados
declaram MIT ou Apache-2.0 para esses arquivos. Essas são declarações dos
próprios fontes, não verificação independente de autoria ou de todos os avisos.

Na extração do candidato Rust `8avAZthl`, `cargo tree --offline --locked
--workspace --all-features --target x86_64-unknown-linux-gnu -i rustix`
selecionou `rustix 1.1.6` via `tempfile 3.27.0`, incluindo também arestas de
desenvolvimento. Isso não comprova que os arquivos Windows, `no_std` ou o
backend libc sejam compilados ou vinculados num binário Linux. A archive-fonte
do candidato não vendoriza `rustix`; os avisos finais continuam dependentes da
composição real. Esta verificação não abrange o candidato Go nem resolve as
questões abertas de `vsimd` e `fpfmt`.

No mesmo conjunto de 213 archives de registry do SBOM de telemetria Rust, uma
busca independente por membros regulares cujo basename comece por `NOTICE`,
sem distinção de maiúsculas, retornou **zero arquivos**; as archives foram
novamente conferidas contra os checksums do lockfile antes da busca. Isso não
exclui avisos embutidos, arquivos com outros nomes, dependências de teste
omitidas do SBOM ou notices presentes somente upstream. Em particular, a
triagem `s3-aws-lc-transitive-notices.md` identifica notices upstream AWS-LC
que não constam da archive `.crate`; ausência local não elimina a necessidade
de revisá-los para qualquer artefato que incorpore os componentes pertinentes.
Nenhuma conclusão de distribuição decorre dessa contagem.

Em `utf8_iter 1.1.0`, o cabeçalho de `src/indices.rs` identifica adaptação de
`CharIndices` da biblioteca padrão Rust na revisão
`ab32548539ec38a939c1b58599249f3b54130026` e reproduz trecho do respectivo
aviso de autoria e referências às licenças Apache-2.0 e MIT. O SHA-256 desse
arquivo é `80d354f207007420d90391a161c7bddec57f76db63a596ee0a4831db1c383216`;
seus bytes na cópia do registry coincidem com o membro da archive, cujo checksum
corresponde ao lockfile. Não há membros regulares sob `tests/` nessa archive;
isso não exclui testes embutidos em fontes ou com outras localizações. O
`COPYRIGHT` menciona domínio público/CC0 somente para trechos de teste assim
designados; não se estendeu essa declaração ao arquivo `indices.rs`. A presença
no grafo não prova sua inclusão em um binário distribuído, e a revisão de
atribuições continua dependente da composição do artefato escolhido.

Em `utf_types 1.0.0/src/utf8.rs`, um comentário anterior ao módulo
`#[cfg(test)]` declara dedicar ao domínio público os direitos sobre o código
de teste abaixo dele, referenciando CC0 1.0; não se aplica essa declaração ao
restante do arquivo. Os bytes examinados correspondem ao membro da archive,
que coincide com o checksum do lockfile; SHA-256 de `src/utf8.rs`:
`1beed1d7bb3e8f75125db543894a57c1773484001e8c82637a6cb3a8e6c02e2e`.
Uma busca textual por `public domain`, `CC0` e `dedicat` nas árvores extraídas
de `utf_types` e `utf8_iter` também encontrou referências nos respectivos
`COPYRIGHT` e em `utf_types/CONTRIBUTING.md`, mas nenhum trecho designado
adicional na árvore de `utf8_iter` por esses termos. Isso não exclui avisos
expressos de outra forma, nem prova inclusão de testes no artefato final.

Nas cópias isoladas já testadas, em Linux x86-64 e sem rede externa:

* Rust: `cargo tree --offline --locked --all-features --target
  x86_64-unknown-linux-gnu --prefix none --format '{p}' -e normal,build`
  produziu 215 pares únicos de pacote/versão, incluindo o pacote raiz e os
  locais. Comparados por nome/versão com `rust-telemetry.cdx.json` histórico,
  somente o próprio pacote raiz não apareceu como componente. O comando com
  `-e normal,build,dev` produziu 224 pares: os nove adicionais são
  `futures-executor 0.3.34`, `futures-macro 0.3.34`, `getrandom 0.3.4`,
  `opentelemetry_sdk 0.33.1`, `ppv-lite86 0.2.21`, `rand 0.9.5`,
  `rand_chacha 0.9.0`, `rand_core 0.9.5` e `zerocopy 0.8.63`.
  Comparar somente nome/versão não verifica checksums, features, relações,
  licenças, código nativo nem bytes efetivamente vinculados.
* Go: `go list -mod=readonly -deps` para o módulo do exemplo selecionou 32
  módulos com versão, inclusive `sqlite-replication v0.0.0` substituído pela
  cópia local. Com `-test`, selecionou 38: os seis adicionais foram
  `github.com/go-logr/logr v1.4.4`, `github.com/go-logr/stdr v1.2.2`,
  `go.opentelemetry.io/auto/sdk v1.2.1`, `go.opentelemetry.io/otel/log v1.47.0`,
  `go.opentelemetry.io/otel/sdk v1.47.0` e
  `go.opentelemetry.io/otel/sdk/metric v1.47.0`. O resultado depende do alvo
  `linux/amd64`, do Go 1.27.3 e do replacement local; não é link map.

Estas consultas reproduzíveis são inventários de seleção, não SBOMs CycloneDX
novos nem revisão completa dos textos. Para cada artefato final ainda é preciso
gerar SBOM vinculado ao build e revisar notices e fontes relevantes; os SBOMs
originais e em quarentena permanecem intactos.

#### Inventários novos do módulo principal Go

Na cópia isolada Go/fixtures, `cyclonedx-gomod 1.12.0` gerou com Go 1.27.3,
`linux/amd64`, `GOPROXY=off` e `GOSUMDB=off` os inventários `mod -json
-licenses -type library`, sem e com `-test`, em caminhos novos sob
`/tmp/opencode/cyclonedx-refresh-iPmrUZod/generated/`. O SHA-256 da archive
do gerador, conferido com o release upstream e o registro anterior, foi
`004b9f5cc595b797fb5423e2ae4c97bcf0f18c712ed2faee1640b09e5efd6d15`.

| Arquivo | Componentes | SHA-256 |
|---|---:|---|
| `go-main.json` | 22 | `d2e87197c3e1d99bd1ae7c82fd9e7d543635f65af87c84fb6577fc17c6f7a706` |
| `go-main-with-tests.json` | 29 | `881b655836ba99f781076b53de18d40379988ac19c33b31956729e84df07b281` |

Ambos passaram `cyclonedx validate --input-format json --input-version v1_6
--fail-on-errors` com a CLI verificada acima. O gerador avisou que não pôde
determinar a versão do módulo raiz porque a cópia não é um repositório Git e
que não detectou sua licença. Detecções de terceiros são evidências, não
conclusões jurídicas, e continuam sujeitas à quarentena documentada para
classificações controversas. Essas saídas temporárias não são SBOMs de release.

Na mesma configuração offline, a tentativa equivalente para o exemplo Go de
telemetria falhou em `go list -mod readonly -json -m all`; a reprodução direta
retornou `module lookup disabled by GOPROXY=off`. Dois arquivos de saída vazios
foram criados pela redireção e não são SBOMs. A repetição com
`GOPROXY=https://proxy.golang.org` e `GOSUMDB=sum.golang.org`, em novos caminhos,
teve sucesso e ambos passaram a validação CycloneDX 1.6 com a CLI verificada:

| Arquivo em `generated/` | Componentes | SHA-256 |
|---|---:|---|
| `go-telemetry-online.json` | 25 | `50d6022acdda0785d1745f713ba1eafd71a82db099c2a991fbea38ea0412b2fe` |
| `go-telemetry-online-with-tests.json` | 40 | `fceaedbfedf0df4fd95fac551dfaa32601277ebcd89011667c2ed17c00e9f981` |

O gerador avisou que não identificou versões e licenças dos módulos locais da
cópia sem Git. A geração online não comprova disponibilidade offline em outro
host. As evidências automáticas de licença requerem revisão, inclusive as
classificações controversas já colocadas em quarentena nas saídas históricas;
nenhuma saída nova foi aprovada para distribuição. Os SBOMs históricos e cópias
em quarentena não foram substituídos.

#### Comparação e quarentena das novas evidências Go

A comparação por `(group, name, version)` dos componentes novos com os SBOMs
históricos correspondentes encontrou conjuntos idênticos para o módulo principal
com e sem testes. Nos dois inventários de telemetria, a única diferença nesse
conjunto foi o módulo local `sqlite-replication`: o histórico usava a
pseudoversão `v0.0.0-20261105182955-f5c0984f0694`, enquanto a cópia sem Git
deixou a versão ausente. Essa comparação não verifica relações ou propriedades.

As evidências controversas reapareceram em `go-main.json`,
`go-main-with-tests.json` e `go-telemetry-online-with-tests.json`: o detector
indicou MIT para `modernc.org/libc v1.80.1` e Apache-2.0 para
`modernc.org/sqlite v1.61.1`. Os arquivos principais upstream têm textos BSD
de três condições, como detalhado em `license-detector-investigation.md`;
nenhuma dessas evidências isoladas cobre todos os textos de terceiros.
O inventário de telemetria sem testes não contém esses dois componentes.

O script `conformance/quarantine-sbom-license-evidence.py` criou três cópias
novas no mesmo diretório temporário; comparação estrutural confirmou que apenas
as duas entradas `evidence.licenses` foram removidas em cada uma. Todas passaram
`cyclonedx validate --input-format json --input-version v1_6 --fail-on-errors`:

| Cópia derivada | SHA-256 |
|---|---|
| `go-main-quarantined.json` | `b8462e49d3a7d3d8b129977237130a37fc5db2edf703198685318b2e9ad6b1f1` |
| `go-main-with-tests-quarantined.json` | `8ad55a2558533b74379de72e1d85bd5a3636695cacff3f656d7c8e16f5a38267` |
| `go-telemetry-online-with-tests-quarantined.json` | `e563e4fb723ca6acf87c259fff4968a80340a9802c2f7f18a419a1bc1b445fd7` |

Os originais novos e históricos foram preservados. Ausência de evidência não
é atribuição de licença, e a quarentena não aprova notices ou distribuição;
persistem outras evidências automáticas e a revisão por artefato final.

Para preservar a evidência além do diretório temporário, os quatro inventários
Go e as três cópias derivadas em quarentena listados acima foram copiados para
`~/git-worktrees/sqlite-replication/sbom-refresh-20261106-evidence/`, em um
diretório novo no filesystem persistente de `/home`. Antes e depois da cópia,
os sete SHA-256 foram conferidos com as tabelas acima; os arquivos não foram
normalizados nem alterados. As saídas temporárias e os SBOMs históricos foram
preservados. O SBOM Rust atualizado permanece na extração persistente do
candidato, no caminho indicado anteriormente. Esses diretórios são evidências
locais, não componentes de um pacote final nem autorização de distribuição.

Comparação adicional dos valores de `evidence.licenses` por nome de componente
comum aos quatro pares de inventários novo/histórico encontrou **zero diferenças**
em cada par. Isso inclui as duas classificações controversas acima e não prova
que as demais sejam corretas. Nos inventários novos não derivados, todos os
componentes do módulo principal têm alguma licença declarada ou evidência de
licença; nos dois inventários do exemplo, o componente da substituição local
`sqlite-replication` não tem nenhuma. A contagem não inclui o componente raiz
em `metadata.component`, não verifica conteúdo de licença ou avisos por arquivo
e não converte detecções em conclusões jurídicas. A revisão de textos principais
e avisos específicos está em `native-notices-review.md`, na triagem de telemetria
e nos demais documentos referenciados; identificar todos os avisos aplicáveis
a um artefato definido continua pendente.

#### Textos adicionais selecionados para testes Go

Os módulos `github.com/go-logr/logr v1.4.4` e `github.com/go-logr/stdr v1.2.2`
foram selecionados apenas ao incluir testes no exemplo Go isolado. Os arquivos
`LICENSE` das cópias locais foram lidos integralmente e contêm Apache 2.0; os
SHA-256 são, respectivamente,
`b40930bbcf80744c86c46a12bc9da056641d722716c378f5659b9e555ef833e1` e
`c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`.
Os cabeçalhos examinados em `logr.go` e `stdr.go` creditam 2019 The logr
Authors e referenciam Apache 2.0. A presença na seleção de testes não comprova
inclusão em qualquer artefato final; outros arquivos, avisos embutidos e a
aplicabilidade dos textos seguem sujeitos a revisão. Nenhum notice foi aprovado.

Na mesma seleção de testes Go, `go.opentelemetry.io/otel/log v1.47.0`,
`go.opentelemetry.io/otel/sdk v1.47.0` e `go.opentelemetry.io/otel/sdk/metric
v1.47.0` possuem `LICENSE` com SHA-256 idêntico
`1ae07514be1d7bb33f0698f8d91fb51b8b9fe1463157ec1c72081a49b9bc6f40`.
O texto foi lido integralmente: após Apache 2.0, inclui também um texto BSD de
três condições com copyright 2009 The Go Authors e seu disclaimer. A licença
na raiz de `go.opentelemetry.io/auto/sdk v1.2.1` tem SHA-256
`c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4`,
idêntico ao texto Apache 2.0 de `stdr` lido acima. As árvores examinadas não
foram integralmente analisadas por avisos embutidos; os textos originais não
são substituídos por este resumo. O conjunto de testes selecionado não prova
que esses módulos integrem o artefato a distribuir, e a aplicabilidade dos
avisos permanece por decidir por artefato.

Uma busca literal por `The Go Authors` nos arquivos `.go` das três árvores
locais `otel/log`, `otel/sdk` e `otel/sdk/metric` acima não retornou resultados.
Uma consulta separada por copyright encontrou cabeçalhos de The OpenTelemetry
Authors, mas foi truncada. A ausência da frase nos arquivos `.go` não explica
por que o texto BSD consta no `LICENSE`, não exclui outros arquivos ou avisos
nem permite eliminar o texto de um pacote de distribuição futuro.

Busca recursiva nas quatro árvores locais `auto/sdk`, `otel/log`, `otel/sdk` e
`otel/sdk/metric` identificou somente o `LICENSE` de raiz de cada módulo entre
os basenames iniciados por `LICENSE`, `NOTICE` ou `COPYING`, sem distinção de
maiúsculas; os quatro hashes correspondem aos registrados acima. A busca não
examina avisos embutidos ou arquivos com outros nomes e não substitui a revisão
de licenças e atribuições para a composição final de cada artefato.

#### Textos adicionais selecionados para testes Rust

Nas nove crates adicionais de desenvolvimento enumeradas acima, as archives do
cache coincidiram com os checksums do lockfile do exemplo. Busca em todos os
membros regulares por basename iniciado por `COPYRIGHT` encontrou um arquivo
em cada uma de `rand 0.9.5`, `rand_chacha 0.9.0` e `rand_core 0.9.5`; as outras
seis não tinham membros com esse prefixo. Os três arquivos são idênticos,
SHA-256 `90eb64f0279b0d9432accfa6023ff803bc4965212383697eee27a0f426d5f8d5`,
e coincidem com as cópias locais extraídas. O texto completo informa que os
direitos permanecem com contribuidores, remete autoria completa ao histórico
e identifica código do projeto Rust sob as mesmas alternativas MIT ou
Apache-2.0, ressalvadas exceções em arquivos individuais. Estas dependências
são adicionais na seleção com desenvolvimento; os candidatos-fonte não as
vendorizam. A busca por nome não cobre avisos embutidos, nem demonstra que
integrem um artefato final. A decisão de incluir fontes de teste, binários de
teste ou apenas artefatos de runtime permanece com o mantenedor e determina a
revisão de avisos correspondente.

Na cópia local do registry, os manifests de `futures-executor 0.3.34`,
`futures-macro 0.3.34`, `getrandom 0.3.4`, `ppv-lite86 0.2.21`, `rand 0.9.5`,
`rand_chacha 0.9.0` e `rand_core 0.9.5` declaram `MIT OR Apache-2.0`; cada
raiz examinada contém arquivos `LICENSE-MIT` e `LICENSE-APACHE`.
`zerocopy 0.8.63` declara `BSD-2-Clause OR Apache-2.0 OR MIT` e contém os
três arquivos correspondentes. `opentelemetry_sdk 0.33.1` declara Apache-2.0,
mas não tinha arquivo com prefixo `LICENSE`, `NOTICE` ou `COPYING` na raiz;
a revisão prévia da archive e do texto upstream está em
`telemetry-examples/upstream-notices-review.md`.

Foram relidos integralmente três textos locais: o MIT de `futures-executor`
credita Alex Crichton e The Tokio Authors (SHA-256
`6652c868f35dfe5e8ef636810a4e576b9d663f3a17fb0f5613ad73583e1b88fd`); o MIT
de `rand` credita Developers of the Rand project e The Rust Project Developers
(`209fbbe0ad52d9235e37badf9cadfe4dbdc87203179c0899e738b39ade42177b`);
o BSD de `zerocopy` credita The Fuchsia Authors
(`83c1763356e822adde0a2cae748d938a73fdc263849ccff6b27776dff213bd32`).
Os textos completos incluem condições de preservação e disclaimers; este resumo
não os substitui. Uma busca recursiva nas nove árvores locais, por basenames
iniciados por `LICENSE`, `NOTICE` ou `COPYING`, sem distinção de maiúsculas,
não encontrou arquivos aninhados. Ela não examina avisos embutidos ou nomes
diferentes. A leitura dos demais textos continua pendente. Estas dependências
adicionais foram selecionadas ao incluir
desenvolvimento; isso não prova sua inclusão no artefato a distribuir nem decide
qual alternativa de licença se aplica. Não houve aprovação de notices.

Também foram lidos integralmente os textos MIT locais de `getrandom`,
`ppv-lite86` e `zerocopy`: creditam respectivamente The rust-random Project
Developers e The Rust Project Developers; The CryptoCorrosion Contributors;
e The Fuchsia Authors. Os SHA-256 respectivos são
`29e9fe5074bd27e0e5d5d110394fbbcd841baee2651a3c4b4560a632702cede4`,
`4cada0bd02ea3692eee6f16400d86c6508bbd3bafb2b65fed0419f36d4f83e8f` e
`1a2f5c12ddc934d58956aa5dbdd3255fe55fd957633ab7d0d39e4f0daa73f7df`.
Os textos MIT de `futures-macro`, `rand_chacha` e `rand_core` têm os mesmos
hashes, respectivamente, dos textos de `futures-executor` e `rand` já lidos;
isso identifica bytes iguais no cache, sem validar sua procedência upstream.
Nenhuma alternativa de licença foi selecionada: as opções Apache e BSD
pertinentes ainda devem ser avaliadas, junto com avisos embutidos e escopo real
de cada artefato. Não se trata de aprovação jurídica ou de distribuição.

Foram também lidos integralmente os textos `LICENSE-APACHE` locais de
`futures-executor 0.3.34` (SHA-256
`275c491d6d1160553c32fd6127061d7f9606c3ea25abfad6ca3f6ed088785427`) e
`rand 0.9.5` (`35242e7a83f69875e6edeff02291e688c97caafe2f8902e4e19b49d3e78b4cab`).
O primeiro inclui apêndice com créditos de Alex Crichton e The Tokio Authors;
o segundo termina nos termos, sem esse apêndice. A seção de redistribuição
trata, entre outros pontos, da entrega da licença, de arquivos modificados e
dos avisos aplicáveis, inclusive quando existe `NOTICE`. Os resumos não
substituem os textos. Os arquivos Apache locais de `futures-macro` e
`rand_chacha` têm, respectivamente, os mesmos hashes desses dois textos; isso
não determina sua aplicação a um artefato específico. Os demais textos Apache,
avisos embutidos e a procedência upstream ainda exigem revisão.

O `LICENSE-APACHE` de `getrandom 0.3.4` também foi lido integralmente no cache;
seu SHA-256 é `aaff376532ea30a0cd5330b9502ad4a4c8bf769c539c87ffe78819d188a18ebf`.
O arquivo contém os termos Apache 2.0 e um apêndice com campos exemplificativos
de copyright, não uma atribuição preenchida. A referência não substitui os
avisos da própria crate ou a revisão do artefato efetivamente distribuído; a
licença alternativa aplicável não foi selecionada.

O `LICENSE-APACHE` de `ppv-lite86 0.2.21` foi lido integralmente no cache;
seu SHA-256 é `0218327e7a480793ffdd4eb792379a9709e5c135c7ba267f709d6f6d4d70af0a`.
Além dos termos Apache 2.0, o arquivo inclui apêndice com copyright 2019 The
CryptoCorrosion Contributors. A leitura não substitui a revisão dos avisos
dos arquivos incluídos no artefato final, nem seleciona uma das alternativas
de licença declaradas pela crate.

Os textos `LICENSE-APACHE` de `rand_core 0.9.5` e `zerocopy 0.8.63` também
foram lidos integralmente no cache; seus SHA-256 são, respectivamente,
`6df43f6f4b5d4587f3d8d71e45532c688fd168afa5fe89d571cb32fa09c4ef51` e
`9d185ac6703c4b0453974c0d85e9eee43e6941009296bb1f5eb0b54e2329e9f3`.
O primeiro contém os termos e instruções do apêndice sem aviso preenchido; o
segundo inclui no apêndice copyright 2023 The Fuchsia Authors. Esses arquivos
são alternativas declaradas pelos respectivos manifests, não uma escolha de
licença ou pacote de notices aprovado. Sua aplicabilidade depende dos arquivos
e artefatos efetivamente distribuídos.

Uma busca limitada a arquivos `.rs` nas cópias de `getrandom 0.3.4` e
`ppv-lite86 0.2.21`, pelos termos `copyright`, `SPDX-License-Identifier`,
`licensed under` ou `license:`, sem distinção de maiúsculas, não retornou
ocorrências. A mesma busca em `rand 0.9.5` e `zerocopy 0.8.63` retornou mais
resultados que o limite da consulta. Por exemplo, `rand/src/prelude.rs` credita
Developers of the Rand project e referencia Apache-2.0 ou MIT; `zerocopy/build.rs`
credita The Fuchsia Authors e identifica BSD-2-Clause, Apache-2.0 ou MIT. Não
foram examinados todos os resultados, outros tipos de arquivos ou avisos sem
esses termos. Esses fontes estão no cache de dependências, não foram identificados
como arquivos vendorizados na archive Rust e sua seleção para testes não prova
inclusão em distribuição.

### Dependências LTX selecionadas no candidato Rust

Na extração da archive `f8313da0…b495131`, `cargo tree --offline --locked -p
sqlite-replication-ltx -e normal,features --target x86_64-unknown-linux-gnu`
selecionou `lz4_flex 0.14.0` e `twox-hash 2.1.5` com `xxhash32`. Foram relidos
integralmente os textos MIT no cache local: `lz4_flex/LICENSE`, copyright 2020
Pascal Seitz, SHA-256 `0982c33390159842ecce8e9d6ce2e5e39961fe0e0ffbb2eec39c9ba46db6db10`;
e `twox-hash/LICENSE.txt`, copyright 2015 Jake Goulding, SHA-256
`f59bcb1c02654665f78a5f290bae76b6358137675f6459f51e6d474a4623d8ea`.
Ambos contêm condições de preservação do aviso e disclaimers completos; o
resumo não é substituto dos textos. Os hashes coincidem com a revisão anterior
`ltx-transitive-notices.md`, inclusive a questão ainda aberta do trecho da
especificação LZ4 reproduzido em comentário. A archive-fonte não vende essas
dependências; o grafo não prova quais objetos integram um binário futuro nem
resolve as obrigações de cada formato de distribuição.

A archive `lz4_flex 0.14.0` do cache tem SHA-256
`ecbdfe44b1bd960b68170b417450a628c43f7cf56bb3c5317e61cb230ee7f226`,
igual ao checksum do `Cargo.lock` do candidato `f8313da0…b495131`. Seu membro
`src/frame/header.rs`, SHA-256
`c9926623a51e20325e717df62bdf1ad0ca24b787e7e04e8122a38d8a28587cc4`, coincide
com a cópia do registry examinada em `ltx-transitive-notices.md`. Portanto a
investigação histórica ali registrada se refere aos mesmos bytes da dependência
fixada do candidato. O commit exato da especificação consultado pelo autor da
crate e as obrigações aplicáveis ao recorte continuam não determinados; essa
dependência não está vendorizada na archive-fonte de investigação.

### Adapter SQLite no candidato Rust

Na extração `f8313da0…b495131`, o grafo normal e de build para
`x86_64-unknown-linux-gnu` selecionou `rusqlite 0.41.1`, com `backup` e
`bundled`, e `libsqlite3-sys 0.39.1` com SQLite incorporado. Os textos `LICENSE`
dos dois wrappers foram relidos integralmente no cache: ambos identificam
copyright 2014 The rusqlite developers e têm SHA-256
`c10c1f27337546471e5f7e4e97fdd398b35b9d4e126115dcd22de8d8e65abf6f`.
O texto MIT requer preservar o aviso e a permissão conforme seus termos e
inclui disclaimer; este resumo não o substitui. A amalgamação local `sqlite3.c`
tem SHA-256 `0c657666d819f261cc4007526ad57e1669de9949df7a8ea847aa284a2bf0ee52`,
igual ao registrado em `sqlite-rust-transitive-notices.md`. O LICENSE do wrapper
não determina sozinho os termos da amalgamação ou de todos os demais arquivos.
Estas dependências não são vendorizadas na archive-fonte; a seleção por Cargo
não comprova conteúdo de um binário final. Continuam necessárias as revisões
por artefato dos textos SQLite, componentes nativos e dependências de build.

### S3/TLS no candidato Rust

Na extração `f8313da0…b495131`, `cargo tree --offline --locked -p
sqlite-replication-storage-s3 -i aws-lc-sys --target x86_64-unknown-linux-gnu`
confirmou o caminho selecionado via `aws-sdk-s3 1.157.0`,
`aws-smithy-runtime 1.16.2`, `aws-smithy-http-client 1.5.2`, `rustls 0.23.46`,
`aws-lc-rs 1.19.0` e `aws-lc-sys 0.47.0`. A archive da última crate no cache
tem SHA-256 `8bfbe32ac36b3c428696ce315464181bc4029d689fd58e3f2a4e58c252242f52`,
igual ao checksum do lockfile extraído e ao registrado em
`s3-aws-lc-transitive-notices.md`. A revisão anterior descreve o LICENSE
composto e textos upstream para imports com licenças ausentes da crate; suas
pendências de aplicabilidade e composição de binários permanecem abertas.
A archive-fonte do candidato não inclui a archive da dependência; este grafo e
checksum não constituem link map nem aprovação de notices ou distribuição.

No `fresh-target` criado para a extração e testes de `f8313da0…b495131`, a
archive intermediária `debug/build/aws-lc-sys-89ce0ebf005deffc/out/
libaws_lc_0_47_0_crypto.a` tem SHA-256
`9b51d35e7f0a590bd9d19193879c9cd69e102f29bb1329784f0270400c3952f3`,
idêntico ao intermediário examinado em `s3-aws-lc-transitive-notices.md`.
`ar t` confirmou membros com nomes `jitterentropy`, `mlkem`, `mldsa` e
`keccak` nessa archive. Assim, a revisão amostral de cabeçalhos e os textos
upstream ali registrados são evidência relevante para esse intermediário exato;
isso não prova que tais objetos sejam vinculados a um executável específico,
nem substitui revisão de outros componentes ou do artefato a distribuir.

## Cópia Go local para teste

Uma cópia de investigação do checkout modificado foi criada sem sobrescrever
arquivos em `~/git-worktrees/sqlite-replication/go-source-candidate-validation-20261106/`.
Inclui `go/` e `conformance/fixtures/`, inclusive arquivos não rastreados não
ignorados, e não define um pacote de distribuição. Seus 127 arquivos foram
verificados contra `SHA256SUMS`, cujo SHA-256 é
`f4fa5779112adb7d5e6e5541cb932629f37028f8c9e413ecbe1fd09bab621d13`.
Na cópia, com Go 1.27.3 e dependências do cache local, `GOPROXY=off go test
-race -count=1 ./...` passou nos pacotes do módulo principal. Workspaces Go
separados, Floci e interoperabilidade opt-in não foram executados nessa cópia;
não se demonstrou instalação offline em outro host nem aprovação de notices.

O workspace Go separado `go/examples/coordinator-telemetry`, incluído na mesma
cópia, também passou `GOPROXY=off go test -race -count=1 ./...` com dependências
do cache local. Essa execução não incorpora nem aprova notices de terceiros.

Inspeção dos nomes registrados no manifesto dessa cópia encontrou 77 arquivos
sob `go/` e 50 sob `conformance/fixtures/`, nenhum com basename iniciado por
`LICENSE`, `NOTICE` ou `COPYING` (comparação sem distinção de maiúsculas).
A cópia não inclui esta documentação de SBOM, nem um pacote de notices de
terceiros. A busca por nomes não verifica avisos embutidos nem estabelece quais
textos ou atribuições seriam exigidos em eventual distribuição. Seu manifesto
e testes não equivalem a uma aprovação para distribuição.

Uma busca textual sem distinção de maiúsculas nos arquivos `.go` dessa cópia,
incluindo o exemplo de telemetria, por `copyright`, `SPDX-License-Identifier`
ou `licensed under` não encontrou ocorrências. A busca não identifica código
copiado sem esses marcadores, outras formas de aviso ou direitos de terceiros;
não substitui a revisão dos textos aplicáveis às dependências selecionadas.
