# Rullst Roadmap 🗺️
### *"O Caminho para o Framework Full-Stack de Rust Definitivo"*

*Read this in [English](./ROADMAP.md).*

Este roadmap descreve os marcos necessários para transformar o **Rullst** da sua versão MVP atual (v0.1.0) em um framework full-stack dominante, pronto para produção, focado em **Produtividade Emocional** e **Engenharia Nativa para IA**.

Nossa estratégia de desenvolvimento segue a filosofia **"Developer Experience de Laravel, Performance de Rust, Arquitetado para Humanos e IA"**.

---

## 🤖 O Paradigma Nativo para IA (Projetado para Humanos e Agentes de IA)

Quase todos os frameworks web modernos (Laravel, Ruby on Rails, Next.js) foram criados antes da era dos LLMs e Agentes de IA. Eles dependem fortemente de "mágica" em tempo de execução, reflexão dinâmica e implicitude complexa que confunde os codificadores de IA e geram alucinações.

**O Rullst foi projetado desde o primeiro dia para ser o primeiro framework web nativo para IA:**
1. **Zero Mágica em Runtime, Compilação Pura:** Macros declarativas de alto nível (`#[derive(Eloquent)]`, `routes!`) e a tipagem forte do Rust oferecem estruturas extremamente explícitas para assistentes de IA, eliminando alucinações de API e permitindo que a IA se autocorrija instantaneamente com as mensagens de erro do compilador.
2. **Scaffolding Rico em Contexto:** O comando `cargo rullst new` irá gerar automaticamente arquivos `.ai-rules` / `.cursorrules` otimizados. Qualquer agente de IA que abrir a pasta entenderá imediatamente as convenções exatas, estilo de código e APIs do Rullst, atingindo 100% de produtividade na hora.
3. **Descoberta Estruturada do Sistema:** Em modo de desenvolvimento, o Rullst gerará um esquema estrutural local (`rullst-schema.json`) detalhando todas as rotas, controllers e models ativos. Isso permite que agentes de IA mapeiem e entendam o projeto inteiro em milissegundos.

---

## 🚀 O Plano Diretor do Rullst

```mermaid
graph TD
    M0["🤖 Pilar: Design Nativo para IA"] --> M1["🛠️ Marco 1: CLI e Geradores"]
    M1 --> M2["🗄️ Marco 2: Supremacia do DB"]
    M2 --> M3["🔒 Marco 3: Auth & Segurança"]
    M3 --> M4["⚡ Marco 4: HTMX & Frontend"]
    M4 --> M5["📦 Marco 5: Utilitários de Produção"]
    M5 --> M6["🏢 Marco 6: Recursos Enterprise"]
    M6 --> M7["🚀 Marco 7: A Vantagem Injusta"]
    M7 --> M8["🌍 Marco 8: Edge Fusion & Auto-Upgrade"]
    M8 --> M9["🤖 Marco 9: DevOps Agêntico"]
    M9 --> M10["📊 Marco 10: Telemetria & Pulse"]
    M10 --> M11["🔮 Marco 11: Omni-Frontend & IA"]
    M11 --> M12["💎 Marco 12: Zero-Copy Streaming"]
    M12 --> M13["🛠️ Marco 13: Compilação Incremental"]
    M13 --> M14["🌐 Marco 14: DB Baseado em Intenção"]
    M14 --> M15["🔬 Marco 15: Pronto para Quantum"]
    M15 --> M16["🧬 Marco 16: Núcleo Auto-Evolutivo"]

    style M0  fill:#ffecd2,stroke:#ff9a00,stroke-width:3px,color:#000
    style M1  fill:#00f2fe,stroke:#fff,stroke-width:2px,color:#000
    style M2  fill:#4facfe,stroke:#fff,stroke-width:2px,color:#000
    style M3  fill:#a18cd1,stroke:#fff,stroke-width:2px,color:#000
    style M4  fill:#fbc2eb,stroke:#fff,stroke-width:2px,color:#000
    style M5  fill:#ff9a9e,stroke:#fff,stroke-width:2px,color:#000
    style M6  fill:#b5ffd9,stroke:#fff,stroke-width:2px,color:#000
    style M7  fill:#ffe57f,stroke:#fff,stroke-width:2px,color:#000
    style M8  fill:#e1bee7,stroke:#fff,stroke-width:2px,color:#000
    style M9  fill:#b3e5fc,stroke:#fff,stroke-width:2px,color:#000
    style M10 fill:#ffccbc,stroke:#fff,stroke-width:2px,color:#000
    style M11 fill:#c8e6c9,stroke:#fff,stroke-width:2px,color:#000
    style M12 fill:#f8bbd0,stroke:#fff,stroke-width:2px,color:#000
    style M13 fill:#dcedc8,stroke:#fff,stroke-width:2px,color:#000
    style M14 fill:#fff9c4,stroke:#fff,stroke-width:2px,color:#000
    style M15 fill:#b2ebf2,stroke:#fff,stroke-width:2px,color:#000
    style M16 fill:#a5d6a7,stroke:#fff,stroke-width:3px,color:#000
```

