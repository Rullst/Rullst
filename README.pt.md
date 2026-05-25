# Rullst 🚀
### *"Rust para quem quer construir, não sofrer."*

![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)
![Status: v0.6.0](https://img.shields.io/badge/Status-v0.6.0-emerald)
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

## 🎯 Arquitetura do Rullst (v0.6.0)

O Rullst é estruturado como um monorepo Cargo Workspace altamente modularizado:

1. **`rullst` (Core Crate):** Abstrai o servidor Axum, gerencia a injeção do ciclo de vida do `rust-eloquent` e re-exporta as dependências de rede e segurança. Também vem com utilitários de produção de primeira classe (Queue, Cache, Scheduler) e funcionalidades corporativas enterprise (Validation, Mailer, Storage, WebSockets e Horizon).
2. **`rullst-macros` (Compile-time Engine):** Contém a macro procedural `html!` que faz parsing da árvore JSX e gera concatenações estáticas otimizadas na memória em tempo de compilação.
3. **`cargo-rullst` (Developer CLI):** A ferramenta que cuida de gerar novos projetos com modelos prontos, conexão de banco configurada e páginas de exemplo.

Para convenções técnicas de arquitetura, padrões de pastas e APIs detalhadas, consulte a nossa [Especificação Oficial (SST)](./docs/spec.md).

---

## 📝 Licença

Distribuído sob a licença MIT. Veja `LICENSE` para mais informações.
