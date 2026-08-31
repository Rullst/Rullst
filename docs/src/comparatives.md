# Comparativo técnico do Rullst e prioridades competitivas

> **Fotografia em 27 de agosto de 2026.** Este documento compara capacidades
> documentadas e verificáveis, não popularidade percebida nem slogans. A fonte
> normativa do Rullst continua sendo a [especificação](spec.md); o
> [capability ledger](capability-ledger.md) registra limites e o
> [programa v12](v12.md) contém os gates de release.

## Resposta curta

O Rullst já tem uma combinação incomum no ecossistema Rust: servidor Axum/Tokio,
ORM, autenticação, segurança em profundidade, IA multi-provider, pagamentos,
email, filas/realtime, Admin CMS, control room e uma pequena fundação `no_std`
para IoT sob uma mesma arquitetura tipada. Em critérios específicos, essa
integração já é mais ampla que o núcleo oficial de vários frameworks
comparados.

Isso ainda não permite afirmar que o Rullst é **o melhor framework do mundo** ou
que é superior a todos eles de forma geral. Django, Rails, Spring Boot,
Laravel, ASP.NET Core e outros têm anos de produção, comunidades, documentação,
extensões, suporte e casos reais que o Rullst ainda precisa conquistar. O
objetivo honesto é transformar diferenciais técnicos em uma plataforma estável,
auditada, mensurável e agradável de usar.

## Como ler o comparativo

Os veredictos usados abaixo têm significado restrito:

- **Diferencial comprovado:** existe código, teste e limite documentado no
  Rullst;
- **Vantagem de escopo:** o Rullst entrega mais componentes oficiais para esse
  caso, mas isso não prova maior qualidade em todos eles;
- **Paridade:** ambos resolvem o problema por caminhos diferentes;
- **Rullst atrás:** a alternativa tem hoje uma solução oficial mais madura ou
  uma experiência que o Rullst ainda não oferece;
- **Não comparável diretamente:** os projetos ocupam camadas diferentes.

As comparações consideram o **núcleo ou suíte oficial** de cada projeto. Um
plugin comunitário pode preencher várias lacunas, por isso este documento não
afirma que algo “não existe no ecossistema” sem evidência. Também não faz
afirmações de desempenho: velocidade só deve ser comparada com código, dataset,
hardware, percentis e metodologia reproduzíveis.

## Onde o Rullst já possui diferenciais reais

| Critério | Evidência atual no Rullst | Veredicto e limite |
|---|---|---|
| Suíte Rust coesa | Quinze pacotes publicáveis cobrem runtime, ORM, Auth, Security, AI, Capital, Mail, Connect, IoT, Nexus, Studio, macros e CLI. | **Vantagem de escopo** diante de bibliotecas HTTP e microframeworks. Mais superfície também aumenta a obrigação de manutenção e testes. |
| Segurança como contrato de composição | `ProductionPreset` expõe a ordem canônica; CSRF, headers, WAF/RASP, DLP, Login Jail, ownership/RBAC e telemetria possuem implementações delimitadas. | **Diferencial comprovado** frente a um simples conjunto de middlewares. Ainda falta consolidar completamente a fronteira entre Core e Security e obter auditoria externa. |
| Ferramentas privilegiadas fail-closed | Nexus exige política de autenticação; Studio requer uma capability local explícita, recusa release builds e valida loopback por request. | **Diferencial comprovado** de segurança por padrão. Não equivale a provar que todo handler está livre de vulnerabilidades. |
| Auditoria de acesso gerado | O scanner AST do CLI exige classificação adjacente para rotas parametrizadas e cobre acessos público, proprietário, role e admin. | **Diferencial comprovado** de engenharia preventiva. É uma barreira adicional, não um substituto para autorização em runtime e pentest. |
| IA first-party com limites | OpenAI, Gemini, Anthropic, DeepSeek e Ollama compartilham guardrails, mascaramento de PII, saídas estruturadas e fixtures offline determinísticas. | **Vantagem de escopo** sobre as suítes oficiais comparadas. Faltam avaliações públicas, políticas completas de tool calling e prova contra ataques adaptativos. |
| Desenvolvimento offline determinístico | Credenciais vazias ou `mock_*` selecionam caminhos sem rede em AI, Connect, Mail e Capital, com erros tipados para capacidades ausentes. | **Diferencial comprovado** de testabilidade. Mocks não validam contratos reais dos provedores. |
| Projetos gerados auditáveis | Seis blueprints, matriz estrutural de 270 combinações, projetos representativos e um gate que instala o CLI empacotado e compila offline cada blueprint verificam a saída distribuída. | **Diferencial comprovado** na base atual. A matriz inteira ainda não é compilada e a evidência precisa ser repetida no SHA final da RC. |
| Web, administração e Edge no mesmo projeto | A suíte combina backend web e uma fundação `no_std` com telemetria e verificação de manifesto OTA Ed25519. | **Vantagem de escopo**, não OTA completo: download, flashing, bootloader, MQTT, HSM e PQC reais continuam fora do contrato implementado. |
| Evidência de release multi-crate | Ordem topológica validada, preflight, empacotamento, SBOM e recuperação parcial estão documentados e parcialmente automatizados. | **Diferencial de governança em construção**. Só vira prova de release quando os gates rodarem verdes no SHA limpo da tag. |