---

## 🛠️ Marco 1: Poder do CLI (`cargo-rullst`)
**Objetivo:** Permitir scaffold e geração de código em segundos. Desenvolvedores não devem criar arquivos de boilerplate manualmente.

- [x] **Geradores de Código:**
  - [x] `cargo rullst make:controller <Nome>` - Gera um controller com as ações básicas de CRUD.
  - [x] `cargo rullst make:model <Nome> [-m]` - Gera um model de Active Record e, opcionalmente, uma migration associada.
  - [x] `cargo rullst make:middleware <Nome>` - Gera um middleware customizado compatível com Axum.
  - [x] `cargo rullst make:cors` & `make:jwt` - Gera middlewares essenciais em Rust puro direto no seu projeto.
  - [x] `cargo rullst generate:openapi` - Geração de OpenAPI/Swagger guiada por IA, sem poluir o código com macros.
  - [x] `cargo rullst make:worker` - Scaffold para workers de background tasks.
- [x] **Ergonomia do Workspace:**
  - [x] Melhorar a velocidade de compilação durante as execuções do CLI.
  - [x] Suporte à flag `--api` para criar scaffolds de APIs REST sem frontend HTML.

---

## 🗄️ Marco 2: Supremacia do Banco de Dados (Migrations & Relacionamentos)
**Objetivo:** Capacitar o `rust-eloquent` e o `Rullst` a gerenciar esquemas relacionais complexos sem complicação.

> [!NOTE]
> **Divisão de Responsabilidades:**
> O trabalho pesado (parsers de esquema SQL, execução de migrations e macros de relacionamento) será desenvolvido diretamente dentro do repositório **`rust-eloquent`** para manter o ORM modular e atraente para toda a comunidade Rust.
> O **Rullst** irá envelopar essas funcionalidades com comandos amigáveis de CLI e injeção automática de conexões.

- [x] **Motor de Migrations (no `rust-eloquent`):**
  - [x] Definição de migrations em SQL puro ou através de DSL intuitiva.
  - [x] Executores CLI integrados no Rullst:
    - [x] `cargo rullst db:migrate` - Executa migrations pendentes.
    - [x] `cargo rullst db:rollback` - Reverte o último lote de migrations.
    - [x] `cargo rullst db:status` - Mostra o histórico de migrações aplicadas.
- [x] **Relacionamentos Active Record (no `rust-eloquent`):**
  - [x] Macros declarativas de relacionamento como `HasMany` e `BelongsTo`.
  - [x] Resolução de associações `BelongsToMany` (Muitos para Muitos).
  - [x] Mecanismos de Lazy e Eager loading para evitar problemas de consultas N+1.
- [x] **Seeders e Factories:**
  - [x] `cargo rullst db:seed` - Popula o banco de dados usando dados fakes pré-configurados.
  - [x] Padrão de factories integrado para geração ágil de entidades de teste.

---

