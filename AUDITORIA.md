# Auditoria Completa: Framework Rullst

Este documento contém o relatório de auditoria completa da biblioteca/framework `Rullst`, focado em Segurança, Atualizações de Dependências, Performance, Bugs, UX/DX e Facilidade de Manutenção.

---

## 1. Segurança (Prioridade Alta)

A análise focou nos vetores clássicos de ataque em frameworks web (XSS, CSRF, Injeção, Traversal, etc.).

### 1.1 Proteção contra CSRF
- **O que foi feito bem**: O `rullst::security::csrf_middleware` implementa o padrão *Double Submit Cookie* e bloqueia corretamente requisições mutáveis (POST/PUT/DELETE) caso o token seja inválido.
- **Ponto de Atenção Crítico (Corpo de Requisição)**: Para extrair o token do corpo (`extract_token_from_body`), o framework consome e aloca até 1MB do corpo na memória (`axum::body::to_bytes(body, 1024 * 1024)`). Se um atacante enviar milhões de requisições POST com 1MB cada, o servidor rapidamente sofrerá de **Memory Exhaustion (DoS)**. Recomenda-se implementar limites globais (via `tower_http::limit::RequestBodyLimitLayer`) e evitar fazer o buffer inteiro da requisição na memória a menos que estritamente necessário.
- **Ponto de Atenção Crítico (Verificação Lax)**: Ao gerar um cookie de CSRF em um simples método GET, a implementação atual o marca como `SameSite=Lax`. Em arquiteturas modernas de API, o ideal seria que a geração do token CSRF ficasse restrita a um endpoint específico, e/ou que usasse `SameSite=Strict` para maior segurança.

### 1.2 Proteção contra XSS
- **O que foi feito bem**: A macro `html!` chama `HtmlEscape::escape_html()` para variáveis dinâmicas injetadas na template. Além disso, o `rullst::security::headers_middleware` injeta o cabeçalho `X-XSS-Protection`.
- **Atenção**: O tipo `RawHtml` permite injeção arbitrária, o que é esperado do design. O desenvolvedor precisa ser muito bem documentado/avisado para não passar dados de usuários não purificados no `RawHtml(user_input)`.

### 1.3 Path Traversal e Storage
- **O que foi feito bem**: O `LocalDriver::resolve_path` em `storage.rs` normaliza o caminho e previne de fato ataques básicos de Path Traversal bloqueando a leitura caso a raiz resolvida não inicie com a variável de ambiente `STORAGE_ROOT`.
- **Atenção**: A construção de diretórios e o parse na stdlib já filtram componentes relativos `..`, mas manter esse driver sempre bem auditado em futuras PRs é vital.

### 1.4 Multitenancy Parameter Leak (Vazamento de Informações)
- O `TenantStrategy::Parameter` permite o id do lojista (tenant_id) na query string. Conforme o próprio comentário do autor documenta, isso é um vazamento crasso em logs e `Referer`. É recomendado um log em *warning* ou documentar que *Parameter* não deve ser usado em produção.

### 1.5 Dylib Plugin Loader e Execução de Código
- Em `server.rs`, `load_dylib_router` usa o `libloading` via `unsafe` para carregar routers alocados num plugin.
- **Risco de Segurança**: Carregar código dinâmico local exige confiança extrema no sistema de arquivos local do host. Em ambientes containerizados com permissões frouxas, um atacante com *File Write Access* pode injetar sua própria `.so` e tomar o controle de todo o processo Rust via uma `rullst_router_init` maliciosa (RCE).
- **Mitigação**: Os arquivos carregáveis devem verificar assinaturas criptográficas ou o diretório alvo da extensão deve ser `chmod 400` pelo sistema operacional.

---

## 2. Atualização e Dependências

A simulação de atualização revelou que o projeto usa um lockfile mas que algumas bibliotecas core estão recebendo releases constantes (embora mantidas em compatibilidade semver pela flag `--dry-run`).

