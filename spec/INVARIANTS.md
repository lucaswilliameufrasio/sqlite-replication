# Invariantes do motor

1. **Commit válido:** nunca gerar LTX de transação WAL parcial ou checksum
   inválido.
2. **Captura antes do descarte:** não reiniciar/truncar WAL antes de tornar
   duráveis os commits elegíveis; CP-01 é gate obrigatório.
3. **Cadeia recuperável:** não afirmar recuperação com lacuna TXID, checksum
   inconsistente, geração divergente ou base ausente.
4. **Imutabilidade:** objeto confirmado nunca é sobrescrito; retry exige bytes e
   checksums iguais.
5. **Restore isolado:** reconstrução sempre usa staging inacessível à aplicação;
   promoção é create-only após validação SQLite e sync.
6. **Proveniência:** versão do formato, identidade de banco/geração, página e
   posição são explícitos e validados.
7. **Falha explícita:** corrupção, checkpoint inseguro, upload ignorado ou
   truncamento de histórico nunca são silenciosos.
8. **Watermarks distintos:** commit local, LTX local durável e último TXID
   remoto confirmado não são intercambiáveis.
9. **Fonte SQLite válida:** não copiar DB vivo por `cp`; `-shm` não é backup
   portátil; snapshot usa Online Backup API homologada.
10. **Sem multiwriter:** o produto é DR com um escritor lógico. `If-None-Match`
    protege chave, não fornece lease distribuído ou fencing.
11. **Banco único:** `ATTACH` e transações entre bases não são atomicamente
    replicáveis por este contrato.
12. **Sem promessa acima da evidência:** Floci não prova compatibilidade de todo
    provedor S3; snapshot não é streaming; testes probabilísticos não aprovam
    CP-01.
