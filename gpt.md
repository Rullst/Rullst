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
| Arquivos rastreados pelo Git | 803 |
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
| `cargo test --workspace --all-features` | Invocado, mas bloqueado antes de compilar/executar: `cargo` não encontrado, exit 1 |
| `cargo clippy --workspace --all-features -- -D warnings` | Invocado, mas bloqueado antes de analisar: `cargo` não encontrado, exit 1 |
| `cargo fmt --all -- --check` | Invocado, mas bloqueado antes de formatar/verificar: `cargo` não encontrado, exit 1 |

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

## 7. Achados críticos — P0

Nesta seção, “crítico” significa que o item pode causar violação de segurança, integridade fiscal/financeira, perda de dados ou quebra do processo de release no uso anunciado. Alguns dependem da aplicação montar a API com entrada externa; a falha da implementação, porém, é confirmada diretamente no código.

### P0-01 — A implementação de NFS-e não produz uma assinatura XMLDSig válida

**Evidência:** `rullst-capital/src/fiscal/signer.rs:15-60`; `docs/src/spec.md:251-254`.

O signer:

- localiza `<infDPS>` por substring, sem canonicalização XML C14N;
- calcula o digest do texto literal;
- calcula `SHA-256(SignedInfo)` e usa esse hash como `SignatureValue`;
- não carrega nem utiliza a chave privada do PKCS#12;
- não executa RSA-SHA256;
- injeta o PFX base64 inteiro no campo `X509Certificate`.

Isso gera um XML com aparência de XMLDSig, mas sem assinatura criptográfica autenticável. A tendência é rejeição pela SEFIN; pior, código chamador pode acreditar que a etapa criptográfica foi concluída.

O cliente amplia o risco:

- `rullst-capital/src/fiscal/client.rs:37-50` cria um `reqwest::Client` sem configurar identidade/certificado mTLS;
- `:60-75` retorna uma resposta mock “Autorizada” quando a senha é `mock` **ou o PFX está vazio**, mesmo se o ambiente selecionado for Production;
- `:112-140` transforma qualquer corpo 2xx não JSON em “Autorizada”, com chave e protocolo inventados;
- um JSON vazio `{}` também recebe defaults de autorização.

**Impacto:** uma rejeição XML, HTML de proxy, resposta inesperada ou configuração incompleta pode ser registrada como nota autorizada. Isso é um risco fiscal e contábil real.

**Correção necessária:** bloquear o modo Production até existir PKCS#12 real, extração segura do certificado/chave, C14N, digest das referências, RSA-SHA256, mTLS e parser estrito dos estados oficiais. Mock deve ser um tipo/ambiente explícito e nunca retornar um objeto indistinguível de uma autorização real.

### P0-02 — `FieldEncryptor` destrói dados e não implementa criptografia

**Evidência:** `rullst-security/src/vault.rs:42-68,83-89`.

A documentação da API diz AES-256-GCM/ChaCha20-Poly1305. Na realidade:

```text
encrypt(plaintext, key) = "ENC:v1:" + SHA256(plaintext || key)
decrypt(ciphertext, qualquer_chave) = "[DECRYPTED:<hash>]"
```

O texto original é irrecuperável e a chave é ignorada na “decriptação”. O teste só verifica se a string contém `DECRYPTED`, então ele consolida o comportamento falso em vez de provar round-trip.

**Impacto:** qualquer aplicação que migre uma coluna por essa API perde o conteúdo original. Além disso, a garantia de confidencialidade anunciada é inexistente.

**Correção necessária:** remover/deprecar imediatamente a API atual ou fazê-la retornar erro explícito. Reutilizar a implementação AES-GCM real de `rullst-orm/src/privacy.rs:73-117`, com envelope versionado, nonce aleatório, AAD, rotação de chave e teste obrigatório `decrypt(encrypt(x)) == x`.

### P0-03 — As garantias criptográficas de IoT são stubs expostos como APIs

**Evidência:**

- `rullst-iot/src/ota.rs:50-54,84-87`: qualquer payload não vazio com qualquer assinatura de 64 bytes é aceito; o teste aprova `[0u8; 64]`;
- `rullst-iot/src/hsm.rs:37-50`: a “chave HSM” é SHA-256 do serial e do nome público do chip; a “assinatura” é outro SHA-256;
- `rullst-iot/src/pqc.rs:3-48`: ML-KEM/Kyber é explicitamente um stub de hashes; encapsular e decapsular nem sequer são operações inversas de um KEM;
- `rullst-iot/src/mqtt.rs:1`: arquivo de uma linha, declarado como stub;
- `rullst-iot/src/lib.rs:86-93`: `MqttDriver` apenas formata um valor como string.

**Impacto:** se `verify_signature` for usado como gate de OTA, firmware arbitrário é aceito. HSM/PQC não fornecem autenticidade, sigilo ou resistência pós-quântica. Isso é especialmente grave porque README e workflows anunciam MQTT industrial, HSM e compliance ML-KEM (`README.md:66,70,382`).

**Correção necessária:** colocar o crate inteiro atrás de uma feature `experimental`, renomear tipos para `Simulated...` e impedir uso de stubs no build de produção. OTA deve usar Ed25519 real, chave pública confiável, manifesto assinado, hash do firmware, anti-rollback e estado que só permita `commit` após verificação. HSM precisa de backends reais; PQC deve usar uma implementação ML-KEM auditada.

### P0-04 — Nexus é um painel administrativo destrutivo aberto por padrão

**Evidência:**

- `rullst-nexus/src/nexus/mod.rs:34-40`: `Nexus::new()` define `auth: None`;
- `:74-93`: dashboard, create, update, delete, batch, chat e páginas de segurança são montados antes de qualquer auth;
- `:140-147`: sem `.with_auth`, o sistema apenas imprime um aviso e continua servindo tudo;
- `rullst-nexus/src/nexus/crud/handlers.rs:258-312,365-387,423-462`: operações destrutivas não exigem `UserContext`, `RbacGuard` ou ownership;
- `examples/blog/src/lib.rs:344-374` monta o painel sem autenticação;
- blueprints Blog/LMS/Portfolio/ERP/SaaS geram `admin` / `password`, por exemplo `cargo-rullst/src/blueprints/saas/routes.rs:24-25,113-114`.

**Impacto:** dependendo de onde o router é montado, um visitante pode ler e alterar toda a base. Nos blueprints, as credenciais são públicas e previsíveis. Isso viola diretamente a invariável de RBAC/ownership.

**Correção necessária:** `Nexus::new()` deve ser fail-closed. A construção do router precisa falhar sem uma política autenticada, exceto em um modo local explicitamente tipado. Remover credenciais fixas dos templates; gerar segredo aleatório ou exigir configuração. Cada handler deve validar papel e ownership no servidor, inclusive batch actions e campos `readonly`/`hidden`.

### P0-05 — O gerador de CORS cria aplicações vulneráveis por padrão

**Evidência:** `cargo-rullst/src/generators/cors_jwt.rs:49-105`.

O middleware gerado lê o header `Origin`, devolve exatamente essa origem em `Access-Control-Allow-Origin` e habilita `Access-Control-Allow-Credentials: true` para qualquer valor diferente de `*`. Não existe allowlist.

**Impacto:** um site atacante escolhe sua própria origem, recebe autorização CORS credentialed e pode acessar respostas autenticadas se cookies/credenciais forem enviados. O problema é multiplicado porque está num scaffold: cada aplicação gerada herda a falha.

**Correção necessária:** gerar uma allowlist explícita a partir de configuração, rejeitar origem ausente/desconhecida, emitir `Vary: Origin`, desabilitar credenciais por padrão e usar `tower-http::cors` ou uma implementação testada. O template também deve eliminar `unwrap()` em produção.