## 🔒 Marco 3: Autenticação & Segurança (Social, Local & Passkeys)
**Objetivo:** Implementar autenticação robusta, segura e instantânea. Qualquer dev deve ser capaz de autenticar usuários de forma segura em minutos.

- [x] **Autenticação Social via `rust-socialite`:**
  - [x] Integrar a crate **[`rust-socialite`](https://crates.io/crates/rust-socialite)** (sua criação!) como o motor oficial de OAuth do framework.
  - [x] Configurações out-of-the-box para Google, GitHub, Facebook, Twitter e provedores genéricos de OpenID.
  - [x] Fluxo fluido: redirecionar para o provedor, tratar o callback e autenticar/registrar o usuário via Active Record.
- [x] **Autenticação Local:**
  - [x] Auxiliares embutidos para hashing seguro de senhas via Argon2/Bcrypt.
  - [x] Middlewares customizados para sessões seguras baseadas em Cookies e Tokens (JWT).
- [ ] **Passkeys & Biometrics First (`rullst::auth::passkey`):** Abstração nativa de WebAuthn para autenticação biométrica (FaceID, TouchID, Windows Hello) integrada ao `cargo rullst auth`. Cadastros e logins sem senha (Passwordless) usando criptografia de chave pública via HTMX/WebAuthn, com fallback fluído para chaves de segurança físicas.
- [x] **O Comando "Mágico" de Auth:**
  - [x] `cargo rullst auth` - Cria instantaneamente um sistema completo de login e registro contendo:
    - Controllers de Login, Registro e Reset de Senha.
    - Telas bonitas e responsivas (templates `html!`) pré-estilizadas.
    - Migration SQL para a tabela de `users`.
- [x] **Padrões de Segurança Robustos:**
  - [x] Proteção automática contra ataques CSRF em submissões de formulários HTML.
  - [x] Middleware padrão de cabeçalhos de segurança (CORS, HSTS, X-Content-Type-Options).

---

## ⚡ Marco 4: HTMX & Interatividade
**Objetivo:** Combinar a simplicidade de Server-Side Rendering (SSR) com a fluidez de Single-Page Applications (SPAs).

- [x] **Suporte de Primeira Classe ao HTMX:**
  - [x] Helpers para verificar cabeçalhos de requisição do HTMX (`rullst::htmx::is_htmx(req)`).
  - [x] Suporte nativo para renderização de templates parciais (renderizar apenas o componente modificado, sem carregar o layout inteiro).
  - [x] Integração nativa e configuração automática do TailwindCSS na inicialização do projeto.

---

## 📦 Marco 5: Utilitários de Produção (Filas, Caching, Scheduler & Assets)
**Objetivo:** Fornecer os recursos fundamentais necessários para escalar aplicações reais em produção.

- [x] **Docker e Containerização:**
  - [x] Flag `cargo rullst new <nome> --docker` para gerar um `Dockerfile` pronto para produção.
  - [x] Geração automática de `docker-compose.yml` para desenvolvimento local (App + DB + Redis).
  - [x] Builds multi-stage otimizados (`scratch` / `distroless`) para deploys em Rust ultra-leves, rápidos e seguros.
- [x] **Filas & Tarefas em Segundo Plano:**
  - [x] API unificada `rullst::queue` com suporte a SQLite (para dev local) e Redis (para produção).
  - [x] Executores assíncronos (workers) rodando tarefas pesadas em background.
- [x] **Camada de Cache:**
  - [x] API unificada `rullst::cache` com drivers para In-Memory e Redis.
- [x] **Agendador de Tarefas (Task Scheduler):**
  - [x] Agendamento declarativo tipo Cron diretamente no `main.rs` (ex: `.schedule("0 0 * * *", limpeza_diaria)`).
- [ ] **Edge-Optimized Assets & Compression Tuning:** Durante o build de produção (`cargo rullst build --release`), o framework pré-comprime assets estáticos usando **Brotli (nível 11)** e **Zstandard**. Arquivos estáticos são servidos pelo Axum via chamadas `sendfile` (Zero-Copy direto pelo Kernel), superando a velocidade do Nginx puro.

---

## 🏢 Marco 6: Funcionalidades Enterprise (Validação, E-mail, Storage & Proteção)
**Objetivo:** Entregar os recursos robustos clássicos esperados de frameworks focados em empresas.

- [x] **Validação Declarativa:** Uma macro `#[derive(Validate)]` para DTOs/structs que retorna automaticamente JSON 422 para APIs ou componentes HTMX com erros para formulários.
- [x] **Sistema de E-mail (`rullst::mail`):** API fluente para envio de e-mails com drivers para SMTP, Resend e SendGrid, suportando templates nativos com a macro `html!`.
- [x] **Abstração de Armazenamento (`rullst::storage`):** API unificada para uploads e gerenciamento de arquivos com drivers para Local (Disco), AWS S3 e Cloudflare R2.
- [x] **WebSockets & Tempo Real:** Suporte nativo a WebSockets no roteador, perfeitamente integrado com a extensão HTMX (`hx-ext="ws"`).
- [x] **Rullst Horizon:** Um dashboard web embutido lindíssimo para monitorar filas, visualizar jobs que falharam e tentar executá-los novamente.
- [ ] **Adaptive Backpressure & Resilient Traffic Shielding:** Middleware de proteção no roteador que monitora as threads assíncronas do Tokio e os tempos de resposta do banco de dados. Caso o sistema atinja saturação iminente, ele graciosamente rejeita (graceful degradation) ou enfileira requisições excedentes, evitando crashes por falta de memória (OOM).
- [ ] **Token-Bucket Rate Limiting Nativo:** Configuração declarativa de limites de taxa (ex: `#[route(get, "/api", rate_limit = "100/m")]`) com suporte a armazenamento distribuído via Redis ou memória compartilhada (`DashMap`).

---

## 🚀 Marco 7: A "Vantagem Injusta" (Domínio Absoluto)
**Objetivo:** Ir além do que é possível em outras linguagens, tornando o Rullst o rei inquestionável do desenvolvimento web moderno.

- [x] **Rullst Live (Server-Driven UI):** Inspirado no Phoenix LiveView e Laravel Livewire. Escreva componentes Rust com estado que sincronizam automaticamente com o navegador via WebSockets. Interatividade de SPA sem escrever uma única linha de JavaScript.
- [x] **Core IA Nativo (`rullst::ai`):** Abstrações declarativas embutidas para LLMs (OpenAI, Gemini, Anthropic, Ollama), Bancos de Dados Vetoriais e Agentes IA. Crie aplicações com RAG em minutos.
- [x] **Rullst Studio:** Uma interface visual nativa para inspecionar, filtrar e editar os registros do seu banco de dados localmente (estilo Prisma Studio). Ativado via `cargo rullst studio`.
- [x] **Testes E2E Declarativos:** API fluente de testes no estilo Laravel: `app.get("/login").assert_status(200).assert_see("Bem-vindo");`.
- [x] **Feature Flags Nativas:** Suporte embutido para ligar/desligar funcionalidades e realizar Testes A/B sem dependências externas.
- [x] **Wasm Islands (`#[client_component]`):** Escreva componentes frontend interativos diretamente em Rust. O Rullst compilará automaticamente esses blocos específicos para WebAssembly leve e os hidratará no cliente de forma transparente, eliminando a necessidade de qualquer linha de JavaScript!
- [x] **Console de Erros "Self-Healing" com IA:** Tela interativa de erro em modo desenvolvimento (estilo Laravel Ignition) integrada a assistentes locais de IA. Quando um erro em runtime ou compilação acontecer, você terá um botão "Auto-Fix com Rullst AI" que aplicará o patch correto diretamente ao seu código-fonte.
- [x] **SaaS Multi-Tenancy Nativo (`rullst::multitenant`):** Isolamento nativo de inquilinos (Multi-tenancy por subdomínio, cabeçalho ou esquema de DB) configurado declarativamente por meio de um único decorator/macro.
- [x] **Hot Reloading via Dynamic Linking:** Redução drástica dos tempos de compilação em desenvolvimento por meio do carregamento dinâmico de bibliotecas (`dylib` / `.so`), permitindo alterar rotas e templates HTML com feedback instantâneo de sub-segundos.

---

## 🌍 Marco 8: Distribuição de Dados e Fusão com a Borda (Edge Fusion)
**Objetivo:** Rodar o Rullst em infraestrutura Edge moderna sem reescrever código e com latência ultra-baixa globalmente.

- [ ] **Rullst Edge Runtime (`rullst::edge`):** Suporte nativo para compilar e rodar aplicações Rullst em infraestrutura WebAssembly (Cloudflare Workers, Fastly Compute, AWS Lambda@Edge) abstraindo as diferenças de Tokio/WASI.
- [ ] **Replicação SQLite Zero-Config:** Drivers nativos para bancos de dados distribuídos na borda (Turso/libsql, Cloudflare D1) integrados ao `rust-eloquent`. Leia e grave localmente com 1ms de latência enquanto o framework sincroniza globalmente em background.

### 🔄 Sistema de Atualização Autônoma (`cargo rullst upgrade`)

> Um dos maiores desafios de DX em qualquer framework full-stack é manter as dependências atualizadas sem quebrar o código do usuário. No ecossistema Rust, isso é ainda mais crítico — mudanças de versão mesmo minor/patch em crates de baixo nível podem causar erros rígidos de compilação. Este sistema torna as atualizações do Rullst praticamente invisíveis.

- [ ] **Verificação de Versão em Background (Não-Intrusiva):** Toda vez que o usuário rodar comandos frequentes como `cargo rullst new` ou `cargo rullst dev`, a CLI realiza uma requisição HTTP leve e assíncrona (numa thread separada, nunca bloqueando o terminal) para a API pública do Crates.io (`https://crates.io/api/v1/crates/rullst`) e compara a versão mais recente com a versão fixada no `Cargo.toml` do usuário. O resultado é salvo em um arquivo temporário local e renovado no máximo uma vez por dia, garantindo zero overhead de rede em execuções repetidas.

- [ ] **Box Visual Elegante no Terminal:** Se uma nova versão for detectada, a CLI exibe um box informativo não-bloqueante direto no terminal — estilizado com `colored` — imediatamente após o comando ser concluído:
  ```
  ┌────────────────────────────────────────────────────────────┐
  │  🚀 Nova versão do Rullst disponível: 1.0.5 → 1.1.0        │
  │  Rode 'cargo rullst upgrade' para atualizar com segurança  │
  │  e correções automáticas de código (codemods).             │
  └────────────────────────────────────────────────────────────┘
  ```

- [ ] **Codemods Automatizados (Atualizações Sem Quebras):** Expandir o `cargo rullst upgrade` em um pipeline autônomo completo de refatoração:
  1. **Atualização do manifest:** A CLI reescreve as strings de versão do `rullst`, `rullst-macros` e `rust-eloquent` no `Cargo.toml` do usuário para o release mais recente.
  2. **Execução de codemods:** Um registro versionado de regras de busca-e-substituição baseadas em regex (ou AST leve) é distribuído junto de cada release da CLI. Se uma API pública mudou entre versões (ex: `.render()` renomeado para `.render_page()`), a CLI reescreve automaticamente todas as ocorrências em `src/**/*.rs` antes que o usuário veja qualquer erro de compilação.
  3. **Portão de validação (`cargo check`):** Após aplicar os codemods, a CLI roda `cargo check` em background. Se o compilador sair limpo, o usuário vê: `"✅ Rullst atualizado com sucesso. Nenhuma quebra detectada."` Se restarem erros, um diff das alterações dos codemods é exibido para revisão do dev.

- [ ] **Blindagem de Dependências (Casca de Abstração Interna):** Enforçar a regra arquitetural de que todas as dependências pesadas de terceiros (ex: `sqlx`, `axum`, `tokio`, `lettre`) são sempre re-exportadas ou encapsuladas dentro da superfície de API pública do Rullst. O código do usuário nunca deve usar `use sqlx::*` diretamente — apenas `use rullst::db::*`. Isso blinda os apps dos usuários contra quebras upstream: quando o `sqlx` lança uma versão breaking, apenas o adaptador interno do Rullst muda, e o código do usuário continua compilando intacto.



## 🤖 Marco 9: DevOps Agêntico e Infraestrutura Autônoma
**Objetivo:** Alavancar o conhecimento profundo que o compilador do Rullst tem sobre o projeto para gerenciar não apenas o código, mas a infraestrutura e o CI/CD.

- [ ] **Provisionamento Autônomo (`cargo rullst deploy --autonomous`):** O compilador analisa os recursos utilizados no código (ex: usa `rullst::storage::S3`, cria um bucket) e conversa com os provedores de nuvem para provisionar a infraestrutura exata, eliminando arquivos complexos de Terraform.
- [ ] **Análise de Gargalos em CI/CD com IA:** Uma esteira de testes que usa LLMs locais para avaliar regressões de performance. Se um commit deixar uma rota mais lenta, a IA analisa o profiling da stack do Tokio e sugere a linha exata que está causando o gargalo.

---

## 📊 Marco 10: Telemetria de Hardware e Pulse
**Objetivo:** Tornar o debug assíncrono e o profiling de performance fluidos, sem depender de setups externos complexos.

- [ ] **Rullst Pulse (Telemetria a Nível de Kernel):** Dashboard visual em tempo real para métricas de hardware/software. Detecte gargalos de CPU, contenção de Mutex, memory leaks e gargalos de I/O com zero overhead.
- [ ] **Time-Travel Debugging no Console de Erros:** Adição de um histórico com os últimos 50 eventos, cliques (HTMX) e queries SQL à tela de "Self-Healing". Permite dar "replay" no cenário exato que causou o "panic" no servidor.
- [ ] **OpenTelemetry Nativo:** Abstração *zero-config* para exportar traces e logs para Datadog, Grafana Loki ou Prometheus.

---

## 🔮 Marco 11: Protocolo Omni-Frontend e Expansão de IA
**Objetivo:** Consolidar o Rullst como o backend supremo para agentes de IA, SPAs tradicionais e Aplicativos Mobile Nativos.

- [ ] **Geração Automática de SDK TypeScript:** Para rotas exportadas via REST/JSON ou WebSockets, gerar automaticamente um cliente TS 100% tipado, eliminando ferramentas como tRPC ou OpenAPI manuais.
- [ ] **Hyper-Media Mobile Bridge:** Protocolo que permite aplicativos mobile híbridos (iOS/Android) consumirem as respostas parciais (HTMX/JSON) do Rullst para renderizar telas nativas instantaneamente (Server-Driven UI para mobile).
- [ ] **AI Agent Tool-Calling:** Expor rotas e controllers automaticamente como "Tools" executáveis para LLMs externos com esquemas gerados nativamente (`rullst-schema.json`).
- [ ] **Injeção Dinâmica de Contexto:** Endpoint seguro `/_rullst/ai-context` que fornece documentação da API em tempo real para agentes de integração.
- [ ] **DB Seeding com IA:** `cargo rullst db:seed --ai` usa modelos locais (ex: Ollama) para gerar dados falsos ultra-realistas e contextualizados.

---

## 💎 Marco 12: Zero-Copy Event Streaming e Arquitetura Time-Travel
**Objetivo:** Unificar nativamente o ciclo de vida dos dados e eliminar a necessidade de infraestruturas pesadas de mensageria de terceiros.

- [ ] **Rullst Ledger (`rullst::ledger`):** Um motor de Event Sourcing integrado diretamente no `rust-eloquent`. Em vez de apenas salvar o estado atual no banco, o framework grava o histórico imutável de eventos por padrão usando persistência Zero-Copy (memória mapeada em disco / mmap).
- [ ] **Built-in Event Streaming:** O próprio binário do Rullst atua como um micro-broker de mensageria assíncrona distribuída entre diferentes instâncias do app via WebSockets/QUIC, eliminando a dependência obrigatória de Kafka ou RabbitMQ.

---

## 🛠️ Marco 13: Compilação Incremental Instantânea e Linker Hacking
**Objetivo:** Erradicar o atrito de tempo de compilação em projetos massivos de Rust, atingindo velocidade de resposta de linguagens interpretadas.

- [ ] **Integração Profunda Mold/Cranelift:** Configurar o scaffolding do framework para forçar o uso de linkers ultra-rápidos (como o `mold`) e usar o backend de compilação `Cranelift` no ambiente de desenvolvimento.
- [ ] **Feedback Loop de Sub-100ms:** Garantir que qualquer alteração em um controller ou modelo recompile apenas um micro-módulo isolado na memória, mantendo o Hot Reloading instantâneo mesmo em projetos com milhares de rotas.

---

## 🌐 Marco 14: Migrações Autônomas com IA e Banco de Dados Baseado em Intenção
**Objetivo:** Inverter o fluxo de design do banco de dados, deixando a IA gerar esquemas e índices perfeitamente otimizados a partir de descrições naturais.

- [ ] **Modelagem Baseada em Intenção (Intent-Based Modeling):** O desenvolvedor descreve a entidade com comentários ricos em Rust. A IA nativa lê, entende a intenção de negócio e gera automaticamente a migration perfeitamente otimizada para o banco (PostgreSQL/MySQL/SQLite).
- [ ] **Índices Auto-Otimizáveis:** Em produção, o Rullst monitora queries lentas (usando a telemetria do Marco 10) e sugere ou cria automaticamente índices secundários seguros para eliminar Table Scans lentos em tempo real.

---

## 🔬 Marco 15: Arquitetura Web Pronta para o Futuro Quântico (A Era Pós-Quântica)
**Objetivo:** Preparar a segurança e a infraestrutura do framework para o dia em que a computação quântica se tornar o padrão de mercado.

- [ ] **Criptografia Pós-Quântica Nativa (PQC):** Substituir gradualmente os algoritmos padrão do framework (JWT, Cookies, Sessões) por algoritmos resistentes a ataques quânticos, como Kyber e Dilithium (padrões NIST).
- [ ] **Abstração de Segurança Híbrida:** Implementar uma camada de transporte híbrida (TLS clássico + TLS quântico) por padrão, blindando a aplicação contra ataques de "Harvest Now, Decrypt Later".
- [ ] **Rullst QLink (`rullst::quantum`):** Camada de abstração de drivers para comunicação com processadores quânticos na nuvem (IBM Quantum, AWS Braket). Uma API fluida para despachar tarefas (quantum tasks) complexas da mesma forma que hoje se despacha tarefas para o Redis.

---

## 🧬 Marco 16: O Núcleo Auto-Evolutivo e Polimórfico (O Framework Mutável)
**Objetivo:** Transformar o framework de uma ferramenta estática em um organismo vivo de software que aprende, se otimiza e se cura sozinho em produção.

- [ ] **Compilação Polimórfica (Polymorphic Code Generation):** Usando telemetria e IA local, o framework analisa o tráfego em produção. Se perceber padrões de dados específicos, ele mesmo reescreve e recompila trechos da sua própria lógica em background (gerando novos .so/.dll) para criar caminhos de execução ultra-otimizados em tempo real.
- [ ] **Autonomous Error Auto-Healing em Produção:** Se o sistema detectar um pânico inédito em produção, a IA analisa o log, gera um patch corretivo, roda a suíte de testes em background e aplica o hot-swap do router em menos de 1 segundo — tudo sem intervenção humana. O dev apenas recebe um relatório informando que o bug foi resolvido enquanto ele dormia.

---

## 🗺️ Estratégia de Execução

Seguiremos **marco por marco**, começando pelo **Marco 1** para polir nossos geradores de CLI.

Se estiver pronto para começar, selecione uma tarefa ou sugira qual componente construir a seguir! 🚀