- **Dependências de Infraestrutura AWS**: `aws-sdk-s3`, `aws-config`, `aws-sdk-sso`, etc. possuem pequenas bumps de versão constantes. Recomenda-se um Dependabot.
- **Dependências Críticas**: O framework prende (pin) versões de `argon2` e `aes-gcm` nas branches `Release Candidate (rc)`. Ex: `"0.11.0-rc.4"`, `"0.6.0-rc.8"`.
  - **Ação**: Isso é um risco de quebra e instabilidade. Assim que a versão final dessas bibliotecas de criptografia for lançada, o framework deve transacionar para elas.

---

## 3. Performance e Concorrência

- **Construtores de Query / Strings em Laços**: A documentação já instrui o uso de `String::with_capacity` e `write!` para HTML. A macro `rullst-macros` compila o HTML de maneira bastante performante (no-runtime parsing).
- **Zstd / Brotli (cargo-rullst/src/main.rs)**: Há um script no cargo-rullst que comprime ativos estáticos no build para Brotli e ZSTD. Ótimo padrão de performance (Pré-compressão).
- **Replication Manager Sleep Tick**: No `db.rs`, a versão WASM usa uma trava por *spinlock* (`while ticks < config.sync_interval_secs ...`). Isso gasta ciclos inúteis do Event Loop no Web Worker. O ideal seria usar o `setTimeout` exportado pelo `web_sys` via Promessas (Promise-based sleep) ao invés do `performance.now()`.

---

## 4. Bugs e Tratamento de Erros

- No `server.rs`, o catch de panic é tratado, mas a transformação do Downcast do `JoinError` assume que pânicos são strings puras (`&str` ou `String`). Objetos arbitrários em *panic!* gerarão a mensagem genérica.
- **Mail Driver Resend**: Em `mail.rs`, caso a API da Resend retorne um erro HTML (como erro de Gateway/Cloudflare do lado deles), a conversão de `res.text().await` trará uma página HTML massiva no payload do log de erro da aplicação, sujando imensamente os logs locais do container de produção.

---

## 5. Experiência do Desenvolvedor (DX) e UX

- **DX Extremamente Alto**: A inclusão do *Artisan* embutido (inspirado no Laravel), suporte forte à HTMX no `htmx.rs`, macros baseados em JSX, suporte multi-tenant transparente via Traits de camada, e Wasm-Islands marcam uma usabilidade formidável para desenvolvedores.
- **Consoles interativos**: O Rullst Studio e as tratativas do *Error Console HTML* são grandes pontos extras.
- **Documentação de Testes**: A adoção de `mod tests` in-file facilita absurdamente a busca de comportamento interno pelos contribuidores.
- **Falta de Tipagem Estrita em Rotas HTMX**: `HtmxResponse` permite disparar e redirecionar strings de maneira arbitrária (`into()`). A longo prazo, desenvolvedores poderiam achar melhor passar construtores de URL tipados.

---

## 6. Facilidade de Manutenção (Maintainability)

- O projeto usa uma estrutura baseada em *Features* no `Cargo.toml`. Essa é uma técnica arquitetural de alto nível para não inflar as dependências com `redis`, `s3`, e `lettre` se o usuário quer apenas a API base.
- **Modularização**: Cada componente do framework tem seu arquivo restrito (ex. `queue.rs`, `storage.rs`, `db.rs`, `mail.rs`). O design pattern *Facade* está presente em todo lado (ex. `Storage::put()`, `Queue::sqlite()`).
- O suporte a `Box<dyn Driver>` permite TDD fluído e fácil substituição de comportamentos, fazendo com que a manutenção e os mocks no código do usuário final seja livre de dores de cabeça.

---

## Conclusão e Próximos Passos
O framework `rullst` se apresenta extremamente moderno, absorvendo boas práticas de ponta de Rust e ecossistemas Fullstack. A principal ação recomendada é rever os pontos de gargalo de alocação de memória nos middlewares de Segurança (o buffer do corpo na verificação do CSRF) e atualizar dependências de criptografia de `rc` para versões estáveis quando lançadas.