### P0-06 — Storage permite escape do diretório e confirma operações inexistentes

**Evidência:** `rullst-core/src/storage.rs:88-124,159-167`.

O caminho principal `Storage::local().put/get()` faz `PathBuf::from(base).join(relative_path)` sem rejeitar `..`, prefixos Windows, caminhos absolutos, links simbólicos ou escape após canonicalização. Existe outro `LocalDriver` com validação, mas o facade público não o utiliza.

Ao mesmo tempo:

- upload S3/R2 retorna `Ok(())` sem enviar bytes;
- download S3/R2 retorna `Ok(vec![])`;
- `resize_webp` ignora as dimensões e devolve o arquivo original.

**Impacto condicional:** se `relative_path` vier do usuário, há leitura/escrita fora do storage base. Nos backends cloud, a aplicação recebe confirmação de persistência que nunca ocorreu, gerando perda silenciosa.

**Correção necessária:** unificar o facade com `LocalDriver`, validar componentes antes de I/O e comprovar que o caminho resolvido permanece sob o root. Backends não implementados devem retornar `Unsupported`, nunca sucesso. O resize também deve ser implementado ou removido.

### P0-07 — O modo de produção é fail-open e inconsistente

**Evidência:** `rullst-core/src/server/builder.rs:110-127,132-177,188-215,329-368`.

O `Server`:

- calcula desenvolvimento apenas com `APP_ENV != "production"`, de forma case-sensitive;
- ignora `RULLST_ENV`, o alias `prod` e `[app].env` já carregado de `Rullst.toml`;
- instala WAF/CSRF/headers apenas quando esse booleano é considerado produção;
- monta console/autofix de desenvolvimento no caso contrário;
- usa `dotenvy::from_filename_override(".env")`, permitindo que `.env` sobrescreva variáveis injetadas pelo ambiente;
- engole falha de inicialização de banco e continua subindo o HTTP;
- diante de `HOST` inválido, cai silenciosamente em `0.0.0.0`.

Auth e CSRF usam uma lógica diferente e aceitam `RULLST_ENV`, `APP_ENV`, `prod` e case-insensitive (`rullst-auth/src/auth.rs:137-145`; `rullst-core/src/security/csrf.rs:13-16`). Uma mesma aplicação pode, portanto, considerar-se produção para cookies/chaves e desenvolvimento para a pilha do Server.

**Impacto:** uma configuração aparentemente válida pode publicar console de desenvolvimento e remover toda a proteção automática. Erro de banco adiado pode virar panic em `Orm::pool()`.

**Correção necessária:** criar um único enum `Environment` validado, derivado de uma precedência documentada, e passá-lo a todos os crates. Valor ausente/desconhecido deve falhar em deploy explicitamente produtivo. Configuração fornecida ao builder não pode ser ignorada. Erros de DB solicitada devem abortar o startup.

### P0-08 — Integridade financeira de webhooks e billing não é segura por padrão

**Evidência:**

- vários providers retornam `Ok(())` quando `webhook_secret` está vazio, incluindo Stripe, Mercado Pago, Coinbase, Polar e Razorpay (`rullst-capital/src/providers/stripe.rs:30-32`, `mercadopago.rs:30-32`, `coinbase.rs:33-35`, `polar.rs:30-32`, `razorpay.rs:36-38`);
- Stripe/Mercado Pago autenticam o timestamp recebido, mas não limitam sua idade (`stripe.rs:34-66`; `mercadopago.rs:34-66`), permitindo replay de mensagens capturadas;
- `rullst-capital/src/providers/alipay.rs:58-82,126-145` chama o fluxo de RSA2, mas usa HMAC-SHA256 com a chave pública e não produz/valida RSA2;
- o gerador de billing atribui novas assinaturas ao usuário fixo `1` e usa email fixo (`cargo-rullst/src/generators/billing.rs:168-203,214-279`);
- segredo ausente vira o previsível `mock_secret` no mesmo template.

**Impacto:** endpoints podem aceitar eventos forjados, repetir cobrança/eventos antigos ou atribuir assinatura ao tenant errado. Alipay tende a não interoperar com o protocolo real.

**Correção necessária:** separar explicitamente verifier real e mock; endpoint público jamais pode aceitar segredo vazio. Adicionar freshness, idempotency store, associação customer→user/tenant obrigatória e testes de replay/cross-tenant. Implementar RSA2 com biblioteca apropriada ou remover o provider da lista de suporte.

### P0-09 — O workflow de release está topologicamente quebrado

**Evidência:** `.github/workflows/release.yml:69-113`; `rullst-core/Cargo.toml:34,42`.

O workflow publica `rullst-core` antes de `rullst-macros`, `rullst-orm-macros` e `rullst-orm`, apesar de Core depender deles. Também omite `rullst-security` e `rullst-iot` do publish, package e hashes. Nexus/Studio dependem de Security, de modo que uma release fresca pode parar no meio depois de pacotes já publicados. O empacotamento/attestation só ocorre **depois** da publicação.

Isso contradiz a ordem oficial do próprio `AGENTS.md` e do release guide.

**Impacto:** a próxima release coordenada pode falhar, ficar parcial e irreversível no crates.io.

**Correção necessária:** executar `cargo package` e todos os gates antes de publicar qualquer crate; calcular o DAG real; publicar macros, fundações, serviços, interfaces, umbrella e CLI nessa ordem; incluir Security/IoT e registrar idempotentemente quais versões já existem.

## 8. Achados altos — P1

### P1-01 — WebAuthn verifica parte do protocolo, mas omite invariantes essenciais

**Evidência:** `rullst-auth/src/auth/passkey/service.rs:86-216,252-350`.

Há pontos positivos: challenge, origin, `rpIdHash` e assinatura ECDSA da assertion são verificados. Isso não é um mock completo. Porém o fluxo de registro não valida:

- `clientDataJSON.type == "webauthn.create"`;
- flags User Presence/User Verification;
- algoritmo, `kty`, curva e tamanhos das coordenadas COSE;
- formato e assinatura da attestation;
- contador inicial real.

Na autenticação, não valida `type == "webauthn.get"`, flags UP/UV, vínculo entre `credential.raw_id` e a passkey recebida nem avanço monotônico do `sign_count`; apenas substitui o contador. A API também recebe o challenge esperado do chamador, sem demonstrar armazenamento one-time/expiração no próprio serviço.

**Impacto:** reduz garantias contra credenciais clonadas, cerimônias do tipo errado e autenticação sem presença/verificação do usuário. A exploração final depende dos controllers, mas as omissões são confirmadas.

**Recomendação:** usar uma implementação WebAuthn auditada ou completar toda a verificação normativa, com testes negativos por campo e persistência atômica de challenge/counter.

### P1-02 — Chaves de sessão fracas ou vazias são aceitas em produção

`rullst-auth/src/auth.rs:119-145` aceita imediatamente qualquer `APP_KEY` presente ou valor do TOML, inclusive string vazia ou curta. Só depois verifica se o ambiente é produção. A mensagem promete 32+ bytes, mas isso não é imposto. `derive_cipher` transforma o valor previsível em uma chave de 32 bytes via SHA-256 (`:172-190`), o que corrige tamanho, não entropia.

Além disso:

- `needs_rehash` (`:83-92`) olha apenas se o algoritmo é `argon2id`, ignorando memória/iterações/paralelismo/versão;
- `decrypt_session` aceita tokens legados sem expiração (`:248-268`);
- o crate anunciado como JWT não contém uma implementação JWT de sessão/aplicação; JWT aparece principalmente em Connect e no gerador do CLI.

**Recomendação:** validar comprimento/entropia e proibir vazio; versionar o envelope; remover fallback não expirável após janela de migração; comparar parâmetros Argon2 reais; documentar honestamente se JWT é responsabilidade do CLI ou do Auth.

