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
        /// Opcional: cria uma aplicação REST headless (sem HTML)
        #[arg(long)]
        api: bool,
    },
    /// Cria um novo Controller na pasta src/controllers/
    #[command(name = "make:controller")]
    MakeController {
        /// Nome do Controller (ex: UsersController ou users)
        name: String,
        /// Opcional: gera as rotas e respostas em formato JSON (API REST headless) em vez de HTML
        #[arg(long)]
        api: bool,
    },
    /// Cria um novo Model na pasta src/models/
    #[command(name = "make:model")]
    MakeModel {
        /// Nome do Model (ex: BlogPost ou blog_post)
        name: String,
        /// Opcional: cria uma migration correspondente para a tabela
        #[arg(short, long)]
        migration: bool,
    },
    /// Cria um novo Middleware na pasta src/middlewares/
    #[command(name = "make:middleware")]
    MakeMiddleware {
        /// Nome do Middleware (ex: Auth ou auth_middleware)
        name: String,
    },
    /// Executa as migrações pendentes no banco de dados
    #[command(name = "db:migrate")]
    DbMigrate,
    /// Reverte o último lote de migrações aplicadas
    #[command(name = "db:rollback")]
    DbRollback,
    /// Mostra o status atual das migrações do projeto
    #[command(name = "db:status")]
    DbStatus,
    /// Popula o banco de dados usando seeders pré-configurados
    #[command(name = "db:seed")]
    DbSeed,
    /// Cria uma nova migração vazia na pasta src/migrations/
    #[command(name = "make:migration")]
    MakeMigration {
        /// Nome da migração (ex: create_users_table)
        name: String,
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
        Commands::New { name, api } => {
            create_new_project(name, *api)?;
        }
        Commands::MakeController { name, api } => {
            create_new_controller(name, *api)?;
        }
        Commands::MakeModel { name, migration } => {
            create_new_model(name, *migration)?;
        }
        Commands::MakeMiddleware { name } => {
            create_new_middleware(name)?;
        }
        Commands::DbMigrate => {
            run_project_db_command("db:migrate")?;
        }
        Commands::DbRollback => {
            run_project_db_command("db:rollback")?;
        }
        Commands::DbStatus => {
            run_project_db_command("db:status")?;
        }
        Commands::DbSeed => {
            run_project_db_command("db:seed")?;
        }
        Commands::MakeMigration { name } => {
            create_new_migration(name)?;
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

/// Normaliza o nome do middleware para snake_case com sufixo "_middleware"
fn middleware_to_snake_case(s: &str) -> String {
    let mut base = s.to_string();
    // Remove sufixo case-insensitive se já existir
    if base.to_lowercase().ends_with("middleware") {
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

    result.push_str("_middleware");

    // Limpa possíveis underscores repetidos (ex: auth__middleware)
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

fn create_new_controller(name: &str, api: bool) -> Result<(), Box<dyn std::error::Error>> {
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
        let template = if api {
            format!(
r#"use axum::{{extract::{{Path, Form}}, response::IntoResponse, Json}};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateDto {{
    // Adicione os campos para criação
}}

#[derive(Deserialize)]
pub struct UpdateDto {{
    // Adicione os campos para atualização
}}

/// Retorna a lista de recursos
pub async fn index() -> impl IntoResponse {{
    Json(serde_json::json!({{
        "controller": "{camel_name}",
        "action": "index",
        "message": "Este controller foi gerado automaticamente pelo Rullst CLI. Ele é 100% amigável para humanos e agentes de IA."
    }}))
}}

/// Retorna um recurso específico
pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "controller": "{camel_name}",
        "action": "show",
        "id": id
    }}))
}}

/// Cria um novo recurso
pub async fn store(Form(_payload): Form<CreateDto>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "message": "Recurso criado com sucesso"
    }}))
}}

/// Atualiza um recurso existente
pub async fn update(Path(id): Path<i32>, Form(_payload): Form<UpdateDto>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "id": id,
        "message": "Recurso atualizado com sucesso"
    }}))
}}