## Frameworks Rust

### Axum

O [Axum](https://docs.rs/axum/latest/axum/) é uma biblioteca de routing e
request handling modular integrada ao ecossistema Tokio, Hyper e Tower. O
próprio Rullst usa essa base, portanto a relação é mais de **plataforma sobre
fundação** que de concorrência direta.

| O que Axum faz melhor | Vantagem atual do Rullst | Veredicto |
|---|---|---|
| API HTTP pequena, composição Tower direta, baixo acoplamento e escape hatch natural. | Convenções, ORM, auth, stack de segurança, jobs, email, AI, geradores, Nexus e Studio já integrados. | **Não comparável diretamente**. O Rullst deve preservar a interoperabilidade Axum, não tentar esconder ou substituir sua fundação. |

### Actix Web

O [Actix Web](https://actix.rs/docs/whatis/) se define como um framework web
poderoso e pragmático; sua documentação cobre composição de
[middleware](https://actix.rs/docs/middleware/) e serviços de produção.

| O que Actix faz melhor | Vantagem atual do Rullst | Veredicto |
|---|---|---|
| Núcleo web consolidado, foco claro e histórico maior de uso real. | Suíte oficial mais ampla, security/AI/admin/CLI integrados e convenções de aplicação. | **Vantagem de escopo**, não prova de maior desempenho ou maturidade. |

### Loco

O [Loco](https://loco.rs/docs/) é o concorrente Rust mais próximo: uma proposta
“Rails on Rust” com modelos, controllers, jobs, mailers, autenticação e CLI. Seus
[geradores](https://loco.rs/docs/reference/generators/) e o modelo uniforme de
[background jobs](https://loco.rs/docs/explanation/background-processing-model/)
são referências importantes.

| O que Loco faz melhor hoje | Vantagem atual do Rullst | Veredicto |
|---|---|---|
| História de produto mais concentrada, documentação de fluxo coesa, scaffolds CRUD/HTMX e workers com backends bem apresentados. | Segurança dedicada, IA multi-provider com guardrails, Capital, Nexus/Studio, auditoria IDOR e fundação IoT fazem parte da suíte oficial. | **Concorrente direto**. O Rullst possui maior amplitude; Loco é uma referência de foco e acabamento de DX. |

### Topcoat, do ecossistema Tokio

O [Topcoat](https://github.com/tokio-rs/topcoat) está no repositório oficial
`tokio-rs` e propõe um framework full-stack modular: expressões reativas escritas
em Rust são traduzidas para JavaScript, sem bundle WASM ou build separado de
cliente, junto a componentes server-rendered assíncronos. O próprio README o
classifica como **early-stage e experimental**, com breaking changes esperadas;
o workspace consultado declara a versão `0.5.0` em seu
[Cargo.toml](https://github.com/tokio-rs/topcoat/blob/main/Cargo.toml).

| O que Topcoat faz melhor hoje | Vantagem atual do Rullst | Veredicto |
|---|---|---|
| Direção mais inovadora para UI reativa server-first, componentes e envio seletivo de comportamento ao cliente. | Backend muito mais amplo: ORM, auth/security, AI, Capital, Mail, filas, Admin, Studio e geradores auditados. | **Projetos complementares e ainda em evolução**. Topcoat lidera a experimentação de UI; Rullst lidera o escopo de plataforma backend. Não integrar na RC enquanto sua API for experimental. |

### Poem e Salvo

[Poem](https://docs.rs/poem/latest/poem/) e
[Salvo](https://salvo.rs/guide/features/) oferecem superfícies HTTP extensas. O
Poem inclui middleware para CSRF, sessões, OpenTelemetry e outros casos; Salvo
documenta HTTP/3, OpenAPI, rate limiting, SSE, WebSocket e WebTransport.

| O que eles fazem melhor hoje | Vantagem atual do Rullst | Veredicto |
|---|---|---|
| Poem tem um catálogo HTTP/middleware concentrado; Salvo tem cobertura oficial mais ampla de protocolos e OpenAPI. | Rullst integra domínio de aplicação, segurança defensiva, IA, administração e scaffolding em vez de se limitar ao HTTP. | **Vantagem de escopo do Rullst**, mas **Rullst atrás** em OpenAPI completo e em protocolos como HTTP/3/WebTransport. |

### Leptos

O [Leptos](https://book.leptos.dev/getting_started/index.html) é um framework
full-stack de UI reativa em Rust, com SSR/CSR, hydration, server functions e
integrações Axum/Actix.

| O que Leptos faz melhor hoje | Vantagem atual do Rullst | Veredicto |
|---|---|---|
| Componentes reativos, hydration e uma experiência full-stack centrada na UI. | Modelo server-first simples com HTML/HTMX e uma suíte backend muito mais ampla. | **Não comparável diretamente**. Interoperabilidade é mais valiosa que duplicar um runtime reativo completo. |

### Dioxus

O [Dioxus 0.7](https://dioxuslabs.com/learn/0.7/) permite compartilhar Rust em
aplicações web, desktop e mobile. A suíte full-stack oferece SSR, hydration,
typed routing, hot reload e
[server functions compatíveis com Axum](https://dioxuslabs.com/learn/0.7/essentials/fullstack/server_functions/);
o [mobile](https://dioxuslabs.com/learn/0.7/guides/platforms/mobile/) é um alvo
first-class baseado em WebView, com renderização WGPU ainda experimental.

| O que Dioxus faz melhor hoje | Vantagem atual do Rullst | Veredicto |
|---|---|---|
| Componentes reativos e uma toolchain coerente para web, desktop, Android e iOS a partir do mesmo projeto. | Backend com ORM, segurança, identidade, AI, billing, mail, jobs, Nexus e Studio sob políticas comuns. | **Dioxus está à frente na camada de aplicação cross-platform; Rullst está à frente no backend especializado**. Uma integração oficial é mais realista e valiosa que recriar UI/mobile dentro do Core. |

## Frameworks de outras linguagens

### Django

O [Django 6.0](https://docs.djangoproject.com/en/6.0/) combina ORM, autenticação,
migrations, formulários e um
[admin model-centric](https://docs.djangoproject.com/en/6.0/ref/contrib/admin/).
Sua [documentação de segurança](https://docs.djangoproject.com/en/6.0/topics/security/)
cobre XSS, CSRF, SQL injection, clickjacking, CSP e segurança de deployment,
mas também deixa explícito, por exemplo, que throttling de autenticação não é
oferecido pelo núcleo.

| Onde Django é referência | Vantagem específica do Rullst | Veredicto |
|---|---|---|
| Maturidade, documentação, ORM/admin, i18n, ecossistema e experiência de produção. | Tipagem e ownership compilados, runtime defense first-party, ferramentas privilegiadas fail-closed, IA com guardrails e binário Rust. | **Rullst tem diferenciais de arquitetura**, mas está atrás em maturidade e não pode alegar segurança global superior. |

### Ruby on Rails

O [Rails 8.1](https://guides.rubyonrails.org/) continua sendo uma referência de
convenção e produtividade: Active Record, generators, Action Mailer, Hotwire,
deploy e [Active Job/Solid Queue](https://guides.rubyonrails.org/active_job_basics.html)
formam uma experiência coerente. O próprio
[guia de segurança](https://guides.rubyonrails.org/security.html) ressalta que
nenhum framework torna uma aplicação segura por si só.

| Onde Rails é referência | Vantagem específica do Rullst | Veredicto |
|---|---|---|
| Convenções lapidadas, velocidade para construir CRUD, ecossistema, material educacional e operação conhecida. | Segurança de memória e concorrência do Rust, contratos explícitos, security/AI first-party e análise estática dos artefatos gerados. | **Rullst pode superar em garantias compiladas e controles específicos**; Rails ainda é a meta de coesão e produtividade. |

### Spring Boot

O [Spring Boot](https://docs.spring.io/spring-boot/reference/) possui uma suíte
enterprise extensa, starters, integração com o ecossistema Java e
[Actuator](https://docs.spring.io/spring-boot/reference/actuator/endpoints.html).
Sua [auto-configuration](https://docs.spring.io/spring-boot/reference/using/auto-configuration.html)
é deliberadamente orientada por dependências e condições do classpath.

| Onde Spring Boot é referência | Vantagem específica do Rullst | Veredicto |
|---|---|---|
| Integrações enterprise, DI, mensageria, observabilidade, suporte comercial, tooling e grande base instalada. | Contratos menores e mais explícitos, static dispatch nas rotas comuns, ownership Rust e menos comportamento decidido por scanning/configuração em runtime. | **Rullst tem vantagem de explícito e compacto**, mas está muito atrás na plataforma enterprise. Spring AOT também reduz parte da diferença; não se deve caricaturá-lo como “apenas reflection”. |

### Laravel

O [Laravel 13](https://laravel.com/docs/13.x/) oferece ORM, container, auth,
policies, migrations, mail, notifications, events e
[queues](https://laravel.com/docs/13.x/queues), além de uma experiência de CLI e
starter kits muito refinada.

| Onde Laravel é referência | Vantagem específica do Rullst | Veredicto |
|---|---|---|
| Ergonomia, Artisan, Eloquent, filas, ecossistema SaaS, documentação e onboarding. | Garantias compiladas do Rust, stack de defesa dedicada, AI/provider mocks first-party e auditoria de acesso dos blueprints. | **Rullst tem diferenciais de segurança e tipagem**, mas Laravel é uma referência essencial de DX e ecossistema. |

### Gin

O [Gin](https://gin-gonic.com/en/docs/introduction/) é um framework HTTP Go
intencionalmente pequeno, com routing, middleware, binding/validation, rendering
e recovery.

| Onde Gin é referência | Vantagem específica do Rullst | Veredicto |
|---|---|---|
| Simplicidade, API concentrada e implantação Go conhecida. | Plataforma full-stack oficial com ORM, segurança, IA, admin, jobs, pagamentos, email e geradores. | **Vantagem de escopo do Rullst**, não vitória direta: usuários de Gin podem preferir justamente montar cada componente. |

### FastAPI

O [FastAPI](https://fastapi.tiangolo.com/features/) transforma type hints e
modelos em validação, JSON Schema, OpenAPI e documentação Swagger/ReDoc, com um
sistema de dependências bastante ergonômico.

| Onde FastAPI é referência | Vantagem específica do Rullst | Veredicto |
|---|---|---|
| Contrato de API/documentação automática, validação declarativa, DI e onboarding para APIs. | Binário e tipos Rust, stack full-stack mais ampla, controles de segurança operacionais e AI integrado. | **Rullst atrás em OpenAPI/SDK e ergonomia de contrato HTTP**; essa é uma prioridade competitiva, não algo a esconder. |

### ASP.NET Core

O [ASP.NET Core 10](https://learn.microsoft.com/en-us/aspnet/core/overview?view=aspnetcore-10.0)
reúne DI, configuração, logging, métricas, Minimal APIs, MVC, Blazor, SignalR,
gRPC, auth e data protection em uma plataforma madura.

| Onde ASP.NET Core é referência | Vantagem específica do Rullst | Veredicto |
|---|---|---|
| Tooling, diagnóstico, compatibilidade enterprise, protocolos, identidade e suporte de longo prazo. | Modelo de ownership do Rust, APIs mais explícitas e possibilidade de uma distribuição nativa menor e altamente especializada. | **ASP.NET Core está à frente como plataforma geral**. O Rullst deve competir por foco, segurança verificável e DX Rust, não por uma lista maior de features. |

## O que já pode ser afirmado publicamente

Afirmações sustentáveis:

- “Rullst é uma suíte full-stack Rust construída sobre Axum, Tokio e Tower.”
- “A suíte reúne primitivas first-party de segurança e IA com limites
  documentados.”
- “Nexus e Studio adotam acesso fail-closed; os blueprints passam por auditoria
  estrutural de autorização.”
- “Integrações externas possuem caminhos offline determinísticos para testes e
  desenvolvimento local.”
- “NFS-e ao vivo, OTA completo, HSM/PQC reais e SIEM operacional completo são
  roadmap, não capacidades de produção atuais.”

Afirmações que ainda não devem ser usadas:

- “o framework mais seguro”, sem auditoria independente e critério comparável;
- “o framework Rust mais rápido”, sem benchmark público reproduzível;
- “substitui Django, Rails, Spring ou Laravel em qualquer projeto”;
- “SOC/SIEM autônomo completo”, enquanto entrega durável, ingestão externa,
  retenção, correlação, casos e operação não estiverem implementados;
- “production-ready” como um único selo para toda a suíte.

## Recomendações para a v12 RC

A RC não deve tentar alcançar toda a ambição do framework. Ela deve provar que
o que existe é instalável, seguro por padrão, reproduzível e descrito com
honestidade. O [programa v12](v12.md) também contém itens da estável e do período
pós-RC; portanto, não é necessário marcar 100% daquele documento para publicar
uma RC.

### Bloqueadores reais

1. Congelar features e escolher o SHA exato da candidata.
2. Fazer o bump atômico para `12.0.0-rc.1`, empacotar os 16 crates e testar
   consumidores usando somente os pacotes empacotados/crates.io.
3. Rodar formato, Clippy e testes all-features no SHA; obter CI Linux, macOS e
   Windows verde no mesmo commit.
4. Compilar projetos materializados representativos de todos os seis blueprints
   em CI e provar em release que Studio não é exposto e Nexus falha fechado.
5. Cumprir os gates já escolhidos: bibliotecas em pelo menos 80% de line
   coverage para a RC, patch coverage em pelo menos 90%, Auth e Security em pelo
   menos 90%, sem caminho crítico em 0% e casos negativos do threat model
   cobertos.
6. Publicar matriz de features, MSRV, políticas de SemVer/depreciação/suporte,
   guias de migração e changelog compatível com a implementação real.
7. Executar auditoria de dependências/licenças, validar SBOM/proveniência e fazer
   uma revisão manual focada em Auth, Nexus, Studio, webhooks e configuração de
   produção.
8. Registrar decisão GO/NO-GO, responsáveis, SHA e evidências antes do primeiro
   upload irreversível.

### Melhorias competitivas permitidas antes da RC

Somente mudanças pequenas e redutoras de risco:

- corrigir diagnósticos e mensagens de configuração dos projetos gerados;
- completar testes negativos já previstos no threat model;
- garantir estados “indisponível” honestos em Studio/Nexus;
- congelar e documentar o schema de `SecurityEvent`, sem prometer SIEM durável;
- corrigir exemplos, links, feature flags e instalação offline/reproduzível.

Topcoat, novos protocolos, conectores enterprise, uma nova UI reativa, NFS-e ao
vivo, hardware IoT e grandes reformulações de Auth **não pertencem à RC**. Cada
uma aumenta a superfície justamente quando a prioridade deve ser estabilizar.

## Recomendações para v12.1

Uma versão 12.1 deve privilegiar melhorias aditivas e compatíveis:

1. **OpenAPI e SDK:** gerar contrato tipado a partir das rotas/extractors,
   validar breaking changes e produzir clientes testados. FastAPI, Salvo e o
   ecossistema Spring mostram por que isso é decisivo.
2. **Matriz gerada completa:** transformar as 270 verificações estruturais em
   compilação incremental/particionada, com smoke E2E por blueprint.
3. **Auth operacional:** concluir política JWT de aplicação, rotação/revogação,
   sessão por dispositivo e TOTP sem inventar criptografia própria.
4. **Rate limit e idempotência distribuídos:** contratos Redis/SQL com testes de
   concorrência, falha e múltiplas instâncias.
5. **Security Event Sink:** API aditiva, redaction, backpressure, retry,
   dead-letter/spool e primeiro transporte padrão (preferencialmente OTLP ou
   HTTP assinado). Adaptadores CEF/syslog podem ser crates opcionais.
6. **AI evals e tool safety:** matriz por provider, datasets versionados,
   autorização por ferramenta, limites de saída e proteção SSRF/egress.
7. **Storage remoto:** adapters opcionais S3/R2 com testes de compatibilidade,
   multipart, retries e isolamento tenant-aware.
8. **DX mensurável:** medir tempo até primeiro CRUD seguro, qualidade dos erros,
   rebuild incremental e número de passos até deploy local.

## Recomendações para v13

A v13 pode receber mudanças arquiteturais que não cabem numa minor:

1. Consolidar o contrato de segurança entre Core e `rullst-security`, mantendo
   uma única ordem e tipos compartilhados sem ciclo de dependências.
2. Definir uma arquitetura de extensões first-party e comunitárias com
   compatibilidade, ownership, manutenção e conformance suites.
3. Evoluir a fundação delimitada de `rullst-messaging` com codec estável,
   persistência e adapters Kafka, RabbitMQ, NATS/JetStream e Redis Streams,
   em vez de misturar mensageria com OAuth em Connect.
4. Evoluir SOC/SIEM como produto verificável: ingestão multi-source, regras,
   correlação, retenção, casos, evidências, RBAC, auditoria e intervenção humana.
   Conectores de fornecedores devem permanecer opcionais.
5. Concluir WebAuthn por biblioteca auditada e testes de conformidade; avaliar
   auditoria externa formal da superfície Auth/Security.
6. Criar programas separados para NFS-e homologada e IoT/OTA real, com
   mantenedores, hardware/ambientes, fault injection e critérios jurídicos ou
   operacionais próprios.
7. Investigar interoperabilidade com Topcoat quando sua API estabilizar. O
   Rullst deve escolher conscientemente entre integração e uma camada reativa
   própria, em vez de copiar uma experiência experimental durante a v12.
8. Publicar benchmarks independentes e reproduzíveis de workloads completos:
   CRUD/ORM, auth, templates, filas, WebSocket e middleware de segurança.

## Critérios para se tornar o principal framework Rust

“Melhor” precisa ser convertido em resultados observáveis:

| Dimensão | Evidência necessária |
|---|---|
| Correção | Zero P0 conhecido, CI multi-OS consistente, testes negativos e recuperação de falhas. |
| Segurança | Threat models versionados, disclosure responsável, dependências governadas, auditoria externa e correções com SLA. |
| Estabilidade | SemVer previsível, MSRV, política de suporte, migrações e janela real de RC. |
| DX | Projeto inicial e CRUD seguro rápidos, erros acionáveis, documentação verificável e escape hatch Axum/SQLx. |
| Desempenho | Benchmarks públicos com metodologia, percentis, consumo de memória e regressão em CI. |
| Ecossistema | Extensões mantidas, conformance tests, exemplos reais e integração com serviços usados em produção. |
| Adoção | Aplicações independentes, feedback de mantenedores externos, contribuidores e casos públicos de operação. |
| Operação | Telemetria interoperável, runbooks, backup/restore, upgrades e incident response testados. |

## Critérios para se tornar o principal framework de todos

Competir também com Django, Rails, Spring Boot, Laravel, FastAPI e ASP.NET Core
exige algo mais difícil que acumular features: o Rullst precisa oferecer um
**golden path completo e seguro**, sem impedir que especialistas substituam cada
camada. As ideias abaixo são realistas quando entregues por etapas; várias já
existem nos roadmaps, mas algumas ainda estavam marcadas de forma mais otimista
que a implementação.

| Implementação sugerida | Origem preservada | Entrega realista e critério de sucesso | Janela |
|---|---|---|---|
| Contrato de API tipado e SDKs | M5, M29 e M34 do roadmap | Um schema canônico gera OpenAPI e clientes TypeScript/Dart/Swift; golden tests provam serialization e CI detecta breaking changes. | v12.1–v13 |
| Perfil oficial Rullst + Dioxus | M14, M16 e visão Omni | Template opcional para web/desktop/mobile, auth compartilhada, client gerado e um app Android E2E. Rullst permanece dono do backend; Dioxus, da UI. | v12.1 experimental; v13 suportado |
| Auth de referência completa | M9 e roadmaps Auth/Security | JWT com issuer/audience/rotação/revogação, sessões por dispositivo, TOTP/recovery e WebAuthn por biblioteca auditada/conformance suite. | v12.1–v13 |
| Multi-tenancy e entitlements seguros | M11 e M33 | Tenant derivado de identidade autenticada, filtros SQL verificáveis, gates server-side, auditoria e negativos cross-tenant em todos os blueprints SaaS. | v12.1 |
| Idempotência e limites distribuídos | M10 e roadmap Capital | A quota de recursos do Capital já possui reserva SQL atômica/idempotente por tenant em quatro protocolos. Ainda faltam stores compartilhados uniformes, expiração/reconciliação e cobertura multi-instância para login, APIs e webhooks. | Capital delimitado na v12; demais em v12.1 |
| Observabilidade que explica o problema | M19, M35 e roadmap Studio | OTLP interoperável, trace waterfall, profiling SQL/N+1, jobs/cache e estado indisponível honesto; nenhum dado inventado. | v12.1–v13 |
| SOC/SIEM operacional, não cenográfico | M12 e roadmap Security | Eventos versionados, redaction, spool/retry/dead-letter, ingestão e correlação, casos, retenção, RBAC e conectores opcionais testados. | v12.1 fundação; v13 produto |
| AI segura e avaliável | Roadmap AI e M19/M36/M37 | Evals versionados, capability matrix por provider, tool authorization, egress/SSRF guard, budgets, aprovação humana e rollback de mudanças. | v12.1–v13 |
| Mensageria por contrato | M15 e roadmap Messaging | A crate separada já possui envelope, limites, idempotência, grupos, leases, retry/DLQ, broker determinístico e conformance suite; adapters Kafka/RabbitMQ/NATS/Redis Streams entram somente quando seus semantics forem comprovados. | Fundação v12; remotos v13 |
| Storage e media isolados | M17 | S3/R2 com assinatura oficial, multipart/retry, limites de path/pixels, mocks determinísticos e fuzzing de codecs em crate opcional. | v12.1+ |
| Extensões sustentáveis | M13 e arquitetura de packages | Manifesto, capability permissions, compatibilidade SemVer, ownership e testes; sandbox Wasm apenas após limites reais de CPU/memória/I/O. | v13 |
| Deploy e upgrades recuperáveis | M26 e M27 | Health/readiness, migrations coordenadas, secrets, canary/rollback e runbook testados; “one click” descreve automação guiada, não disponibilidade garantida. | v12.1–v13 |
| Performance demonstrada | M2 e benchmarks do ORM | Repositório público de workloads, hardware fixado, throughput, p50/p95/p99, memória e regressões; comparar aplicações completas, não uma função isolada. | contínuo |
| Confiança externa | Programa v12 e governança | RC pública, auditoria independente, política de suporte, mantenedores externos, apps reais e divulgação coordenada de vulnerabilidades. | começa na v12 RC |

Três regras impedem essa ambição de destruir o projeto:

1. O Core continua pequeno; capacidades grandes entram em crates/perfis
   opcionais com conformance suites.
2. Nenhuma integração vira “implementada” apenas por compilar ou possuir um
   adapter nominal: precisa de teste de contrato e falha segura.
3. NFS-e, hardware IoT, HSM/PQC e um SOC hospedado são programas próprios, com
   mantenedores e infraestrutura, mesmo quando usam Rullst como plataforma.

O caminho mais forte para o Rullst não é vencer uma competição de quantidade de
features. É combinar a produtividade que Rails/Laravel/Loco ensinaram, a
previsibilidade operacional de Spring/ASP.NET Core, a ergonomia de contrato do
FastAPI, a experiência cross-platform do Dioxus, a composição de Axum/Tower e
as garantias de Rust — sem anunciar como pronto o que ainda é visão.

## Fontes externas consultadas

Somente documentação ou repositórios oficiais foram usados nesta fotografia:

- Rust: [Axum](https://docs.rs/axum/latest/axum/),
  [Actix Web](https://actix.rs/docs/), [Loco](https://loco.rs/docs/),
  [Topcoat](https://github.com/tokio-rs/topcoat),
  [Poem](https://docs.rs/poem/latest/poem/),
  [Salvo](https://salvo.rs/guide/features/) e
  [Leptos](https://book.leptos.dev/getting_started/index.html),
  [Dioxus](https://dioxuslabs.com/learn/0.7/);
- outras linguagens: [Django 6.0](https://docs.djangoproject.com/en/6.0/),
  [Rails 8.1](https://guides.rubyonrails.org/),
  [Spring Boot](https://docs.spring.io/spring-boot/reference/),
  [Laravel 13](https://laravel.com/docs/13.x/),
  [Gin](https://gin-gonic.com/en/docs/),
  [FastAPI](https://fastapi.tiangolo.com/) e
  [ASP.NET Core 10](https://learn.microsoft.com/en-us/aspnet/core/?view=aspnetcore-10.0).

Essas fontes e versões mudam. A comparação deve ser revalidada antes de virar
material de lançamento e toda alegação de desempenho deve viver em um benchmark
versionado, não nesta página.