### P1-03 — DLP e PII podem apagar, truncar ou tornar respostas protocolarmente inválidas

`rullst-security/src/dlp.rs:152-169` bufferiza qualquer resposta até 2 MiB. Em overflow/erro, devolve body vazio mantendo status e headers. Após mascarar, preserva `Content-Length`, `Content-Encoding` e `ETag` antigos e trata binário como UTF-8 lossy. Streaming, SSE, gzip e downloads podem ser corrompidos.

`rullst-core/src/security/pii.rs:12-50` tem problema semelhante; stream/overflow pode virar 500 e headers podem ficar incompatíveis com o novo corpo.

**Recomendação:** aplicar transformação somente a content-types textuais e bodies bufferizáveis; remover/recalcular headers; fazer bypass seguro para streaming/compressão/binário; nunca trocar overflow por sucesso com body vazio.

### P1-04 — CSRF e webhooks não têm composição utilizável em produção

O Server aplica CSRF sobre o router inteiro (`rullst-core/src/server/builder.rs:356-368`) sem mecanismo de exceção declarativa. Blueprints colocam `/billing/webhook` sob essa mesma pilha (`cargo-rullst/src/blueprints/saas/routes.rs:40-45,129-134`). Um provedor externo não possui o cookie/token double-submit e tende a ser bloqueado.

O próprio middleware só considera GET seguro (`rullst-core/src/security/csrf.rs:39-46`), então HEAD e OPTIONS também exigem token, quebrando semântica HTTP e preflight CORS.

**Recomendação:** criar políticas por rota. Webhook deve ficar isento de CSRF somente quando protegido por assinatura obrigatória, freshness e idempotência. Tratar GET/HEAD/OPTIONS/TRACE conforme semântica segura aplicável.

### P1-05 — WAF/RASP não fazem a inspeção profunda anunciada

`rullst-core/src/security/waf.rs:81-148` inspeciona query, Referer, Cookie e User-Agent, mas nunca o JSON/form body. `rullst-security/src/rasp.rs:76-98,138-169` também não lê body. Como o corpo é o principal vetor de dados em POST/PUT/PATCH, “deep inspection” é uma descrição incorreta.

Os detectores são heurísticos e baseados em substring/decodificação simples. Eles podem ser uma camada auxiliar, mas não devem ser apresentados como substituto de validação, bind SQL ou parser contextual.

### P1-06 — A política de headers não corresponde à alegação “A+ com nonces”

O CSP padrão contém `'unsafe-inline'` e `'unsafe-eval'` (`rullst-core/src/security/headers.rs:47-61`; `rullst-core/src/config.rs:68-70`). Isso reduz materialmente a proteção contra XSS e contradiz as alegações do README/SST. A implementação de nonce em `rullst-security/src/headers.rs:109-177` não entrega esse nonce ao request/renderizador, tornando difícil usá-lo nas views; quando `dynamic_csp=false`, não emite CSP.

**Recomendação:** uma policy única, nonce acessível via extension/contexto de rendering, CSP sem `unsafe-eval`, migração gradual de estilos inline e testes de headers na aplicação realmente servida.

### P1-07 — O middleware de webhook perde método, URI, extensões e body

`rullst-capital/src/webhook.rs:25-53` consome a request original, cria outra com `Request::new`, copia somente headers e a extensão do evento, e encaminha body vazio. Método, URI, versão e demais extensões voltam aos defaults.

**Impacto:** extractors downstream veem metadados falsos, não conseguem acessar o payload bruto e podem perder contexto de tenant/auth/tracing.

**Recomendação:** separar `parts` e body da request original antes da leitura, preservar todas as parts e reconstruir com o body original ou com bytes compartilhados.

### P1-08 — Inicialização global do ORM pode ficar irrecuperavelmente parcial

`rullst-orm/src/pool.rs:215-253` publica `DB_POOL` e `DB_DRIVER` nos `OnceLock`s antes de terminar a conexão de todas as réplicas. Se uma réplica falha, a função retorna erro, mas a primária global fica inicializada; uma nova tentativa falha como “already initialized”.

Os getters `Orm::pool()` e `driver()` usam `expect` (`:260-307`). Como o Server engole erro de inicialização (`rullst-core/src/server/builder.rs:152-177`), a aplicação pode subir degradada e quebrar mais tarde com panic.

**Recomendação:** preparar todo o estado localmente e publicar uma única estrutura atômica só após sucesso; disponibilizar somente getters fallible no caminho de produção; propagar falha de DB explicitamente solicitada no startup.

### P1-09 — O janitor do Login Guard limpa clones, não os mapas ativos

`rullst-security/src/login_guard.rs:42-52` clona os `DashMap`s e a task de limpeza retém entradas nesses clones. Ela não remove entradas dos mapas usados pelo guard original.

**Impacto:** a proteção anunciada contra crescimento de memória não funciona. Identidades únicas, especialmente se derivadas de headers forjáveis, podem fazer os mapas crescerem continuamente.

**Recomendação:** guardar mapas em `Arc<DashMap<...>>`, compartilhar exatamente as mesmas instâncias com a task e adicionar limite de cardinalidade/eviction.

### P1-10 — Rate limit “distribuído” é no-op e a identidade do cliente é forjável

`rullst-security/src/rate_limit.rs:72-80` apenas troca um enum em `with_distributed`; `check()` continua usando o mesmo mapa global em memória. `rullst-security/src/rate_limit.rs:109-122` e `rullst-core/src/resilience.rs:306-329` confiam diretamente em `X-Forwarded-For`/`X-Real-IP`.

Sem uma lista de proxies confiáveis, o cliente escolhe seu IP e contorna o limite. Sem `ConnectInfo`, clientes sem header podem cair no mesmo bucket `anonymous`. Diferentes instâncias/configurações ainda podem interferir por compartilhar mapa/chave.

**Recomendação:** implementar backend distribuído real ou retornar `Unsupported`; usar peer address e somente aceitar forwarded headers de proxies confiáveis; namespacing por limiter e janela/eviction coerentes.

### P1-11 — Há XSS e autorização somente visual no Nexus

Achados confirmados:

- resposta de LLM externo é inserida diretamente como HTML em `rullst-nexus/src/nexus/ai_chat.rs:264-296`;
- PK textual do banco entra sem escape em atributos, URLs e `onclick` em `rullst-nexus/src/nexus/crud/views.rs:75-82,123-136`;
- flags `readonly` e `hidden` só afetam a view; create/update aceitam os campos enviados manualmente (`rullst-nexus/src/nexus/crud/handlers.rs:156-165,286-297`);
- batch delete ignora erro do banco e informa sucesso (`rullst-nexus/src/nexus/crud/handlers.rs:455-467`);
- `page=0` pode causar underflow em `rullst-nexus/src/nexus/crud/query.rs:102-109`.

**Recomendação:** renderizar conteúdo não confiável como texto/HTML sanitizado por allowlist; nunca construir JS inline com dados; aplicar política de campos e RBAC nos handlers; validar paginação e propagar erro do banco.

### P1-12 — Studio mistura telemetria real, dados inventados e rotas quebradas

Pontos positivos: `RadarSnapshot::collect()` e `SpanCollector` são consultados em páginas reais. Porém:

