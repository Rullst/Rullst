# Avaliação técnica completa do framework Rullst

**Data da auditoria:** 24 de agosto de 2026  
**Commit avaliado:** `96222fbd31bec3d20bc50db68c41bb85ca595779`  
**Fonte normativa usada:** `docs/src/spec.md`  
**Tipo de avaliação:** arquitetura, aderência à especificação, segurança, confiabilidade, qualidade de API, testes, CI/release e maturidade por crate.

## 1. Resposta direta

O Rullst tem uma **boa visão arquitetural e uma base tecnicamente interessante**, mas a implementação do framework inteiro ainda não sustenta várias das promessas de produção feitas pela documentação. A separação nominal em crates, a API de routing, o `html!` com escape, partes do ORM e o volume de testes mostram trabalho real e uma direção coerente. Por outro lado, há subsistemas críticos que são incompletos, simulados ou inseguros por padrão.

Minha conclusão é:

- **A arquitetura conceitual é boa**, especialmente na divisão por domínios e na preferência por APIs explícitas e compile-time.
- **A arquitetura efetiva é apenas mediana**, porque `rullst-core` depende obrigatoriamente do ORM, há duas implementações concorrentes de segurança e o metapacote não representa todos os subsistemas anunciados.
- **Existe bastante coisa a melhorar**, e as prioridades não são apenas refatorações cosméticas: há bloqueadores fiscais, criptográficos, administrativos, de scaffolding e de release.
- **Existem bugs concretos de severidade alta e crítica.** Os mais sérios podem produzir autorização fiscal falsa, perda irreversível de dados, acesso administrativo aberto, CORS inseguro em projetos gerados, travessia de diretório no storage e aceitação de firmware com assinatura falsa.
- **Não considero o conjunto completo pronto para produção** enquanto os itens P0 deste documento não forem corrigidos ou explicitamente removidos/rotulados como experimentais.
- Também não considero justo chamar todo o repositório de “ruim” ou “cheio de bugs”: a maturidade varia muito. Routing/HTML e vários fundamentos estão em situação bem melhor que Fiscal, IoT, Nexus, partes de Security, Studio e alguns geradores do CLI.

Em uma frase: **o Rullst parece hoje uma fundação promissora misturada com funcionalidades enterprise ainda em estágio de protótipo, mas apresentadas como concluídas**.

## 2. Nota geral

As notas abaixo são uma avaliação técnica heurística, não uma medida matemática de cobertura ou segurança.

| Dimensão | Nota | Leitura |
|---|---:|---|
| Visão e separação nominal em crates | **7,0/10** | Os domínios estão bem identificados e a organização física é legível. |
| Arquitetura efetiva de dependências | **5,5/10** | Core acoplado ao ORM, segurança duplicada e umbrella incompleto reduzem a modularidade real. |
| Aderência à SST (`spec.md`) | **5,5/10** | Estimativa de 55–60%: alguns fundamentos aderem bem, mas Connect, IoT, Fiscal, AI e compatibilidade divergem bastante. |
| Segurança e defaults seguros | **3,5/10** | Existem mecanismos bons, mas também defaults abertos, criptografia simulada e decisões fail-open. |
| Confiabilidade operacional | **4,0/10** | Há panics, estados globais parciais, erros ignorados e APIs que retornam sucesso sem realizar a operação. |
| Engenharia de testes, pela estrutura estática | **6,5/10** | O investimento aparente é alto; a execução local ficou inconclusiva por ausência do toolchain. |
| CI e release | **3,5/10** | A matriz principal é útil, mas vários gates não bloqueiam e o release está em ordem topológica incorreta. |
| Fidelidade da documentação ao código | **3,0/10** | README, auditorias e compliance fazem alegações que o código atual contradiz. |
| Prontidão enterprise do conjunto | **4,0/10** | Partes utilizáveis coexistem com bloqueadores de produto e segurança. |
| **Avaliação global atual** | **5,0/10** | Boa fundação, maturidade muito desigual e dívida de honestidade contratual. |

## 3. Escopo, método e limitações

Esta avaliação incluiu:

- leitura integral da SST em `docs/src/spec.md`;
- inventário do workspace, manifests, features, dependências e exports;
- inspeção de todos os crates e aprofundamento nos caminhos de maior risco;
- buscas sistemáticas por `panic!`, `unwrap`, `expect`, `unsafe`, stubs, mocks, SQL dinâmico, autenticação, criptografia, telemetria e fallbacks;
- revisão dos workflows de CI, release, fuzz, Miri, Kani, unsafe, WASM, auditoria e supply chain;
- comparação entre código, README, `AUDIT.md`, `SECURITY_COMPLIANCE.md`, `CHANGELOG.md` e a SST;
- verificação do estado do Git antes da criação deste relatório.

Inventário estático observado:

| Item | Quantidade |
|---|---:|
| Membros no workspace | 16: 15 crates e o exemplo `examples/blog` |
| Arquivos no repositório | 755 |
| Arquivos Rust | 549 |
| Declarações de teste Rust encontradas | 927, distribuídas em 224 arquivos |
| Targets de fuzz | 40 |
| Benchmarks | 8 |
| Provas Kani | 21 |
| Blocos `proptest!` | 2 |
| Workflows GitHub Actions | 32 |

Essas quantidades são **contagens estáticas**. Elas não equivalem a testes executados, cobertura ou ausência de bugs.

### Limitação importante de execução

O ambiente desta auditoria não possui `cargo`, `rustc` ou `rustup` no `PATH` nem nos locais usuais. Portanto:

| Comando obrigatório | Resultado local |
|---|---|
| `cargo test --workspace --all-features` | Não executado: `cargo` não encontrado |
| `cargo clippy --workspace --all-features -- -D warnings` | Não executado: `cargo` não encontrado |
| `cargo fmt --all -- --check` | Não executado: `cargo` não encontrado |

Isso significa que a trifeta ficou **inconclusiva por limitação do ambiente**, e não que os testes passaram ou falharam. Nenhuma conclusão deste relatório usa `AUDIT.md` ou badges como substituto para uma execução reproduzível.

Também não foram realizados exploração ativa, chamadas reais a provedores, transmissão fiscal, publicação no crates.io ou teste de hardware. Os achados de código são marcados como confirmados quando o comportamento decorre diretamente da implementação; riscos dependentes de como a API é montada são indicados como condicionais.

## 4. Mapa de maturidade por crate

| Crate | Estado observado | Avaliação resumida |
|---|---|---|
| `rullst` | Parcial | Umbrella simples e legível, mas sem features/reexports de Security e IoT; a feature OAuth também está mal conectada. |
| `rullst-core` | Substancial, com riscos | Routing, server, HTML, cache, filas e telemetria têm volume real; há acoplamento ao ORM, defaults fail-open, métricas simuladas, panics e drivers que fingem sucesso. |
| `rullst-macros` | Boa base, uma API imatura | `html!` possui escape real. A macro pública `#[route]` ainda é apenas fundacional e pode surpreender usuários. |
| `rullst-orm` | Substancial | CRUD/macros/binds e privacidade AES-GCM são reais; globais com `OnceLock`, panics e inicialização parcial de réplicas prejudicam robustez. |
| `rullst-orm-macros` | Substancial | Bem decomposto internamente, mas injeta panic em lazy loading e nem todas as assinaturas coincidem com a SST. |
| `rullst-auth` | Parcial/substancial | Argon2 e sessão AES-GCM são reais; não há JWT de aplicação no crate, a política de rehash é incompleta e WebAuthn omite verificações importantes. |
| `rullst-security` | Parcial e inconsistente | Tem vários módulos úteis, porém `FieldEncryptor` não criptografa, o janitor de login não limpa o estado real e há problemas em DLP, auditoria, rate limit e telemetria. |
| `rullst-ai` | Parcial | OpenAI, Gemini, Anthropic e Ollama existem; faltam DeepSeek e integração obrigatória dos guardrails/PII, e alguns caminhos mock ainda fazem rede. |
| `rullst-capital` | Amplo, com bloqueador crítico | Há vários provedores e bons fallbacks em partes do módulo. A assinatura e a resposta NFS-e não são válidas para produção. |
| `rullst-connect` | Maduro como OAuth, não aderente ao papel declarado | Implementa muitos provedores OAuth/OIDC, PKCE e testes. Não implementa RabbitMQ, Redis Streams, Kafka, WebSockets ou SSE como exige a SST. |
| `rullst-iot` | Protótipo/stub | MQTT é explicitamente stub; OTA, HSM e PQC simulam criptografia. Não deve ser exposto como implementação industrial ou zero-trust. |
| `rullst-mail` | Parcial/substancial | Drivers reais, fila e validações existem; fallback mock não é uniforme e o facade multi-tenant ignora o tenant em um caminho. |
| `rullst-studio` | Parcial | Usa coletores reais em partes, mas exibe métricas/estados hardcoded, tem composição de rota quebrada e risco de XSS no polling. |
| `rullst-nexus` | Funcional, inseguro por padrão | CRUD dinâmico existe e sanitiza identificadores, porém monta operações administrativas sem autenticação/RBAC por padrão. |
| `cargo-rullst` | Amplo, mas com regressões | Tem muitos geradores e blueprints; bugs de flags, nomes, paths, IDs, CORS e credenciais padrão tornam alguns scaffolds perigosos ou inválidos. |

