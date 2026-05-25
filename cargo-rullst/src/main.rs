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
    /// Cria um novo Controller na pasta src/controllers/
    #[command(name = "make:controller")]
    MakeController {
        /// Nome do Controller (ex: UsersController ou users)
        name: String,
    },
    /// Cria um novo Model na pasta src/models/
    #[command(name = "make:model")]
    MakeModel {
        /// Nome do Model (ex: BlogPost ou blog_post)
        name: String,
        /// Opcional: cria uma migration SQL correspondente para a tabela
        #[arg(short, long)]
        migration: bool,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Se executado como um subcomando do cargo (ex: 'cargo rullst new'),
    // o cargo passa "rullst" como o primeiro argumento real.
    // Nós removemos ele da lista de argumentos para que o Clap consiga fazer o parse uniformemente.
    let args: Vec<String> = std::env::args().collect();
    let filtered_args = if args.len() > 1 && args[1] == "rullst" {
        let mut new_args = vec![args[0].clone()];
        new_args.extend_from_slice(&args[2..]);
        new_args
    } else {
        args
    };

    let cli = Cli::parse_from(filtered_args);

    match &cli.command {
        Commands::New { name } => {
            create_new_project(name)?;
        }
        Commands::MakeController { name } => {
            create_new_controller(name)?;
        }
        Commands::MakeModel { name, migration } => {
            create_new_model(name, *migration)?;
        }
    }

    Ok(())
}

/// Verifica se o diretório de execução atual é um projeto Rullst válido
fn is_rullst_project() -> bool {
    let cargo_toml_path = Path::new("Cargo.toml");
    if !cargo_toml_path.exists() {
        return false;
    }
    match fs::read_to_string(cargo_toml_path) {
        Ok(content) => content.contains("rullst"),
        Err(_) => false,
    }
}

/// Normaliza o nome do controller para snake_case com sufixo "_controller"
fn to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    // Remove sufixo case-insensitive se já existir
    if base.to_lowercase().ends_with("controller") {
        let len = base.len();
        base.truncate(len - 10);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    result.push_str("_controller");

    // Limpa possíveis underscores repetidos (ex: users__controller)
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result
}

/// Converte o nome do controller para CamelCase (PascalCase) com sufixo "Controller"
fn to_camel_case(s: &str) -> String {
    let snake = to_snake_case(s);
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Normaliza o nome do model para snake_case
fn model_to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    // Remove sufixo "Model" ou "model" se presente
    if base.to_lowercase().ends_with("model") {
        let len = base.len();
        base.truncate(len - 5);
    }

    let mut result = String::new();
    let mut prev_is_lower = false;
    for c in base.chars() {
        if c == '_' || c == '-' {
            result.push('_');
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                result.push('_');
            }
            result.push(c.to_ascii_lowercase());
            prev_is_lower = false;
        } else {
            result.push(c);
            prev_is_lower = true;
        }
    }

    // Limpa underscores repetidos
    let mut clean_result = String::new();
    let mut prev_is_underscore = false;
    for c in result.chars() {
        if c == '_' {
            if !prev_is_underscore {
                clean_result.push(c);
            }
            prev_is_underscore = true;
        } else {
            clean_result.push(c);
            prev_is_underscore = false;
        }
    }
    clean_result.trim_matches('_').to_string()
}

/// Converte o nome do model para PascalCase (CamelCase)
fn model_to_pascal_case(s: &str) -> String {
    let snake = model_to_snake_case(s);
    let mut result = String::new();
    let mut capitalize_next = true;
    for c in snake.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }
    result
}

/// Pluraliza o nome da tabela no padrão Active Record
fn pluralize(s: &str) -> String {
    let lower = s.to_lowercase();
    if lower.ends_with("ss") {
        format!("{}es", lower)
    } else if lower.ends_with("s") {
        lower
    } else if lower.ends_with("y") {
        let len = lower.len();
        if len > 1 {
            let before_y = &lower[len - 2..len - 1];
            if before_y == "a" || before_y == "e" || before_y == "i" || before_y == "o" || before_y == "u" {
                format!("{}s", lower)
            } else {
                format!("{}ies", &lower[..len - 1])
            }
        } else {
            format!("{}s", lower)
        }
    } else if lower.ends_with("ch") || lower.ends_with("sh") || lower.ends_with("x") || lower.ends_with("z") {
        format!("{}es", lower)
    } else {
        format!("{}s", lower)
    }
}