- `rullst-core/src/radar.rs:91,129-140` usa fallback fixo de 24 MB, latência mínima artificial de 15 μs, CPU fixa em 0,5 e quantidade heurística de tasks;
- `rullst-studio/src/security_radar.rs:41-74,183-186` hardcoda integridade “100%” e estado “PRODUCTION_GUARD_ACTIVE”;
- o subrouter é aninhado em `/security/stats`, mas publica `/stats`, resultando em `/security/stats/stats`; o JS busca `/studio/security/stats` (`rullst-studio/src/lib.rs:52-64`; `security_radar.rs:35-39,326-330`);
- eventos entram via `innerHTML` sem escape (`security_radar.rs:344-355`), criando XSS quando a rota for corrigida;
- `env_viewer.rs:8-39` não reconhece `DATABASE_URL`/`REDIS_URL` como sensíveis e pode cortar UTF-8 em boundary inválido;
- feature flags interpolam nomes sem escape e usam placeholder incorreto em SELECT PostgreSQL (`feature_flags.rs:43-65,118-156`).

**Recomendação:** definir contrato de métrica com “indisponível” em vez de número falso; alinhar montagem e URLs; usar `textContent`; classificar segredos por deny/allowlist robusta; testar cada rota sob o mesmo prefixo usado pelos blueprints.

### P1-13 — Geradores do CLI têm regressões funcionais concretas

Além do CORS crítico, foram confirmados:

- `--nix` e `--buildah` são enviados na ordem trocada (`cargo-rullst/src/cli.rs:341-348`; `cargo-rullst/src/generators/project/mod.rs:43-50`);
- Island usa helpers que acrescentam `_controller`/`Controller`, gerando nomes errados, importa `rullst::view` inexistente e gera `unwrap()` (`cargo-rullst/src/generators/mod.rs:89-150`; `cargo-rullst/src/generators/island.rs:17-18,59-87`);
- Resource reutiliza os mesmos helpers e produz nomes/paths incoerentes (`resource.rs:27-106`);
- `cargo rullst new ../dummy_test` usa o path literal também como package/import name, sem separar basename e destino (`cargo-rullst/src/generators/project/wizard.rs:25-45`; `cargo-rullst/src/generators/project/mod.rs:54-69`);
- IDs dos blueprints foram deslocados pela inserção de Portfolio, contrariando o contrato estável 0/1/2/3 da SST (`cargo-rullst/src/generators/project/wizard.rs:91-103`; `cargo-rullst/src/blueprints/mod.rs:29-43`);
- o gerador Docs SSG previsto na SST não existe;
- grandes templates continuam inline em vez de `include_str!`/blueprints testáveis.

**Impacto:** comandos podem criar o artefato errado, código que não compila ou projetos incompatíveis com scripts/documentação anteriores.

**Recomendação:** criar testes end-to-end que geram cada combinação em diretório temporário e executam `cargo check`; congelar IDs públicos; separar `destination_path`, `package_name`, `module_name` e `type_name` em tipos distintos.

### P1-14 — AI não integra os guardrails anunciados e mocks são incompletos

Os providers presentes são Anthropic, Gemini, Ollama e OpenAI (`rullst-ai/src/ai/providers/mod.rs:1-8`); DeepSeek não existe. O fluxo usa `AiProvider/AiClient` com `Arc<dyn AiProvider>`, não o `LlmClient` descrito. Não foi encontrado um pipeline automático que aplique prompt-injection filter e PII masking antes de cada request; providers enviam mensagens diretamente.

OpenAI/Gemini possuem fallback em parte do chat para chave vazia/`mock_`, mas caminhos de vision/embedding ainda fazem HTTP (`rullst-ai/src/ai/providers/openai.rs:92-197`; `gemini.rs:124-164`). `structured_prompt` apenas adiciona instrução textual, remove fences e faz parse JSON, sem schema nativo (`rullst-ai/src/ai/mod.rs:238-249`).

**Recomendação:** tornar guardrails uma etapa não contornável do client de alto nível; aplicar mock a todas as capacidades; adicionar DeepSeek ou reduzir claims; separar “JSON parseável” de structured output com schema.

### P1-15 — Connect é sólido em OAuth, mas tem defaults/panics perigosos

Constructors gerados em `rullst-connect/src/macros.rs:21-27` e providers como Google/Auth0/Cognito usam `assert!` para credenciais e redirect URL. Credencial vazia não ativa fallback determinístico, contrariando a política do repositório e podendo causar panic em produção.

OIDC discovery aceita URL que apenas começa com `http://localhost`; um domínio como `localhost.evil` pode passar (`rullst-connect/src/providers/oidc/discovery.rs:43-65`). O metadata não amarra rigorosamente issuer/endpoints ao solicitado. JWKS permanece em cache sem TTL/refresh (`rullst-connect/src/provider/jwks.rs:14-47`), então rotação de chaves pode exigir restart.

**Recomendação:** constructors fallible com `impl Into<String>`; URL parsing por host exato; validação de issuer/endpoints HTTPS; cache JWKS com TTL, refresh on unknown `kid` e stale-if-error.

### P1-16 — Mail tem validações úteis, mas não as torna invariantes

Drivers Resend/SendGrid/Postmark fazem request real mesmo com credenciais vazias/`mock_*`; o fallback Memory existe, porém não é selecionado automaticamente (`rullst-mail/src/drivers/*.rs`; `rullst-mail/src/facade.rs:131-174`). `send_for_tenant` ignora `tenant_id` em um caminho do facade. Validações de CRLF, segredos e deliverability existem, mas não são necessariamente chamadas antes do envio.

Tracking compara HMAC como string comum, aceita segredo vazio e não expira timestamp (`rullst-mail/src/tracking.rs:89-101,153-165`).

**Recomendação:** um único pipeline de envio deve validar segurança e selecionar transport real/mock de maneira tipada. Tracking deve exigir segredo forte, comparação constante e TTL.

### P1-17 — Política zero-panic é violada em caminhos de produção

Exemplos confirmados:

- `rullst-core/src/client.rs:19-31`: `expect`/`unwrap` em todas as etapas do client WASM;
- `rullst-core/src/server/builder.rs:52-61,222-232`: `panic!` deliberado para hot reload em release;
- `rullst-orm/src/pool.rs:260-307`: getters públicos com `expect`;
- `rullst-orm-macros/src/relationships.rs:119-139,348-356`: código gerado com panic/expect/unwrap;
- `cargo-rullst/src/generators/auth/controllers.rs:118,170,183` e `island.rs:75-86`: aplicações geradas com unwrap/panic.

O total bruto de ocorrências inclui muitos testes e não deve ser tratado como contagem de bugs. Esses exemplos, contudo, são caminhos não-test comprovados e contradizem a regra explícita da SST.

### P1-18 — Contexto de tenant pode ser escolhido pelo próprio cliente

`rullst-core/src/security/tenant_guard.rs:26-70` aceita `X-Tenant-ID`/`X-Organization-ID` e insere esse valor como contexto sem demonstrar vínculo com a identidade autenticada. Se repositories usam esse contexto para isolamento, um usuário pode trocar o header e selecionar outro tenant.

**Recomendação:** derivar tenant de claims/sessão e validar membership/role; headers de tenant só podem ser aceitos de gateway interno confiável e assinados/autenticados.

### P1-19 — CSWSH permite host com prefixo enganoso

`rullst-security/src/cswsh.rs:29-45` aceita origin cujo host começa com `localhost` ou `127.0.0.1`. Assim, `localhost.evil.example` pode ser classificado como local.

**Recomendação:** fazer parse de URL e comparar host exato/IP, esquema e porta contra allowlist normalizada.

## 9. Achados médios e dívida técnica relevante — P2

Os itens abaixo não têm todos a mesma urgência, mas devem entrar no backlog antes de chamar a versão de estável.

