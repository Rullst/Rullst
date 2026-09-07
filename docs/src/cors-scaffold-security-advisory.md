# Aviso de segurança — CORS em scaffolds antigos

## Estado

- **Componente afetado:** aplicações geradas por versões antigas de
  `cargo-rullst make:cors` ou por blueprints que copiavam uma política CORS
  permissiva/refletida.
- **Framework atual:** corrigido. Novos scaffolds usam uma allowlist explícita e
  falham quando `CORS_ALLOWED_ORIGINS` está ausente, vazia, contém `*` ou uma
  origem inválida.
- **Aplicações já geradas:** não são modificadas automaticamente por uma
  atualização do CLI. Cada repositório precisa revisar o middleware que já foi
  copiado para seu código-fonte.
- **Severidade:** depende da aplicação. O risco é maior quando respostas
  autenticadas por cookie ou bearer token podem ser lidas por uma origem não
  confiável.

Este é um aviso de migração do scaffold, não um CVE, pentest ou afirmação de que
toda configuração final de CORS é segura.

## Como identificar uma aplicação potencialmente afetada

Revise o middleware CORS gerado e procure qualquer um destes padrões. O trecho
abaixo é deliberadamente incompleto e inseguro: serve somente como padrão de
busca, não como código para copiar ou executar.

```rust,ignore
AllowOrigin::mirror_request()
.allow_origin(Any)
.allow_credentials(true)
```

Também é inseguro copiar o valor do header `Origin` para
`Access-Control-Allow-Origin` sem compará-lo com uma allowlist administrada pelo
servidor. A combinação de origem refletida ou wildcard com credenciais merece
correção imediata.

Uma busca inicial pode ser feita na raiz da aplicação:

```bash
rg -n 'mirror_request|allow_origin\(Any\)|allow_credentials\(true\)|Access-Control-Allow-Origin' src
```

O resultado exige revisão humana: a presença de `allow_credentials(true)` não é
por si só uma vulnerabilidade quando a lista de origens é fechada e correta.

## Migração recomendada

1. Faça uma cópia/revisão do middleware existente e gere a versão atual em uma
   branch separada com `cargo rullst make:cors`.
2. Substitua reflexão/wildcard por uma lista exata de origens com esquema, host e
   porta, por exemplo:

   ```text
   CORS_ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com
   ```

3. Mantenha credenciais desabilitadas, salvo quando a aplicação realmente usa
   cookies ou autenticação cross-origin. Se forem necessárias, habilite-as
   somente depois de validar a allowlist exata.
4. Restrinja métodos e headers ao contrato real da API. O scaffold atual inclui
   um baseline, não conhecimento automático de todos os endpoints da aplicação.
5. Teste pelo menos uma origem permitida e origens negativas com host parecido,
   subdomínio não autorizado, porta diferente, `null` e ausência de `Origin`.

Exemplo de verificação manual (uma origem não autorizada não deve receber
`Access-Control-Allow-Origin`):

```bash
curl -i -X OPTIONS https://api.example.com/resource \
  -H 'Origin: https://evil.example' \
  -H 'Access-Control-Request-Method: POST'
```

## Critério de conclusão

A migração está concluída quando:

- nenhuma origem é refletida ou aceita por wildcard;
- o processo falha de forma explícita quando a configuração exigida está
  ausente ou inválida;
- credenciais só são aceitas para origens exatas autorizadas;
- testes negativos confirmam que origens enganosamente parecidas não recebem
  autorização CORS;
- caches/proxies recebem `Vary: Origin` quando a resposta depende da origem.

O template vigente pode ser consultado em
[`cargo-rullst/src/generators/cors_middleware.rs.template`](../../cargo-rullst/src/generators/cors_middleware.rs.template).
