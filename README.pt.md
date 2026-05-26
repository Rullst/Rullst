<p align="center">
  <img src="./Rullst.png" alt="Rullst Logo" width="500">
</p>

# Rullst - 📜🦀🌐🤖🚀
### *"Rust para quem quer construir, não sofrer."*

> 📖 **[Veja todas as mudanças no nosso Changelog!](./CHANGELOG.md)**  
> 📚 **[Leia a Documentação Oficial!](https://venelouis.github.io/Rullst/)**  
> 📦 **[Veja no Crates.io!](https://crates.io/crates/rullst)**

> [!WARNING]  
> **Em construção! 🚧**  
> O Rullst está em **desenvolvimento constante e melhoria rápida**. Como estamos estabilizando o framework e atualizando dependências vitais, você pode eventualmente encontrar bugs ou "crashes". Pedimos perdão sincero por qualquer inconveniência! Por favor, considere tornar-se um **contribuinte** para nos ajudar a construir talvez o melhor framework web de toda a internet. 🦀❤️

![Crates.io](https://img.shields.io/crates/v/rullst?style=flat-square&color=orange)
![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Status: v1.0.4](https://img.shields.io/badge/Status-v1.0.4-emerald)
![Built with: Axum & Rust Eloquent](https://img.shields.io/badge/Stack-Axum%20%7C%20Rust%20Eloquent-blue)

O **Rullst** (Rust + Fullstack) é o primeiro framework web opinativo em Rust projetado obsessivamente para a **Produtividade Emocional** do desenvolvedor. 

Ele foi criado para preencher o maior abismo existente na comunidade Rust atual: a barreira que transforma a programação web em uma pesquisa de doutorado em compiladores. Nós acreditamos que você deve gastar energia criando seu negócio, não lutando contra o compilador.

---

## 💡 O Manifesto Rullst

> *"A maioria dos frameworks Rust trata o desenvolvedor web como um engenheiro de compiladores. O Rullst trata o desenvolvedor como alguém que quer construir produtos incríveis rapidamente."*

No ecossistema atual, para fazer um CRUD simples, você é forçado a lutar contra o Borrow Checker dentro de closures complexas de HTML, mapear rotas em arquivos gigantescos e lidar com ORMs verbosos que exigem dezenas de structs para uma única tabela.

O Rullst redefine essa experiência. Nós oferecemos um ecossistema integrado que traz a doçura e a velocidade de desenvolvimento do **Laravel/Next.js** com a velocidade de Fórmula 1 do **Rust/Axum**:

* **Chega de Frankenstein:** Um único framework que gerencia seu servidor (Axum), seu banco de dados (`rust-eloquent`) e sua renderização de interface.
* **Chega de lutar contra o Borrow Checker na UI:** A macro `html!` processa JSX-like puro direto no servidor (SSR). É HTML bruto, seguro e ultra-rápido enviado diretamente para o navegador.
* **Active Record Real:** Integramos de forma nativa o ORM **`rust-eloquent`**. Salvar e gerenciar registros é tão simples quanto `user.save()`.
* **Engenharia Nativa para IA & AI-Friendly:** Projetado desde o primeiro dia para pareamento moderno com inteligências artificiais. Tipagem forte, zero mágica dinâmica em tempo de execução, arquivos `.ai-rules` gerados no scaffold e esquemas estruturados evitam alucinações de agentes de IA e permitem correção automática imediata pelo compilador.

---

## 🏆 Tudo Que Você Precisa, Incluso (v1.0.0)

O Rullst entrega **7 marcos completos** cobrindo todas as camadas do desenvolvimento web moderno:

| Categoria | Funcionalidades |
|---|---|
| 🛠️ **CLI & DX** | Wizard `cargo rullst new`, `make:controller`, `make:model -m`, `make:middleware`, `make:worker`, `generate:openapi`, `cargo rullst upgrade` (auto-cura) |
| 🗄️ **Banco de Dados** | ORM Active Record, Migrations (`db:migrate`, `db:rollback`, `db:status`), Seeders & Factories, HasMany / BelongsTo / BelongsToMany, Eager Loading |
| 🔒 **Auth & Segurança** | Hashing Argon2, Sessões JWT & Cookie, Proteção CSRF, OAuth Social (Google, GitHub, Facebook, Twitter via `rust-socialite`), `cargo rullst auth` |
| ⚡ **Frontend** | Suporte HTMX de primeira classe, TailwindCSS integrado, renderização parcial, **Rullst Live** (UI server-driven inspirada no Phoenix LiveView), **Wasm Islands** (`#[client_component]`) |
| 📦 **Produção** | Filas (SQLite/Redis), Cache (Memory/Redis), Task Scheduler (Cron), Docker multi-stage builds, Dashboard **Rullst Horizon** |
| 🏢 **Enterprise** | Validação Declarativa, Mailer (SMTP/Resend/SendGrid), Storage (Local/S3/R2), WebSockets, Multi-Tenancy, Feature Flags, Testes E2E |
| 🚀 **Vantagem Injusta** | **AI Core** (`rullst::ai` — OpenAI/Gemini/Anthropic/Ollama + RAG), **Rullst Studio** (GUI visual de DB), **Console Self-Healing com IA** (auto-fix), **Hot Reloading via `dylib`** |

---

## 🛠️ Como é Programar no Rullst?

Esqueça o boilerplate. Uma aplicação Rullst completa com banco de dados em memória e renderização segura contra XSS possui exatamente esta cara:

```rust
use rullst::{html, routes, Server, Router, response::{Html, IntoResponse}};
use rust_eloquent::{Eloquent, EloquentModel, sqlx::{self, FromRow}};

// 1. Defina seu modelo Active Record instantaneamente
#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
}

// 2. Rota HTML com JSX-like limpo, rápido e seguro contra XSS
async fn home() -> impl IntoResponse {
    let name = "Rullst";
    let users = User::all().await.unwrap();

    Html(html! {
        <div style="font-family: sans-serif; text-align: center; padding: 5rem; background: #0f172a; color: #f8fafc; height: 100vh;">
            <h1 style="font-size: 3rem; background: linear-gradient(to right, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent;">
                "Bem-vindo ao " {name}
            </h1>
            <p style="color: #94a3b8; font-size: 1.2rem; margin-bottom: 2rem;">
                "Rust para quem quer construir, não sofrer."
            </p>
            <div style="display: inline-block; padding: 1rem 2rem; background: #1e293b; border-radius: 0.5rem; border: 1px solid #334155;">
                "Total de usuários no banco: " {users.len()}
            </div>
        </div>
    })
}

// 1. Declare a macro artisan aqui para interceptar argumentos CLI para migrações
rullst::artisan!();

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 2. A macro artisan! intercepta comandos `db:*` automaticamente.
    // Se for uma execução normal, o servidor continua daqui em diante.

    let router = routes![
        get("/" => home),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
```

---

## ⚡ Comece em 10 Segundos

Gere uma aplicação totalmente operacional com o nosso assistente CLI interativo!

```bash
# 1. Execute o assistente de linha de comando interativo
cargo rullst new

# O assistente vai perguntar:
# 🚀 App name? (no spaces allowed) -> meu-app
# 🏗️ What would you like to build? -> Full-Stack Web App (SaaS, Portfolio, Blog) / REST API
# 🔥 Enable Hot Reloading by default? -> Yes / No
# 🗄️ Will your project need a Data Base? -> Yes / No
# 💾 Select a DB Provider -> Sqlite / Postgres / MySQL/MariaDB

# 2. Entre na pasta do projeto
cd meu-app

# 3. Inicie sua aplicação full-stack de alta performance imediatamente!
cargo run

# 🔥 Ou ative o Hot Reloading instantâneo (sem reiniciar o servidor!):
HOT_RELOAD=1 cargo run
```

---

## 🗄️ Migrações de Banco de Dados (Artisan CLI)

O Rullst possui um executor de migrações embutido de altíssima performance. Não há necessidade de binários externos, o CLI roda as migrações geradas usando closures de Rust puro!

```bash
# Cria uma nova migração (Rust DSL)
cargo rullst make:migration create_users_table

# Roda todas as migrações pendentes no banco
cargo rullst db:migrate

# Desfaz o último lote de migrações
cargo rullst db:rollback
```

Por baixo dos panos, a macro `rullst::artisan!()` cuida de barrar a inicialização do seu servidor web caso o processo tenha sido executado exclusivamente para gerenciar banco de dados.

---

## ⚡ Começando em 10 Segundos com a CLI

Instale a CLI do Rullst e crie aplicações completas instantaneamente:

```bash
# 1. Compile e execute a CLI oficial do monorepo
cargo run -p cargo-rullst -- new meu-app

# 2. Acesse a pasta do projeto
cd meu-app

# 3. Rode sua aplicação web + banco de dados imediatamente!
cargo run
```

---

## 🛡️ Atualizações Seguras (Self-Healing Upgrades)

Tem medo de quebrar sua aplicação ao atualizar o framework? Não tenha. O Rullst foi construído com uma filosofia de "Atualizações Auto-Curáveis".

Quando uma nova versão do Rullst introduz mudanças na API, nós nunca quebramos seu código imediatamente. Em vez disso, usamos avisos `#[deprecated]`. Você pode atualizar toda a sua aplicação automaticamente usando nossa CLI:

```bash
cargo rullst upgrade
```

Este comando atualizará a dependência do Rullst de forma segura e utilizará as poderosas ferramentas de refatoração `cargo fix` do Rust para reescrever seu código automaticamente, adaptando-o para as novas assinaturas de API. Atualizações sem estresse, para sempre.

---

## 🔥 Hot Reloading (Loop de Dev Sem Downtime)

O Rullst suporta **Hot Reloading via Dynamic Linking** — altere suas rotas, handlers e templates e veja as mudanças refletidas **instantaneamente** sem reiniciar o servidor ou perder conexões:

```bash
# Inicie sua aplicação em modo hot-reload
HOT_RELOAD=1 cargo run

# ⚡ Edite qualquer handler em src/ → Rullst detecta a mudança
# 🔄 Recompilação de fundo da cdylib
# 🚀 Router trocado atomicamente — zero downtime!
```

Por baixo dos panos, o Rullst compila suas rotas como biblioteca dinâmica (`cdylib`), carrega via `libloading`, e usa um file-watcher (`notify`) para detectar mudanças e disparar rebuilds em background. O router é trocado atomicamente via `Arc<RwLock<Router>>` — o servidor HTTP nunca reinicia e conexões TCP nunca são derrubadas.

---

## 🎯 Arquitetura do Rullst (v1.0.3)

O Rullst é estruturado como um monorepo Cargo Workspace altamente modularizado:

1. **`rullst` (Core Crate):** Abstrai o servidor Axum, gerencia a injeção do ciclo de vida do `rust-eloquent` e re-exporta as dependências de rede e segurança. Inclui utilitários de produção (Queue, Cache, Scheduler), funcionalidades enterprise (Validation, Mailer, Storage, WebSockets, Horizon), Core IA (`rullst::ai`), Rullst Live (UI server-driven), Wasm Islands, e Hot Reloading via dynamic linking.
2. **`rullst-macros` (Compile-time Engine):** Contém a macro procedural `html!` que faz parsing da árvore JSX e gera concatenações estáticas otimizadas na memória em tempo de compilação.
3. **`cargo-rullst` (Developer CLI):** A ferramenta que cuida de gerar novos projetos com modelos prontos, conexão de banco configurada e páginas de exemplo.

Para convenções técnicas de arquitetura, padrões de pastas e APIs detalhadas, consulte a nossa [Especificação Oficial (SST)](./docs/spec.md).

---

## 📝 Licença

Distribuído sob a licença MIT. Veja `LICENSE` para mais informações.