## 5. O que está bom

É importante preservar o que já funciona bem, em vez de reescrever tudo.

### 5.1. Organização física e intenção arquitetural

A divisão em crates expressa domínios reconhecíveis e facilita localizar código. Os manifests centralizam versão, edition e MSRV. `rullst-core`, macros, ORM, serviços, dashboards e CLI têm fronteiras físicas razoáveis. Mesmo quando as dependências precisam ser corrigidas, a taxonomia de produto é uma boa base.

### 5.2. Routing e API HTTP

`routes!`, `Router` e `Server` formam uma API pequena e compreensível (`rullst-core/src/routing.rs:168-210`; `rullst-core/src/server/builder.rs:38-50`). A preferência por Axum/Tower e tipos explícitos reduz “mágica” em runtime e está alinhada ao objetivo AI-native.

### 5.3. Escape de HTML

O `html!` não é apenas uma alegação. Há escape de nós de texto e atributos em `rullst-macros/src/html_parser.rs:124-135,169-187`, com escape central e `RawHtml` explícito em `rullst-core/src/html.rs:9-23,68-113`. Essa é uma fronteira de segurança bem desenhada: o caminho seguro é o padrão e o bypass exige intenção explícita.

### 5.4. Fundamentos do ORM

As macros de Active Record estão separadas em operações de query, CRUD e relacionamentos. Em geral, valores dinâmicos passam por bind SQLx e o Nexus sanitiza identificadores antes de interpolar nomes de tabelas/colunas. `rullst-orm/src/privacy.rs:73-117` contém AES-256-GCM real, com nonce aleatório e erros tipados; isso contrasta positivamente com o falso `FieldEncryptor` do crate Security.

### 5.5. Amplitude de testes e ferramentas

O repositório investe em testes unitários e de integração, fuzz, benchmarks, Miri, Kani, testcontainers, MSRV e matriz multi-OS. A CI principal executa testes em Linux, macOS e Windows e há serviços reais de banco na matriz. A estrutura é muito melhor que a de um protótipo sem testes, embora a efetividade de vários gates precise ser corrigida.

### 5.6. Alguns controles de segurança são bons

Há comparação constante de tokens em pontos importantes, HMAC em diversos webhooks, limite de corpo em CSRF/webhook/DLP, `HttpOnly`/`SameSite` nas sessões, Argon2 executado em helpers assíncronos, zeroization de segredos e sanitização de identificadores. O problema não é ausência total de segurança; é a inconsistência entre bons controles e outros caminhos inseguros.

### 5.7. Implementações externas com substância

`rullst-connect` possui um conjunto amplo de provedores OAuth/OIDC e testes. Capital possui muitos provedores e vários fallbacks offline. Mail tem Resend, SendGrid, Postmark, SMTP e fila. Studio já consulta `RadarSnapshot` e `SpanCollector` em algumas páginas. Esses módulos devem ser refinados, não descartados.

## 6. Avaliação da arquitetura

### 6.1. O desenho pretendido

O desenho da SST é coerente: um kernel HTTP mínimo, serviços de domínio independentes, macros para ergonomia, um umbrella opcional e ferramentas de desenvolvimento separadas. Isso favoreceria builds menores, teste isolado, evolução semver e escolha granular de capacidades.

### 6.2. O grafo real tem acoplamento excessivo