fn create_new_controller(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validar se está na raiz do projeto Rullst
    if !is_rullst_project() {
        println!("{}", "❌ Erro: Comando deve ser executado na raiz de um projeto Rullst válido.".red().bold());
        println!("{}", "Certifique-se de que a pasta atual contém um arquivo 'Cargo.toml' com dependência do 'rullst'.".yellow());
        std::process::exit(1);
    }

    let snake_name = to_snake_case(name);
    let camel_name = to_camel_case(name);

    println!("{}", format!("🛠️ Gerando controller Rullst: {}...", camel_name).cyan().bold());

    // 2. Garantir que a pasta src/controllers existe
    let controllers_dir = Path::new("src/controllers");
    if !controllers_dir.exists() {
        fs::create_dir_all(controllers_dir)?;
    }

    // 3. Garantir que o src/controllers/mod.rs existe
    let mod_path = controllers_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // 4. Registrar o novo controller no mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
        fs::write(&mod_path, mod_content)?;
    }

    // 5. Criar o arquivo do controller
    let controller_path = controllers_dir.join(format!("{}.rs", snake_name));
    if controller_path.exists() {
        println!("{}", format!("⚠️ Aviso: O controller '{}.rs' já existe. Pulando criação do arquivo.", snake_name).yellow());
    } else {
        let template = format!(
r#"use rullst::{{html, response::{{Html, IntoResponse}}}};

/// Retorna a lista de recursos
pub async fn index() -> impl IntoResponse {{
    Html(html! {{
        <div style="font-family: system-ui, sans-serif; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; background: #0f172a; color: #f8fafc; padding: 2rem; box-sizing: border-box;">
            <div style="max-width: 600px; text-align: center; background: #1e293b; padding: 3rem; border-radius: 1rem; border: 1px solid #334155; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.3);">
                <h1 style="font-size: 2.5rem; margin: 0 0 1rem 0; background: linear-gradient(to right, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; font-weight: 800;">
                    "{camel_name}"
                </h1>
                <p style="color: #94a3b8; font-size: 1.1rem; line-height: 1.6; margin-bottom: 2rem;">
                    "Este controller foi gerado automaticamente pelo Rullst CLI. Ele é 100% amigável para humanos e agentes de IA."
                </p>
                <div style="display: inline-block; padding: 0.75rem 1.5rem; background: #0f172a; border-radius: 0.5rem; border: 1px solid #334155; color: #38bdf8; font-family: monospace; font-size: 0.95rem;">
                    "pub async fn index() -> impl IntoResponse"
                </div>
            </div>
        </div>
    }})
}}

/// Retorna um recurso específico
pub async fn show() -> impl IntoResponse {{
    Html(html! {{
        <div style="font-family: system-ui, sans-serif; display: flex; flex-direction: column; align-items: center; justify-content: center; min-height: 100vh; background: #0f172a; color: #f8fafc; padding: 2rem; box-sizing: border-box;">
            <div style="max-width: 600px; text-align: center; background: #1e293b; padding: 3rem; border-radius: 1rem; border: 1px solid #334155; box-shadow: 0 10px 15px -3px rgba(0, 0, 0, 0.3);">
                <h1 style="font-size: 2.5rem; margin: 0 0 1rem 0; background: linear-gradient(to right, #38bdf8, #818cf8); -webkit-background-clip: text; -webkit-text-fill-color: transparent; font-weight: 800;">
                    "{camel_name} - Detalhes"
                </h1>
                <div style="display: inline-block; padding: 0.75rem 1.5rem; background: #0f172a; border-radius: 0.5rem; border: 1px solid #334155; color: #38bdf8; font-family: monospace; font-size: 0.95rem;">
                    "pub async fn show() -> impl IntoResponse"
                </div>
            </div>
        </div>
    }})
}}
"#);
        fs::write(&controller_path, template)?;
    }

    // 6. Tentar injetar "pub mod controllers;" no src/main.rs se necessário
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod controllers;") && !main_content.contains("mod controllers;") {
            main_content = format!("pub mod controllers;\n{}", main_content);
            fs::write(main_path, main_content)?;
            println!("{}", "ℹ️ Adicionado 'pub mod controllers;' ao topo de src/main.rs automaticamente.".cyan());
        }
    }

    println!("{}", format!("✨ Controller '{}' criado em '{}' com sucesso!", camel_name, controller_path.display()).green().bold());
    println!("{}", "Como mapear nas rotas:".cyan());
    println!("{}", format!("  1. Use: 'use crate::controllers::{};'", snake_name).cyan());
    println!("{}", format!("  2. Adicione: 'get(\"/url\" => {}::index)' no seu macro routes!.", snake_name).cyan());

    Ok(())
}