| ID | Achado confirmado | Evidência/efeito |
|---|---|---|
| P2-01 | Auditoria encadeada usa serialização ambígua | `rullst-security/src/audit/chain.rs:77-128` concatena campos com `:` sem escaping/length-prefix. Campos diferentes contendo `:` podem gerar o mesmo material assinado. `verify_record` valida um registro isolado, não a continuidade de uma sequência. Chave vazia também é aceita. |
| P2-02 | Telemetria declara HMAC sem verificar HMAC | `rullst-security/src/telemetry.rs:103-195` grava `verified_hmac: true` em eventos locais e usa IP `127.0.0.1`; `current_timestamp_str` usa epoch `% 60` (`:288-298`), então “59s ago” volta a “Just now” a cada minuto. |
| P2-03 | Honeypot confia em XFF e bane indefinidamente | `rullst-security/src/honey/middleware.rs:51-74,120-144` aceita identidade forjável, usa substring para trap e mantém bans sem expiração, criando bypass, falso positivo e crescimento de memória. |
| P2-04 | TrafficShield pode panic fora de runtime | `rullst-core/src/resilience.rs:74-113` chama `tokio::spawn` em construtor síncrono. As tasks não têm cancelamento no drop. |
| P2-05 | Scheduler permite sobreposição ilimitada | `rullst-core/src/scheduler.rs:129-154` cria novo task a cada tick, sem impedir job anterior ainda em execução, timeout, shutdown ou política de panic. |
| P2-06 | Queue worker faz spawn sem limite e pode deixar jobs presos | `rullst-core/src/queue/worker.rs:69-105` ignora erros de `mark_complete/failed`; crash pode deixar job em `processing`. JSON inválido vira `Null` silenciosamente em `rullst-core/src/queue/sqlite.rs:127-146`. |
| P2-07 | Hot-swap descarrega biblioteca potencialmente em uso | `rullst-core/src/server/hotswap.rs:104-115` libera biblioteca antiga após três reloads sem provar que requests em voo terminaram. O loader não verifica ponteiro null antes de `Box::from_raw` (`dylib_loader.rs:102-107`). É risco de UB no modo dev. |
| P2-08 | `std::env::set_var` unsafe dentro do runtime | `rullst-core/src/server/builder.rs:188-192` altera ambiente global potencialmente após threads terem iniciado, sem uma invariável documentada. |
| P2-09 | Replicação de DB é simulação | `rullst-core/src/db.rs:84-98` registra que está “sincronizando” e comenta que imprime sucesso para emular replicação. Deve retornar `Unsupported` ou ser rotulado experimental. |
| P2-10 | Substituição de placeholders SQL é textual | `rullst-orm/src/pool.rs:50` substitui `?` sem parser, podendo alterar literais/comentários. A estratégia precisa ser consciente do dialeto ou removida. |
| P2-11 | Validação de identificador aceita hífen | `rullst-orm/src/schema/validation.rs:9-50` permite hífen que depois pode ser usado sem quote, produzindo SQL inválido/ambíguo. |
| P2-12 | Auditoria ORM perde diffs e segredos aninhados | `rullst-orm/src/audit.rs:108-155` pode interpretar JSON inválido/array/primitivo como ausência de diferença; masking cobre apenas primeiro nível. |
| P2-13 | MFA aceita código abreviado | `rullst-security/src/mfa.rs:82-106` converte TOTP para `u32` sem exigir seis dígitos; zeros iniciais podem ser omitidos. URI `otpauth` usa escape HTML, não percent-encoding (`:110-116`). |
| P2-14 | Formulário de auth gerado bloqueia Tokio e vaza timing | `cargo-rullst/src/generators/auth/controllers.rs` faz hash/verify síncrono em handlers async, carrega usuários e busca email linearmente, retorna cedo para usuário inexistente e contém unwraps. |
| P2-15 | JWT gerado é incompleto | `cargo-rullst/src/generators/cors_jwt.rs:162-225` injeta `jsonwebtoken = "9.3"`, divergindo de outras versões, sem issuer/audience e sem validar força do segredo. |
| P2-16 | `send_for_tenant` não resolve tenant | O facade de Mail ignora `tenant_id` em um caminho, apesar do nome da API. Isso pode aplicar transport/config global quando o chamador espera isolamento. |
| P2-17 | Rotas Studio mantêm aliases fora do padrão | Existem variantes `/tools/*` e `/studio/tools/*`, enquanto a SST exige URLs limpas `/studio/*`. Backward compatibility pode justificar aliases, mas a rota canônica precisa ser única e testada. |
| P2-18 | Macro `#[route]` é pública mas fundacional | `rullst-macros/src/lib.rs:196-216` ignora atributo/path e reemite uma função de forma incompleta. Se for API pública anunciada, pode gerar surpresa ou erro de compilação. |
| P2-19 | Sinais de API semver incompletos | Muitos enums/configs públicos não têm `#[non_exhaustive]`; não foi encontrada uma trilha relevante de `#[deprecated]`. Adicionar campo/variante pode quebrar consumidores. |
| P2-20 | Construtores não seguem ergonomia uniforme | Há muitos `new(String, ...)` em vez de `impl Into<String>` e vários constructors validam com panic/assert. Isso é dívida de API, além dos casos de segurança já citados. |
| P2-21 | Dependência `mutants` está em dependências normais do ORM | `rullst-orm/Cargo.toml` inclui `mutants = 0.0.4` no conjunto regular; deve ser confirmado se é necessário em runtime ou movido para tooling/dev. |
| P2-22 | Compliance gerado é incondicional | `cargo-rullst/src/generators/audit.rs:508-521` marca OWASP, criptografia, CSP, SOC2, TLS e SBOM como PASS sem testar o projeto, inclusive afirmando Vault AES-256. Isso produz evidência falsa. |
| P2-23 | Basic Auth do Nexus não tem rate limit | A senha usa comparação constante, o que é positivo, mas o username usa igualdade comum e não há rate limiting/lockout na camada montada. Basic também depende integralmente de TLS externo. |
| P2-24 | Fallback de memória do Radar não é “telemetria real” | Quando coleta real falha, retornar 24 MB e CPU fixa é pior que expor `None/Unavailable`, porque dashboards e alertas passam a tomar decisões com dado inventado. |

## 10. Matriz de aderência à SST

| Área da SST | Estado | Avaliação |
|---|---|---|
| Workspace e existência dos crates | **Parcial** | Os crates principais existem, mas responsabilidades reais de Connect/IoT divergem e Security está duplicada. |
| Core HTTP (`routes!`, `Router`, `Server`) | **Boa** | API legível e substancial. O builder precisa de startup fail-closed. |
| `html!`, escape e `RawHtml` | **Boa** | Escape automático é real; bypass é explícito. |
| ORM Active Record/Repository | **Parcial/boa** | CRUD, macros e binds existem; assinaturas divergem em pontos, globais/panics prejudicam robustez. |
| CLI e scaffolding | **Fraca** | Amplitude alta, mas há bugs concretos de flags, nomes, paths, IDs, CORS, auth e billing. Docs SSG não existe. |
| Erros e zero-panic | **Fraca** | Erros tipados são comuns, porém há panics em runtime, macros e código gerado. |
| Middleware e segurança | **Parcial/fraca** | Bons controles isolados coexistem com detecção fail-open, WAF sem body, CSP fraca e pilhas duplicadas. |
| Compatibilidade pública | **Fraca** | `#[non_exhaustive]` e depreciação não são aplicados de modo uniforme. |
| External providers/mocks | **Parcial** | Capital tem vários mocks, mas Mail/Connect/AI não cobrem todos os caminhos; vazio pode também abrir webhook. |
| Capital/pagamentos | **Parcial** | Muitos providers reais; webhooks/billing/Alipay têm falhas relevantes. |
| Fiscal NFS-e | **Crítica/não aderente** | XMLDSig, mTLS e parser de autorização não são implementações válidas. |
| Auth | **Parcial** | Argon2 e AES-GCM reais; JWT ausente no crate e WebAuthn incompleto. |
| AI | **Parcial** | Quatro providers existem; falta DeepSeek, guardrails automáticos e mocks completos. |
| Studio | **Parcial** | Parte da telemetria é conectada; há métricas falsas, rota quebrada e XSS latente. |
| Nexus | **Crítica** | CRUD existe, mas não é fail-closed nem aplica RBAC/ownership server-side. |
| Connect | **Não aderente ao papel** | É OAuth/OIDC, não mensageria/streaming. Como OAuth, tem substância. |
| IoT | **Não aderente/protótipo** | MQTT, OTA, HSM e PQC não cumprem as garantias anunciadas. |
| Mail | **Parcial** | Drivers/fila existem; segurança/fallback/tenant não são invariantes do facade. |
| Studio URLs e dados reais | **Parcial** | Existem aliases legacy e estados hardcoded proibidos pela SST. |

