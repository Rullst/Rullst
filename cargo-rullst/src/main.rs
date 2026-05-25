use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use colored::*;

#[derive(Parser)]
#[command(name = "cargo-rullst")]
#[command(about = "CLI oficial do Rullst Framework", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Cria uma nova aplicação Rullst
    New {
        /// Nome do projeto
        name: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::New { name } => {
            create_new_project(name)?;
        }
    }

    Ok(())
}

fn create_new_project(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", format!("🚀 Criando nova aplicação Rullst: {}...", name).green().bold());
    
    let path = Path::new(name);
    if path.exists() {
        println!("{}", format!("❌ Erro: A pasta '{}' já existe.", name).red());
        std::process::exit(1);
    }
    
    // Create folders
    fs::create_dir_all(path.join("src/pages"))?;
    fs::create_dir_all(path.join("src/models"))?;
    
    // Get absolute path to the Rullst framework folder for local referencing
    let current_dir = std::env::current_dir()?;
    let rullst_path = if current_dir.join("rullst").exists() {
        current_dir.join("rullst").canonicalize()?.display().to_string()
    } else {
        "c:\\Users\\venelouis\\Desktop\\REPOS\\Rullst\\rullst".to_string()
    };
    
    // Get absolute path to rust-eloquent for local referencing
    let rust_eloquent_path = if current_dir.join("rust-eloquent").exists() {
        current_dir.join("rust-eloquent/rust-eloquent").canonicalize()?.display().to_string()
    } else if current_dir.parent().map(|p| p.join("rust-eloquent/rust-eloquent").exists()).unwrap_or(false) {
        current_dir.parent().unwrap().join("rust-eloquent/rust-eloquent").canonicalize()?.display().to_string()
    } else {
        "c:\\Users\\venelouis\\Desktop\\REPOS\\rust-eloquent\\rust-eloquent".to_string()
    };
    
    // Fix Windows path escaping in Cargo.toml and strip UNC prefix \\?\ if present
    let rullst_path = rullst_path.trim_start_matches(r"\\?\").replace("\\", "/");
    let rust_eloquent_path = rust_eloquent_path.trim_start_matches(r"\\?\").replace("\\", "/");


    // Write Cargo.toml
    let cargo_toml = format!(
r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
rullst = {{ path = "{rullst_path}" }}
rust-eloquent = {{ path = "{rust_eloquent_path}" }}
tokio = {{ version = "1.43", features = ["full"] }}
serde = {{ version = "1.0", features = ["derive"] }}
sqlx = {{ version = "0.8", features = ["sqlite", "runtime-tokio"] }}

[workspace]
"#);

    fs::write(path.join("Cargo.toml"), cargo_toml)?;

    // Write Rullst.toml configuration
    let rullst_toml = r#"[database]
url = "sqlite://rullst.db"
"#;
    fs::write(path.join("Rullst.toml"), rullst_toml)?;

    // Write src/main.rs
    let main_rs = r#"use rullst::{html, routes, Server, Router, response::{Html, IntoResponse}};
use rust_eloquent::{Eloquent, EloquentModel, sqlx::{self, FromRow}};


// 1. Defina o seu modelo de banco de dados usando o ORM rust-eloquent embutido!
#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
}

async fn home() -> impl IntoResponse {
    let name = "Rullst";
    
    // Exemplo de uso do ORM: Buscar usuários ativos do banco
    let db_status = match User::all().await {
        Ok(users) => format!("Banco conectado! Total de usuários cadastrados: {}", users.len()),
        Err(e) => format!("Banco offline ou não configurado: {}", e),
    };

    Html(html! {
        <div style="font-family: sans-serif; display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100vh; background: #0f172a; color: #f8fafc;">
            <h1 style="font-size: 3rem; margin-bottom: 0.5rem; background: linear-gradient(to right, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent;">
                "Bem-vindo ao " {name}
            </h1>
            <p style="color: #94a3b8; font-size: 1.2rem; margin-bottom: 2rem;">
                "O framework fullstack definitivo para Rust. Focado em Segurança, Manutenção e Velocidade."
            </p>
            <div style="padding: 1rem 2rem; background: #1e293b; border-radius: 0.5rem; border: 1px solid #334155; color: #38bdf8;">
                {db_status}
            </div>
        </div>
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Inicializa uma conexão SQLite em memória para o nosso modelo de exemplo rodar instantaneamente!
    Eloquent::init("sqlite::memory:").await?;

    // Executa uma migração manual para criar a tabela de usuários do nosso exemplo
    let pool = Eloquent::pool();
    sqlx::query("CREATE TABLE IF NOT EXISTS users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)")
        .execute(pool)
        .await?;

    // Insere um usuário de exemplo usando a facilidade do Active Record!
    let mut demo_user = User { id: 0, name: "Admin Rullst".to_string() };
    demo_user.save().await?;

    let router = routes![
        get("/" => home),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
"#;

    fs::write(path.join("src/main.rs"), main_rs)?;

    println!("{}", format!("✨ Projeto '{}' criado com sucesso!", name).green().bold());
    println!("{}", "Como rodar:".cyan());
    println!("{}", format!("  cd {}", name).cyan());
    println!("{}", "  cargo run".cyan());

    Ok(())
}
