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
    M0[Pilar: Design Nativo para IA] --> M1[Marco 1: CLI e Geradores de Código]
    M1 --> M2[Marco 2: Supremacia do Banco de Dados]
    M2 --> M3[Marco 3: Autenticação Completa e Segurança]
    M3 --> M4[Marco 4: Integração com HTMX e Frontend]
    M4 --> M5[Marco 5: Utilitários de Produção]
    style M0 fill:#ffecd2,stroke:#ff9a00,stroke-width:3px,color:#000
    style M1 fill:#00f2fe,stroke:#fff,stroke-width:2px,color:#000
    style M2 fill:#4facfe,stroke:#fff,stroke-width:2px,color:#000
    style M3 fill:#a18cd1,stroke:#fff,stroke-width:2px,color:#000
    style M4 fill:#fbc2eb,stroke:#fff,stroke-width:2px,color:#000
    style M5 fill:#ff9a9e,stroke:#fff,stroke-width:2px,color:#000
```

---

## 🛠️ Marco 1: Poder do CLI (`cargo-rullst`)
**Objetivo:** Permitir scaffold e geração de código em segundos. Desenvolvedores não devem criar arquivos de boilerplate manualmente.

- [ ] **Geradores de Código:**
  - [ ] `cargo rullst make:controller <Nome>` - Gera um controller com as ações básicas de CRUD.
  - [ ] `cargo rullst make:model <Nome> [-m]` - Gera um model de Active Record e, opcionalmente, uma migration associada.
  - [ ] `cargo rullst make:middleware <Nome>` - Gera um middleware customizado compatível com Axum.
- [ ] **Ergonomia do Workspace:**
  - [ ] Melhorar a velocidade de compilação durante as execuções do CLI.
  - [ ] Suporte à flag `--api` para criar scaffolds de APIs REST sem frontend HTML.

---

## 🗄️ Marco 2: Supremacia do Banco de Dados (Migrations & Relacionamentos)
**Objetivo:** Capacitar o `rust-eloquent` e o `Rullst` a gerenciar esquemas relacionais complexos sem complicação.

> [!NOTE]
> **Divisão de Responsabilidades:**
> O trabalho pesado (parsers de esquema SQL, execução de migrations e macros de relacionamento) será desenvolvido diretamente dentro do repositório **`rust-eloquent`** para manter o ORM modular e atraente para toda a comunidade Rust.
> O **Rullst** irá envelopar essas funcionalidades com comandos amigáveis de CLI e injeção automática de conexões.

- [ ] **Motor de Migrations (no `rust-eloquent`):**
  - [ ] Definição de migrations em SQL puro ou através de DSL intuitiva.
  - [ ] Executores CLI integrados no Rullst:
    - [ ] `cargo rullst db:migrate` - Executa migrations pendentes.
    - [ ] `cargo rullst db:rollback` - Reverte o último lote de migrations.
    - [ ] `cargo rullst db:status` - Mostra o histórico de migrações aplicadas.
- [ ] **Relacionamentos Active Record (no `rust-eloquent`):**
  - [ ] Macros declarativas de relacionamento como `HasMany` e `BelongsTo`.
  - [ ] Resolução de associações `BelongsToMany` (Muitos para Muitos).
  - [ ] Mecanismos de Lazy e Eager loading para evitar problemas de consultas N+1.
- [ ] **Seeders e Factories:**
  - [ ] `cargo rullst db:seed` - Popula o banco de dados usando dados fakes pré-configurados.
  - [ ] Padrão de factories integrado para geração ágil de entidades de teste.

---

## 🔒 Marco 3: Autenticação & Segurança (Social & Local Auth)
**Objetivo:** Implementar autenticação robusta, segura e instantânea. Qualquer dev deve ser capaz de autenticar usuários de forma segura em minutos.

- [ ] **Autenticação Social via `rust-socialite`:**
  - [ ] Integrar a crate **[`rust-socialite`](https://crates.io/crates/rust-socialite)** (sua criação!) como o motor oficial de OAuth do framework.
  - [ ] Configurações out-of-the-box para Google, GitHub, Facebook, Twitter e provedores genéricos de OpenID.
  - [ ] Fluxo fluido: redirecionar para o provedor, tratar o callback e autenticar/registrar o usuário via Active Record.
- [ ] **Autenticação Local:**
  - [ ] Auxiliares embutidos para hashing seguro de senhas via Argon2/Bcrypt.
  - [ ] Middlewares customizados para sessões seguras baseadas em Cookies e Tokens (JWT).
- [ ] **O Comando "Mágico" de Auth:**
  - [ ] `cargo rullst auth` - Cria instantaneamente um sistema completo de login e registro contendo:
    - Controllers de Login, Registro e Reset de Senha.
    - Telas bonitas e responsivas (templates `html!`) pré-estilizadas.
    - Migration SQL para a tabela de `users`.
- [ ] **Padrões de Segurança Robustos:**
  - [ ] Proteção automática contra ataques CSRF em submissões de formulários HTML.
  - [ ] Middleware padrão de cabeçalhos de segurança (CORS, HSTS, X-Content-Type-Options).

---

## ⚡ Marco 4: HTMX & Interatividade
**Objetivo:** Combinar a simplicidade de Server-Side Rendering (SSR) com a fluidez de Single-Page Applications (SPAs).

- [ ] **Suporte de Primeira Classe ao HTMX:**
  - [ ] Helpers para verificar cabeçalhos de requisição do HTMX (`rullst::htmx::is_htmx(req)`).
  - [ ] Suporte nativo para renderização de templates parciais (renderizar apenas o componente modificado, sem carregar o layout inteiro).
  - [ ] Integração nativa e configuração automática do TailwindCSS na inicialização do projeto.

---

## 📦 Marco 5: Utilitários de Produção (Filas, Caching e Scheduler)
**Objetivo:** Fornecer os recursos fundamentais necessários para escalar aplicações reais em produção.

- [ ] **Filas & Tarefas em Segundo Plano:**
  - [ ] API unificada `rullst::queue` com suporte a SQLite (para dev local) e Redis (para produção).
  - [ ] Executores assíncronos (workers) rodando tarefas pesadas em background.
- [ ] **Camada de Cache:**
  - [ ] API unificada `rullst::cache` com drivers para In-Memory e Redis.
- [ ] **Agendador de Tarefas (Task Scheduler):**
  - [ ] Agendamento declarativo tipo Cron diretamente no `main.rs` (ex: `.schedule("0 0 * * *", limpeza_diaria)`).

---

## 🗺️ Estratégia de Execução

Nós avançaremos **marco por marco**, começando pelo **Marco 1** para lapidar nossos geradores no CLI. 

Quando estiver pronto para começar, escolha uma tarefa ou sugira qual componente criar a seguir! 🚀