/// Deleta um recurso
pub async fn delete(Path(id): Path<i32>) -> impl IntoResponse {{
    Json(serde_json::json!({{
        "id": id,
        "message": "Recurso deletado com sucesso"
    }}))
}}
"#)
        } else {
            format!(
r#"use rullst::{{html, response::{{Html, IntoResponse}}}};
use axum::extract::{{Path, Form}};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct CreateDto {{
    // Adicione os campos para criação
}}

#[derive(Deserialize)]
pub struct UpdateDto {{
    // Adicione os campos para atualização
}}

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
pub async fn show(Path(id): Path<i32>) -> impl IntoResponse {{
    Html(html! {{ 
        <div>"Detalhes do recurso "{{id}}</div> 
    }})
}}

/// Cria um novo recurso
pub async fn store(Form(_payload): Form<CreateDto>) -> impl IntoResponse {{
    Html(html! {{ <div>"Recurso criado com sucesso"</div> }})
}}

/// Atualiza um recurso existente
pub async fn update(Path(id): Path<i32>, Form(_payload): Form<UpdateDto>) -> impl IntoResponse {{
    Html(html! {{ <div>"Recurso "{{id}}" atualizado com sucesso"</div> }})
}}

/// Deleta um recurso
pub async fn delete(Path(id): Path<i32>) -> impl IntoResponse {{
    Html(html! {{ <div>"Recurso "{{id}}" deletado com sucesso"</div> }})
}}
"#)
        };
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
        let migrations_dir = Path::new("src/migrations");
        if !migrations_dir.exists() {
            fs::create_dir_all(migrations_dir)?;
        }

        let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
        let migration_name = format!("create_{}", plural_name);
        let file_stem = format!("m{}_{}", timestamp, migration_name);
        let migration_path = migrations_dir.join(format!("{}.rs", file_stem));

        let template = format!(
r#"use rust_eloquent::schema::{{Schema, Blueprint, Migration}};
use rust_eloquent::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rust_eloquent::sqlx::Error> {{
        Schema::create("{plural_name}", |table| {{
            table.id();
            // Adicione seus campos aqui (ex: table.string("title");)
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rust_eloquent::sqlx::Error> {{
        Schema::drop_if_exists("{plural_name}").await
    }}
}}
"#,
            file_stem = file_stem,
            plural_name = plural_name
        );

        fs::write(&migration_path, template)?;
        println!("{}", format!("✨ Migração em Rust criada em '{}' com sucesso!", migration_path.display()).green().bold());

        // Regenerar src/migrations/mod.rs
        regenerate_migrations_mod()?;
    }

    println!("{}", "Como importar e usar:".cyan());
    println!("{}", format!("  1. Use: 'use crate::models::{}::{};'", snake_name, pascal_name).cyan());
    println!("{}", format!("  2. Busque dados: 'let items = {}::all().await?;'", pascal_name).cyan());

    Ok(())
}

fn create_new_middleware(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Validar se está na raiz do projeto Rullst
    if !is_rullst_project() {
        println!("{}", "❌ Erro: Comando deve ser executado na raiz de um projeto Rullst válido.".red().bold());
        println!("{}", "Certifique-se de que a pasta atual contém um arquivo 'Cargo.toml' com dependência do 'rullst'.".yellow());
        std::process::exit(1);
    }

    let snake_name = middleware_to_snake_case(name);

    println!("{}", format!("🛠️ Gerando middleware Rullst: {}...", snake_name).cyan().bold());

    // 2. Garantir que a pasta src/middlewares existe
    let middlewares_dir = Path::new("src/middlewares");
    if !middlewares_dir.exists() {
        fs::create_dir_all(middlewares_dir)?;
    }

    // 3. Garantir que o src/middlewares/mod.rs existe
    let mod_path = middlewares_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    // 4. Registrar o novo middleware no mod.rs
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

    // 5. Criar o arquivo do middleware
    let middleware_path = middlewares_dir.join(format!("{}.rs", snake_name));
    if middleware_path.exists() {
        println!("{}", format!("⚠️ Aviso: O middleware '{}.rs' já existe. Pulando criação do arquivo.", snake_name).yellow());
    } else {
        let template = format!(
r#"use axum::{{extract::Request, middleware::Next, response::Response}};

pub async fn {}(req: Request, next: Next) -> Response {{
    // Pre-request logic here
    
    let response = next.run(req).await;
    
    // Post-request logic here
    
    response
}}
"#, snake_name);
        fs::write(&middleware_path, template)?;
    }

    // 6. Tentar injetar "pub mod middlewares;" no src/main.rs se necessário
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod middlewares;") && !main_content.contains("mod middlewares;") {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace("pub mod controllers;", "pub mod controllers;\npub mod middlewares;");
            } else if main_content.contains("pub mod models;") {
                main_content = main_content.replace("pub mod models;", "pub mod models;\npub mod middlewares;");
            } else {
                main_content = format!("pub mod middlewares;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!("{}", "ℹ️ Adicionado 'pub mod middlewares;' ao src/main.rs automaticamente.".cyan());
        }
    }

    println!("{}", format!("✨ Middleware '{}' criado em '{}' com sucesso!", snake_name, middleware_path.display()).green().bold());
    println!("{}", "Como mapear nas rotas usando Axum layers:".cyan());
    println!("{}", "  1. Use: 'use axum::middleware::from_fn;'".cyan());
    println!("{}", format!("  2. Use: 'use crate::middlewares::{}::{};'", snake_name, snake_name).cyan());
    println!("{}", format!("  3. Adicione: '.layer(from_fn({}))' no seu router.", snake_name).cyan());

    Ok(())
}

fn create_new_project(name: &str, api: bool) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", format!("🚀 Criando nova aplicação Rullst: {}...", name).green().bold());
    
    let path = Path::new(name);
    if path.exists() {
        println!("{}", format!("❌ Erro: A pasta '{}' já existe.", name).red());
        std::process::exit(1);
    }
    
    // Create folders
    fs::create_dir_all(path.join("src/pages"))?;
    fs::create_dir_all(path.join("src/models"))?;
    
    // Scaffold initial src/migrations/mod.rs file
    let migrations_dir = path.join("src/migrations");
    fs::create_dir_all(&migrations_dir)?;
    fs::write(migrations_dir.join("mod.rs"), r#"// Generated by Rullst.

pub fn get_migrations() -> Vec<Box<dyn rust_eloquent::schema::Migration>> {
    vec![]
}
"#)?;
    
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
serde_json = "1.0"
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
    let main_rs = if api {
        r#"use rullst::{routes, Server, Router, response::IntoResponse};
use rust_eloquent::{Eloquent, EloquentModel, sqlx::{self, FromRow}};
use serde::Serialize;

pub mod migrations;

// 1. Defina o seu modelo de banco de dados usando o ORM rust-eloquent embutido!
#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
}

#[derive(Serialize)]
struct HomeResponse {
    message: String,
    database_status: String,
}

async fn home() -> impl IntoResponse {
    let name = "Rullst";
    
    // Exemplo de uso do ORM: Buscar usuários ativos do banco
    let db_status = match User::all().await {
        Ok(users) => format!("Banco conectado! Total de usuários cadastrados: {}", users.len()),
        Err(e) => format!("Banco offline ou não configurado: {}", e),
    };

    axum::Json(HomeResponse {
        message: format!("Bem-vindo à API REST Rullst: {}", name),
        database_status: db_status,
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Intercepta comandos do Artisan (ex: cargo rullst db:migrate) antes de inicializar o servidor
    rullst::artisan!(crate::migrations::get_migrations());

    // O Rullst inicializa a conexão com o banco de dados especificado em Rullst.toml
    // automaticamente em tempo de execução quando Server::run é chamado!

    let router = routes![
        get("/" => home),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
"#
    } else {
        r#"use rullst::{html, routes, Server, Router, response::{Html, IntoResponse}};
use rust_eloquent::{Eloquent, EloquentModel, sqlx::{self, FromRow}};

pub mod migrations;

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
    // 1. Intercepta comandos do Artisan (ex: cargo rullst db:migrate) antes de inicializar o servidor
    rullst::artisan!(crate::migrations::get_migrations());

    // O Rullst inicializa a conexão com o banco de dados especificado em Rullst.toml
    // automaticamente em tempo de execução quando Server::run é chamado!

    let router = routes![
        get("/" => home),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
"#
    };

    fs::write(path.join("src/main.rs"), main_rs)?;

    println!("{}", format!("✨ Projeto '{}' criado com sucesso!", name).green().bold());
    println!("{}", "Como rodar:".cyan());
    println!("{}", format!("  cd {}", name).cyan());
    println!("{}", "  cargo run".cyan());

    Ok(())
}

// ==========================================
// HELPER FUNCTIONS FOR DATABASE OPERATIONS
// ==========================================

fn run_project_db_command(command: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!("{}", "❌ Erro: Comando deve ser executado na raiz de um projeto Rullst válido.".red().bold());
        std::process::exit(1);
    }

    println!("{}", format!("⏳ Executando 'cargo run -- {}'...", command).cyan().bold());

    let status = std::process::Command::new("cargo")
        .args(&["run", "--", command])
        .status()?;

    if !status.success() {
        println!("{}", format!("❌ Falha ao executar o comando db: {}", command).red().bold());
        std::process::exit(status.code().unwrap_or(1));
    }

    Ok(())
}

fn create_new_migration(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!("{}", "❌ Erro: Comando deve ser executado na raiz de um projeto Rullst válido.".red().bold());
        std::process::exit(1);
    }

    let snake_name = name.to_lowercase().replace("-", "_").trim_start_matches("m").to_string();
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_{}", timestamp, snake_name);
    
    println!("{}", format!("🛠️ Gerando migração Rullst: {}...", file_stem).cyan().bold());

    let migrations_dir = Path::new("src/migrations");
    if !migrations_dir.exists() {
        fs::create_dir_all(migrations_dir)?;
    }

    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));
    let table_name = get_table_name_from_migration(&snake_name);

    let template = format!(
r#"use rust_eloquent::schema::{{Schema, Blueprint, Migration}};
use rust_eloquent::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rust_eloquent::sqlx::Error> {{
        Schema::create("{table_name}", |table| {{
            table.id();
            // Adicione seus campos aqui (ex: table.string("title");)
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rust_eloquent::sqlx::Error> {{
        Schema::drop_if_exists("{table_name}").await
    }}
}}
"#,
        file_stem = file_stem,
        table_name = table_name
    );

    fs::write(&migration_path, template)?;
    println!("{}", format!("✨ Migração em Rust criada em '{}' com sucesso!", migration_path.display()).green().bold());

    regenerate_migrations_mod()?;

    Ok(())
}

fn get_table_name_from_migration(name: &str) -> String {
    let s = name.to_lowercase();
    if s.starts_with("create_") && s.ends_with("_table") {
        s[7..s.len() - 6].to_string()
    } else if s.starts_with("create_") {
        s[7..].to_string()
    } else {
        "table_name".to_string()
    }
}

fn regenerate_migrations_mod() -> Result<(), Box<dyn std::error::Error>> {
    let migrations_dir = Path::new("src/migrations");
    if !migrations_dir.exists() {
        return Ok(());
    }

    let paths = fs::read_dir(migrations_dir)?;
    let mut modules = vec![];
    for path in paths {
        let path = path?.path();
        if let Some(ext) = path.extension() {
            if ext == "rs" {
                if let Some(stem) = path.file_stem() {
                    let stem_str = stem.to_string_lossy().to_string();
                    if stem_str != "mod" && stem_str.starts_with('m') {
                        modules.push(stem_str);
                    }
                }
            }
        }
    }
    modules.sort();

    let mut mod_content = String::new();
    mod_content.push_str("// Generated by Rullst. Do not edit manually.\n\n");
    for m in &modules {
        mod_content.push_str(&format!("pub mod {};\n", m));
    }
    mod_content.push_str("\npub fn get_migrations() -> Vec<Box<dyn rust_eloquent::schema::Migration>> {\n");
    mod_content.push_str("    vec![\n");
    for m in &modules {
        mod_content.push_str(&format!("        Box::new({}::MigrationImpl),\n", m));
    }
    mod_content.push_str("    ]\n");
    mod_content.push_str("}\n");

    fs::write(migrations_dir.join("mod.rs"), mod_content)?;
    Ok(())
}