O principal desvio é `rullst-core` depender obrigatoriamente de `rullst-orm` (`rullst-core/Cargo.toml:42`) e reexportá-lo (`rullst-core/src/lib.rs:121-123`). Assim, o suposto kernel HTTP mínimo já arrasta o ORM/SQLx. A feature `orm` opcional do umbrella (`rullst/Cargo.toml:17-18`) não remove esse custo porque `rullst-core` continua dependendo do ORM.

O objetivo deveria ser algo próximo a:

```text
rullst-core  <-  rullst-orm / auth / mail / ai / capital / connect / security
      ^
      |
   rullst (umbrella com features opcionais)
```

Hoje, simplificadamente, existe:

```text
rullst -> rullst-core -> rullst-orm
                  \-> segurança básica própria
rullst-security ------> outra pilha de segurança, não usada pelo Server padrão
```

Essa direção aumenta tempo de compilação, bloat transitivo e dificuldade de usar somente o kernel HTTP.

### 6.3. Existem duas fontes de verdade de segurança

Há `rullst_core::security` e o crate `rullst-security`. O `Server` padrão aplica WAF, CSRF e headers do Core (`rullst-core/src/server/builder.rs:356-368`), não a pilha mais rica do crate dedicado. Isso causa três problemas:

1. o usuário não sabe qual implementação representa a política oficial;
2. telemetria do crate Security pode não refletir a proteção efetivamente instalada;
3. correções podem ser feitas em um lado e não chegar ao outro.

Recomendação: definir interfaces/camadas canônicas em `rullst-security`, deixar no Core apenas contratos ou um conjunto mínimo explicitamente nomeado e montar uma policy stack única pelo `Server`.

### 6.4. O umbrella não representa o produto anunciado

`rullst/Cargo.toml:16-33` não oferece features/dependências para `rullst-security` nem `rullst-iot`, e `rullst/src/lib.rs:1-30` não os reexporta. Connect só aparece como feature `oauth`. A feature `mailer` ativa `lettre` no umbrella, mas não encaminha de forma clara `rullst-mail/mail-smtp`; a feature `oauth` ativa a dependência Connect sem um reexport correspondente no crate raiz.

O umbrella deveria ter uma matriz de features testada, documentada e simétrica: toda feature ativa exatamente a implementação e o reexport esperados.

### 6.5. Contrato e implementação divergiram

O problema mais amplo é que a SST funciona ao mesmo tempo como arquitetura, roadmap e documentação de funcionalidade pronta. Exemplos:

- Connect é um ótimo candidato a crate OAuth, mas a SST o define como mensageria/streaming.
- IoT declara explicitamente stubs, enquanto README/SST o chamam de cliente MQTT 5 e edge industrial.
- AI usa `AiProvider/AiClient` e dispatch dinâmico, enquanto a SST fala em `LlmClient` e guardrails automáticos.
- Capital funde contratos em `BillingProvider`, em vez dos traits separados descritos.
- O ORM tem APIs semelhantes às documentadas, mas algumas assinaturas públicas diferem.

É preciso tomar uma decisão de produto: **ou a SST é contrato da versão atual, e o código deve alcançá-la antes da release; ou é roadmap, e tudo que não existe deve ser marcado explicitamente como planejado/experimental**.

### 6.6. Compatibilidade futura está inconsistente

O Core usa `#[non_exhaustive]` em várias configurações, mas muitos enums/configs públicos nos demais crates não usam. Exemplos: `SecureHeadersConfig`, `TimingGuardConfig`, `FieldKind`, `SubscriptionStatus` e `PayoutStatus`. A busca também não encontrou uma política real de `#[deprecated]` aplicada às APIs. Isso contraria `docs/src/spec.md:183-190` e torna novas variantes/campos potencialmente breaking changes.

### 6.7. Static dispatch é uma preferência não concretizada

AI usa `Arc<dyn AiProvider>`, Capital mantém providers globais em `Box<dyn ...>`, Connect retorna `Box<dyn Provider>` e Mail guarda transportes dinâmicos. Dispatch dinâmico pode ser legítimo em registries runtime, mas a SST e o AGENTS declaram static dispatch como preferência. A arquitetura deve esclarecer onde dinamismo é necessário e oferecer caminhos genéricos para casos estáticos, em vez de prometer uma propriedade que a superfície pública não cumpre.