fn create_new_model(name: &str, create_migration: bool) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validar se está na raiz do projeto Rullst
    if !is_rullst_project() {
        println!("{}", "❌ Erro: Comando deve ser executado na raiz de um projeto Rullst válido.".red().bold());
        println!("{}", "Certifique-se de que a pasta atual contém um arquivo 'Cargo.toml' com dependência do 'rullst'.".yellow());
        std::process::exit(1);
    }

    let snake_name = model_to_snake_case(name);
    let pascal_name = model_to_pascal_case(name);
    let plural_name = pluralize(&snake_name);

    println!("{}", format!("🛠️ Gerando model Rullst: {}...", pascal_name).cyan().bold());

    // 2. Garantir que a pasta src/models existe
    let models_dir = Path::new("src/models");
    if !models_dir.exists() {
        fs::create_dir_all(models_dir)?;
    }

    // 3. Garantir que o src/models/mod.rs existe
    let mod_path = models_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // 4. Registrar o novo model no mod.rs
    let mut mod_content = fs::read_to_string(&mod_path)?;
    let mod_declaration = format!("pub mod {};", snake_name);
    if !mod_content.contains(&mod_declaration) {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str(&mod_declaration);
        mod_content.push('\n');
        fs::write(&mod_path, mod_content)?;
    }

    // 5. Criar o arquivo do model
    let model_path = models_dir.join(format!("{}.rs", snake_name));
    if model_path.exists() {
        println!("{}", format!("⚠️ Aviso: O model '{}.rs' já existe. Pulando criação do arquivo.", snake_name).yellow());
    } else {
        let template = format!(
r#"use rust_eloquent::{{Eloquent, EloquentModel, sqlx::{{self, FromRow}}}};

#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "{plural_name}")]
pub struct {pascal_name} {{
    pub id: i32,
    // Adicione seus campos aqui (ex: pub name: String)
}}
"#);
        fs::write(&model_path, template)?;
    }

    // 6. Tentar injetar "pub mod models;" no src/main.rs se necessário
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod models;") && !main_content.contains("mod models;") {
            main_content = format!("pub mod models;\n{}", main_content);
            fs::write(main_path, main_content)?;
            println!("{}", "ℹ️ Adicionado 'pub mod models;' ao topo de src/main.rs automaticamente.".cyan());
        }
    }

    println!("{}", format!("✨ Model '{}' criado em '{}' com sucesso!", pascal_name, model_path.display()).green().bold());

    // 7. Criar migration se solicitado
    if create_migration {
        let migrations_dir = Path::new("migrations");
        if !migrations_dir.exists() {
            fs::create_dir_all(migrations_dir)?;
        }

        // Formata o timestamp usando chrono (ex: YYYYMMDDHHMMSS)
        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let migration_filename = format!("{}_create_{}_table.sql", timestamp, plural_name);
        let migration_path = migrations_dir.join(&migration_filename);

        let sql_template = format!(
r#"-- Up
CREATE TABLE IF NOT EXISTS {plural_name} (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    -- Adicione seus campos aqui (ex: name TEXT NOT NULL)
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Down
DROP TABLE IF EXISTS {plural_name};
"#);

        fs::write(&migration_path, sql_template)?;
        println!("{}", format!("✨ Migration SQL criada em '{}' com sucesso!", migration_path.display()).green().bold());
    }

    println!("{}", "Como importar e usar:".cyan());
    println!("{}", format!("  1. Use: 'use crate::models::{}::{};'", snake_name, pascal_name).cyan());
    println!("{}", format!("  2. Busque dados: 'let items = {}::all().await?;'", pascal_name).cyan());

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

    // Extract a valid package name from the path (e.g. "..\dummy_test" -> "dummy_test")
    let project_name = path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(name)
        .replace("\\", "")
        .replace("/", "")
        .replace(".", "")
        .replace("-", "_");

    // Write Cargo.toml
    let cargo_toml = format!(
r#"[package]
name = "{project_name}"
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