Minha estimativa global permanece em **55–60% de aderência**, mas a média esconde extremos: HTML/routing estão bem mais próximos do contrato; Fiscal/IoT/Connect e segurança administrativa estão muito abaixo.

## 11. Testes, CI e release

### 11.1. Distribuição estática dos testes

| Área | Declarações de teste encontradas |
|---|---:|
| `rullst-connect` | 182 |
| `rullst-core` | 173 |
| `rullst-orm` | 129 |
| `rullst-security` | 83 |
| `rullst` | 79 |
| `rullst-capital` | 54 |
| `rullst-mail` | 53 |
| `rullst-auth` | 34 |
| `rullst-iot` | 34 |
| `rullst-studio` | 34 |
| `rullst-ai` | 22 |
| `rullst-orm-macros` | 22 |
| `rullst-nexus` | 18 |
| `cargo-rullst` | 9 |
| `rullst-macros` | 1 |
| **Total** | **927** |

O volume é um ponto positivo, mas a distribuição mostra uma lacuna: o CLI, que gera uma grande quantidade de código e concentra regressões observadas, tem apenas nove declarações de teste. O exemplo Blog não tem testes próprios. Muitos testes verificam a própria simulação como comportamento desejado — por exemplo, OTA aceita `[0; 64]` e Vault considera uma string de hash como “decriptada”. Portanto, quantidade sem qualidade de oráculo pode até congelar um bug.

### 11.2. Pontos positivos da automação

- testes workspace all-features na matriz Linux/macOS/Windows (`.github/workflows/ci.yml:44-62`);
- MSRV explícito e job específico (`ci.yml:74-89`);
- bancos reais via testcontainers na matriz (`ci.yml:64-72`);
- targets no_std reais em workflow próprio;
- ampla presença de fuzz, bench, Kani, Miri, cargo-deny/audit e supply-chain checks;
- a maioria das actions está pinada.

### 11.3. Gaps dos gates

1. **Clippy oficial não usa todas as features.** `ci.yml:39` e `release.yml:38` executam `--all-targets`, não `--all-features`. Um Clippy adicional cobre somente três features do umbrella (`ci.yml:41-42`). Isso não cumpre a regra do AGENTS.

2. **Features strict de DB não são compiladas isoladamente.** `--all-features` ativa strict Postgres/MySQL/SQLite simultaneamente, mas condicionais do ORM dão precedência a Postgres (`rullst-orm/src/lib.rs:20-44`). Não há jobs dedicados `strict-mysql`, `strict-sqlite` e `strict-postgres`, então branches exclusivas podem apodrecer.

3. **WASM sempre pode ficar verde.** Os dois `cargo check` de `.github/workflows/wasm-matrix.yml:39-40` terminam com `|| true`.

4. **Unsafe policy é informativa, não enforcement.** `.github/workflows/unsafe-policy.yml:76-77` sempre termina em `exit 0`, e a busca omite Security/IoT. A lógica de comentário SAFETY também não comprova comentário precedente. Há unsafe real em `dylib_loader.rs`, `builder.rs` e `radar.rs`.

5. **Kani/Miri/mutants/udeps não bloqueiam.** Kani e Miri têm `continue-on-error`; mutants e udeps usam `|| true`. Isso pode ser uma escolha aceitável de telemetria, mas a documentação não pode chamá-los de gates formais.

6. **A matriz do workflow manual de fuzz não cobre todos os targets.** Há 10 manifests de fuzz e 40 targets, mas `.github/workflows/fuzzing.yml`, acionado por `workflow_dispatch`, inclui 34. Ficam fora exatamente `fuzz_dlp`, `fuzz_sanitizer`, `fuzz_message_serde`, `fuzz_email_validator`, `fuzz_email_tracking` e `fuzz_email_security`.

7. **O job “IoT QEMU” não executa QEMU.** Ele não instala nem roda em hardware simulado. O workflow também possui unit tests, mas esse job comprova somente compilação para o target, não comportamento em QEMU/hardware simulado.

8. **O benchmark tolera regressão enorme.** A documentação fala em regressão de nanossegundos, mas o YAML configura `alert-threshold: '300%'`.

9. **Quatro actions não estão pinadas por SHA.** Foram observadas em corpus sync, udeps e release. Em pipeline de publicação, pinning deve ser obrigatório.

10. **`deny.toml` ignora 11 advisories sem owner/prazo.** Exceção pode ser necessária, mas precisa de justificativa, responsável, compensating control e data de expiração.

### 11.4. O que a CI ainda não prova

Mesmo quando o workflow está verde, ele não prova:

- que NFS-e é aceita pela SEFIN;
- que OTA valida Ed25519;
- que HSM/PQC são algoritmos reais;
- que Nexus está protegido quando montado;
- que CORS gerado possui allowlist;
- que todos os scaffolds compilam;
- que Studio exibe dados reais;
- que os checks formal/unsafe/mutation são gates.

Esses comportamentos precisam de testes de contrato que falhem com as implementações atuais, não apenas testes unitários que confirmem stubs.

## 12. Documentação, compliance e honestidade de produto

A divergência documental é um dos maiores riscos do projeto, pois influencia decisões de adoção.

### 12.1. Alegações que o código atual contradiz

| Alegação | Realidade observada |
|---|---|
| “100% memory safe / no unsafe blocks” (`README.md:50`; `SECURITY_COMPLIANCE.md:14`) | Há unsafe de produção no loader dinâmico, builder e FFI do Radar. Unsafe pode ser legítimo, mas a alegação zero é falsa. |
| “Zero-Panic / 100% crash-free” (`README.md:284`) | Há `panic!`, `expect` e `unwrap` em Core, ORM, macros e scaffolds de produção. |
| “OWASP A+ / CSP com nonces” (`README.md:186`; `AUDIT.md:66`) | O Server padrão usa CSP com `unsafe-inline`/`unsafe-eval`; a pilha com nonce nem é a mesma instalada por padrão. |
| “Vault AES-256” (`SECURITY_COMPLIANCE.md:10`) | `rullst-security::FieldEncryptor` é hash irreversível; somente o ORM tem AES-GCM real. |
| “RBAC/IDOR enforced” (`SECURITY_COMPLIANCE.md:9`) | Nexus não exige auth/RBAC/ownership por padrão e Tenant Guard aceita header sem vínculo. |
| “MQTT 5 industrial” (`README.md:66,382`) | `mqtt.rs` é stub e `MqttDriver` só converte valor para string. |
| “PQC ML-KEM/HSM compliance” (`README.md:70`) | PQC e HSM são hashes simulados explicitamente marcados como stubs. |
| “NFS-e / XMLDSig ICP-Brasil” (`docs/src/spec.md:251-254`) | Não há assinatura com chave privada, C14N, mTLS ou parser de autorização estrito. |
| “DeepSeek + guardrails/PII built-in” (`README.md:278,352`) | DeepSeek está ausente e guardrails não formam pipeline obrigatório antes de requests. |
| “Connect: Kafka/RabbitMQ/Redis Streams/WS/SSE” (`docs/src/spec.md:306`) | O crate atual implementa OAuth/OIDC/social login. |
| “Compliance PASS” gerado pelo CLI | O gerador imprime PASS incondicionalmente sem avaliar o projeto. |

### 12.2. Documentos de auditoria não são evidência reproduzível atual

`AUDIT.md` declara 739 dependências e três advisories, enquanto o `Cargo.lock` contém 773 packages e o `deny.toml` atual ignora 11 advisories. Ele também declara Clippy all-targets/all-features limpo e nota 9,8/10, algo que não pôde ser reproduzido neste ambiente. `SECURITY_COMPLIANCE.md` afirma zero unsafe apesar de blocos existentes.

O SBOM, por outro lado, lista 773 componentes e tem timestamp recente, alinhando-se melhor ao lock. O caminho correto é gerar todos esses relatórios em CI a partir do commit/tag e anexar logs/artefatos verificáveis, nunca manter PASS estático no repositório.

### 12.3. Estado de versão confuso

- todos os manifests publicáveis estão em `12.0.0`;
- o `CHANGELOG.md` ainda trata 12 como Unreleased;
- as tags locais observadas vão somente até `v5.0.0`;
- o último commit usa `feat: ...`, sem o scope obrigatório por `<type>(<scope>): <summary>`.

Isso não é bug runtime, mas reduz confiança na governança de release. Versão do código, changelog, tag, crates publicados, SBOM e release notes precisam apontar para a mesma realidade.

## 13. Manutenibilidade e qualidade estrutural

### 13.1. Arquivos acima da meta de 500 linhas

| Linhas | Arquivo |
|---:|---|
| 674 | `rullst-core/src/artisan/studio_views.rs` |
| 597 | `rullst-core/src/error_console/renderer.rs` |
| 588 | `cargo-rullst/src/cli.rs` |
| 582 | `cargo-rullst/src/blueprints/portfolio.rs` |
| 581 | `cargo-rullst/src/blueprints/erp.rs` |
| 576 | `rullst-connect/src/extractors.rs` |
| 569 | `rullst-core/src/cache.rs` |
| 545 | `rullst-core/src/resilience.rs` |
| 511 | `cargo-rullst/src/generators/audit.rs` |
| 504 | `rullst-mail/src/lib.rs` |

A regra é um alvo, não um limite absoluto. Ainda assim, esses são bons candidatos para decomposição porque concentram lógica de domínio, rendering/templates e branches de feature.

### 13.2. Templates inline aumentam drift

Controller, Island, CORS/JWT, Dockerfile e vários blueprints são strings grandes dentro de Rust. Isso dificulta syntax highlighting, snapshots, format/check do código gerado e reutilização. A SST já prescreve `include_str!` e templates separados; seguir essa regra permitiria testar cada arquivo gerado diretamente.

### 13.3. Ausência de TODO não significa completude

Não há grande concentração de `TODO`/`FIXME`, mas operações incompletas estão implementadas como sucesso ou como API final: S3/R2, DB replication, MQTT, OTA, HSM, PQC, resize, fiscal e telemetria. Um `todo!()` visível seria, em vários casos, menos perigoso que `Ok(())` falso.

### 13.4. APIs deveriam tornar estados inválidos irrepresentáveis

Vários problemas decorrem de `String`, bools e opções soltas:

- ambiente é uma string interpretada diferentemente por cada crate;
- secret vazio significa ora mock, ora endpoint sem autenticação, ora request real;
- `Nexus` permite construir router sem política de auth;
- Fiscal mock e Fiscal autorizado compartilham a mesma resposta;
- backends não implementados retornam sucesso.

Tipos como `Environment`, `CredentialMode<Real|Mock>`, `VerifiedWebhook`, `AuthorizedNfse`, `AuthenticatedNexus` e `ImplementedStorageBackend` poderiam mover essas garantias para compile-time.

## 14. O que eu manteria, refatoraria e reescreveria

### Manter e evoluir

- a separação nominal em crates;
- `routes!`, Router e a ergonomia geral do Server;
- o modelo seguro de escape do `html!`/`RawHtml`;
- a decomposição das macros ORM e o uso de SQLx bind;
- AES-GCM de `rullst-orm::privacy`;
- os providers OAuth/OIDC de Connect, renomeando/redefinindo a responsabilidade do crate se necessário;
- os providers reais de Capital/Mail/AI que já têm testes e clientes HTTP claros;
- a infraestrutura multi-OS, MSRV, fuzz, bench e testcontainers.

### Refatorar profundamente

- grafo Core→ORM e features do umbrella;
- uma única pilha Security integrada ao Server;
- resolução de ambiente/configuração e startup fail-closed;
- estado global do ORM;
- facades de Mail/Storage/providers e política de mock;
- montagem/segurança/telemetria do Studio e Nexus;
- geradores do CLI e seus templates.

### Reescrever ou desabilitar até existir implementação real

- `rullst-security::FieldEncryptor`;
- signer e parser NFS-e;
- OTA/HSM/PQC/MQTT industrial de IoT;
- RSA2 de Alipay;
- S3/R2 e replication stubs que retornam sucesso;
- middleware CORS gerado;
- compliance generator que imprime PASS incondicional.

## 15. Roadmap recomendado

### Fase 0 — contenção imediata, antes de qualquer nova release

1. Marcar Fiscal, IoT crypto/MQTT, `FieldEncryptor`, S3/R2 e Alipay como `experimental` ou fazer seus caminhos reais retornarem `Unsupported`.
2. Tornar Nexus fechado por padrão; remover `admin/password` de todos os blueprints e exemplos.
3. Corrigir o CORS gerado e publicar aviso de segurança para projetos já scaffoldados.
4. Corrigir path traversal do facade Storage.
5. Unificar detecção de produção e impedir subida sem as proteções configuradas.
6. Exigir segredo de webhook em endpoints reais; nunca usar fallback mock em rota pública.
7. Bloquear o workflow de release atual e corrigir ordem/omissões antes da próxima tag.
8. Atualizar README, SST, `AUDIT.md` e compliance para diferenciar “implementado”, “experimental” e “roadmap”.

### Fase 1 — segurança e confiabilidade do kernel

1. Criar um enum de ambiente único e uma política de configuração com precedência testada.
2. Fazer DB initialization atômica/fallible; remover getters panicking dos caminhos normais.
3. Validar APP_KEY e retirar sessão legacy sem expiração.
4. Corrigir WebAuthn com biblioteca/fluxo auditado.
5. Corrigir DLP/PII para content-type, streaming e headers.
6. Separar CSRF de webhooks assinados e adicionar freshness/idempotência.
7. Corrigir Login Guard, rate limit, trusted proxies, Tenant Guard e CSWSH.
8. Eliminar panics/unwraps confirmados em produção e código gerado.

### Fase 2 — integridade de produto e scaffolding

1. Criar harness que execute todos os comandos do CLI em tempdirs e rode `cargo fmt --check`, `cargo check` e testes smoke nos projetos gerados.
2. Corrigir flags Nix/Buildah, nomes Island/Resource, path/package, IDs de blueprint, Auth, Billing e Docs SSG.
3. Aplicar RBAC/ownership e field policy server-side no Nexus.
4. Corrigir rotas, escaping, env redaction e fonte de métricas do Studio.
5. Completar mocks offline de AI/Mail/Connect sem transformar endpoints reais em fail-open.

### Fase 3 — alinhar arquitetura e contrato

Escolher explicitamente uma das duas estratégias:

**Estratégia A — cumprir a SST atual:** implementar Connect de mensageria, MQTT 5/zero-copy, DeepSeek, guardrails automáticos, NFS-e real e todos os contratos descritos.

**Estratégia B — reduzir o contrato da versão 12:** documentar Connect como OAuth, mover mensageria para outro crate/roadmap, marcar IoT/Fiscal experimentais e remover comparativos/claims ainda não sustentados.

A Estratégia B entrega confiança mais rápido. A Estratégia A entrega a visão completa, mas exige um ciclo de engenharia e homologação considerável.

Depois, desacoplar Core do ORM, consolidar Security, completar o umbrella e padronizar `#[non_exhaustive]`, builders e `impl Into<String>`.

### Fase 4 — engenharia de release

1. Tornar a trifeta do AGENTS um gate real com `--all-features`.
2. Compilar/testar cada feature strict isoladamente.
3. Fazer unsafe, WASM, Kani/Miri e mutation tests refletirem claramente se são bloqueantes ou informativos.
4. Cobrir os 40 fuzz targets ou documentar tiers.
5. Empacotar/validar todos os crates antes de publicar o primeiro.
6. Gerar SBOM, audit e compliance por tag, com commit digest e artefatos assinados.
7. Alinhar `12.0.0`, changelog, tag, crates.io e release notes.

## 16. Gates mínimos para considerar o framework pronto

Eu só chamaria o conjunto de production-ready quando os seguintes critérios fossem reproduzíveis:

### Build e qualidade

- `cargo fmt --all -- --check` verde;
- `cargo clippy --workspace --all-targets --all-features -- -D warnings` verde;
- `cargo test --workspace --all-features` verde em Linux/Windows/macOS;
- cada feature exclusiva de DB compilada e testada isoladamente;
- cada blueprint/gerador produz projeto que compila;
- nenhuma API de produção retorna sucesso para operação não implementada.

### Segurança

- Nexus inacessível sem auth e RBAC/ownership testados em create/read/update/delete/batch;
- CORS por allowlist, CSP real com nonce e headers testados na aplicação final;
- APP_KEY/webhook secrets validados e sem modo vazio fail-open;
- WebAuthn com suíte negativa normativa;
- DLP/PII testados com JSON, HTML, gzip, binário, SSE, streaming e overflow;
- tenant e IP derivados de identidade/proxy confiável;
- inventário de `unsafe` com comentário SAFETY e teste de fronteira FFI;
- zero panics comprovado nos caminhos de runtime definidos pela política.

### Fiscal, pagamentos e IoT

- NFS-e homologada de ponta a ponta contra ambiente oficial, incluindo rejeições;
- XMLDSig validada independentemente e certificado/mTLS real;
- webhooks com replay/freshness/idempotência e testes cross-tenant;
- OTA com vetores Ed25519 válidos/inválidos e anti-rollback;
- HSM/PQC somente com implementação real ou nomes explícitos de simulação;
- MQTT validado contra broker real e parser fuzzado.

### Observabilidade e documentação

- métricas desconhecidas aparecem como indisponíveis, nunca como números inventados;
- Studio/Nexus sem XSS e com rotas smoke-tested;
- README/SST/compliance gerados ou revisados contra o mesmo commit;
- toda capacidade marcada como estável, experimental ou roadmap.

## 17. Priorização resumida

| Horizonte | Prioridade |
|---|---|
| **Agora** | Fiscal, falso Vault, IoT crypto, Nexus/default credentials, CORS gerado, Storage traversal, ambiente fail-open, webhooks/billing e release pipeline. |
| **Em seguida** | WebAuthn, APP_KEY, ORM atômico, DLP/PII, CSRF/webhooks, WAF claims, rate limit, tenant isolation, panics. |
| **Depois** | CLI end-to-end, Studio, AI guardrails/mocks, Mail, Connect/OIDC hardening, semver e arquivos grandes. |
| **Estratégico** | Decidir SST vs roadmap, desacoplar Core/ORM, unificar Security e completar umbrella. |

Este relatório registra **9 grupos P0, 19 grupos P1 e 24 itens P2**. Isso não representa 52 CVEs independentes: alguns são lacunas de produto ou grupos de bugs relacionados. Ainda assim, a quantidade e a severidade justificam uma fase de hardening antes de expandir funcionalidades.

## 18. Veredito final

### A arquitetura está boa?

**A ideia está boa; a arquitetura executada ainda precisa de consolidação.** A divisão por crates, o estilo explícito, as macros e a direção compile-time são fundamentos fortes. Eu daria 7/10 para a visão e aproximadamente 5,5/10 para o grafo/contratos reais. O acoplamento Core→ORM, a segurança duplicada e a diferença entre crate anunciado e crate implementado são os maiores problemas arquiteturais.

### Existe muita coisa a melhorar?

**Sim.** Não apenas estilo, documentação ou cobertura: há áreas que precisam ser reescritas ou bloqueadas para produção. A boa notícia é que não é necessário jogar o framework fora. O trabalho deve ser de redução de escopo, hardening e honestidade contratual, preservando routing, HTML, fundamentos ORM, providers já substanciais e infraestrutura de testes.

### Ele apresenta muitos bugs ou erros?

**Há vários bugs importantes e múltiplos bloqueadores críticos**, mas eles estão concentrados em certas fronteiras: Fiscal, IoT, Nexus, Security/Vault, storage, scaffolds, configuração de produção, webhooks e release. Outras partes estão significativamente mais maduras. Portanto, a resposta correta não é “tudo está quebrado”, e sim “a maturidade é muito desigual e as promessas enterprise ultrapassaram a implementação”.

### Eu usaria hoje?

- Para estudo, prototipagem local e evolução do próprio framework: **sim**.
- Para uma aplicação controlada usando somente routing/HTML e uma parte revisada do ORM: **possivelmente, com hardening e pin do commit**.
- Para fiscal real, OTA, HSM/PQC, painel Nexus exposto, billing multi-tenant gerado ou claims de compliance: **não na forma atual**.
- Para uma release pública estável 12.0.0 com todas as alegações do README: **não antes dos P0 e dos gates mínimos**.

Minha recomendação principal é simples: **reduzir temporariamente o que o Rullst afirma entregar e elevar a qualidade do que permanece estável**. Corrigir essa distância entre promessa e implementação aumentará mais a credibilidade do projeto do que adicionar novos módulos neste momento.

---

## Apêndice A — Referências centrais para a correção

- SST: `docs/src/spec.md`
- Server e ambiente: `rullst-core/src/server/builder.rs`
- Storage: `rullst-core/src/storage.rs`
- Telemetria Radar: `rullst-core/src/radar.rs`
- CSRF/WAF/headers: `rullst-core/src/security/`
- Estado global ORM: `rullst-orm/src/pool.rs`
- Criptografia ORM real: `rullst-orm/src/privacy.rs`
- Vault falso: `rullst-security/src/vault.rs`
- Login/rate/DLP: `rullst-security/src/login_guard.rs`, `rate_limit.rs`, `dlp.rs`
- Passkeys: `rullst-auth/src/auth/passkey/service.rs`
- Signer/cliente fiscal: `rullst-capital/src/fiscal/signer.rs`, `client.rs`
- Webhooks/providers: `rullst-capital/src/webhook.rs`, `providers/`
- IoT: `rullst-iot/src/ota.rs`, `hsm.rs`, `pqc.rs`, `mqtt.rs`
- Nexus: `rullst-nexus/src/nexus/`
- Studio: `rullst-studio/src/`
- CLI/scaffolds: `cargo-rullst/src/generators/`, `blueprints/`
- CI/release: `.github/workflows/ci.yml`, `.github/workflows/release.yml`

## Apêndice B — O que não foi alterado

Esta auditoria não corrigiu o código-fonte. O único artefato criado para atender ao pedido é este arquivo `gpt.md`. A próxima etapa recomendada é transformar os itens P0 em issues pequenas, cada uma com teste de regressão e critério de aceite reproduzível.
