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
    /// Cria toda a estrutura de autenticação (login, registro, model User, migrations, middlewares e views)
    Auth,
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
        Commands::Auth => {
            scaffold_auth_system()?;
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

fn scaffold_auth_system() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!("{}", "❌ Erro: Comando deve ser executado na raiz de um projeto Rullst válido.".red().bold());
        std::process::exit(1);
    }

    println!("{}", "🛡️  Iniciando scaffolding do sistema de autenticação Rullst...".cyan().bold());

    // 1. Criar Migration do Usuário
    let migrations_dir = Path::new("src/migrations");
    fs::create_dir_all(migrations_dir)?;
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S").to_string();
    let file_stem = format!("m{}_create_users_table", timestamp);
    let migration_path = migrations_dir.join(format!("{}.rs", file_stem));
    
    let migration_template = format!(
r##"use rust_eloquent::schema::{{Schema, Blueprint, Migration}};
use rust_eloquent::async_trait;

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {{
    fn name(&self) -> &'static str {{
        "{file_stem}"
    }}

    async fn up(&self) -> Result<(), rust_eloquent::sqlx::Error> {{
        Schema::create("users", |table| {{
            table.id();
            table.string("name").not_null();
            table.string("email").not_null();
            table.string("password_hash").nullable();
            table.string("oauth_provider").nullable();
            table.string("oauth_id").nullable();
            table.timestamps();
        }}).await
    }}

    async fn down(&self) -> Result<(), rust_eloquent::sqlx::Error> {{
        Schema::drop_if_exists("users").await
    }}
}}
"##,
        file_stem = file_stem
    );
    fs::write(&migration_path, migration_template)?;
    println!("{}", "  ✨ Criada migration da tabela 'users'.".green());
    regenerate_migrations_mod()?;

    // 2. Criar Model do Usuário
    let models_dir = Path::new("src/models");
    fs::create_dir_all(models_dir)?;
    let model_path = models_dir.join("user.rs");
    let model_template = r##"use rust_eloquent::{Eloquent, EloquentModel, sqlx::{self, FromRow}};

#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub password_hash: Option<String>,
    pub oauth_provider: Option<String>,
    pub oauth_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
"##;
    fs::write(&model_path, model_template)?;
    println!("{}", "  ✨ Criado model 'User'.".green());

    let mod_models_path = models_dir.join("mod.rs");
    if !mod_models_path.exists() {
        fs::write(&mod_models_path, "")?;
    }
    let mut mod_models_content = fs::read_to_string(&mod_models_path)?;
    if !mod_models_content.contains("pub mod user;") {
        mod_models_content.push_str("pub mod user;\n");
        fs::write(&mod_models_path, mod_models_content)?;
    }

    // 3. Criar Middleware de Autenticação
    let middlewares_dir = Path::new("src/middlewares");
    fs::create_dir_all(middlewares_dir)?;
    let middleware_path = middlewares_dir.join("auth_middleware.rs");
    let middleware_template = r##"use axum::{
    extract::Request,
    middleware::Next,
    response::{Response, Redirect, IntoResponse},
};

pub async fn auth_middleware(mut req: Request, next: Next) -> Response {
    let headers = req.headers();
    
    // 1. Extrai o cookie de sessão criptografado
    if let Some(cookie) = rullst::auth::extract_session_cookie(headers) {
        let app_key = rullst::auth::get_app_key();
        
        // 2. Descriptografa o user_id
        if let Ok(user_id) = rullst::auth::decrypt_session(&cookie, &app_key) {
            // 3. Insere o user_id nas extensions da requisição para acesso nos controllers
            req.extensions_mut().insert(user_id);
            return next.run(req).await;
        }
    }
    
    // 4. Redireciona para o login se não estiver autenticado
    Redirect::to("/login").into_response()
}
"##;
    fs::write(&middleware_path, middleware_template)?;
    println!("{}", "  ✨ Criado middleware 'auth_middleware'.".green());

    let mod_middlewares_path = middlewares_dir.join("mod.rs");
    if !mod_middlewares_path.exists() {
        fs::write(&mod_middlewares_path, "")?;
    }
    let mut mod_middlewares_content = fs::read_to_string(&mod_middlewares_path)?;
    if !mod_middlewares_content.contains("pub mod auth_middleware;") {
        mod_middlewares_content.push_str("pub mod auth_middleware;\n");
        fs::write(&mod_middlewares_path, mod_middlewares_content)?;
    }

    // 4. Criar Telas HTML (Pages)
    let pages_dir = Path::new("src/pages");
    fs::create_dir_all(pages_dir)?;
    let pages_path = pages_dir.join("auth.rs");
    let pages_template = r##"use rullst::html;
use axum::response::Html;

pub fn login_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = if let Some(err) = error {
        html! {
            <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
                {err}
            </div>
        }
    } else {
        String::new()
    };

    Html(html! {
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <title>"Entrar - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <style>
                    "
                    body {
                        background-color: #0b0f19;
                        color: #f1f5f9;
                        font-family: system-ui, -apple-system, sans-serif;
                        margin: 0;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        min-height: 100vh;
                        padding: 1rem;
                        box-sizing: border-box;
                    }
                    .card {
                        background: #111827;
                        border: 1px solid #1f2937;
                        border-radius: 1rem;
                        padding: 2.5rem;
                        width: 100%;
                        max-width: 420px;
                        box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
                        text-align: center;
                    }
                    h1 {
                        font-size: 2rem;
                        margin: 0 0 0.5rem 0;
                        background: linear-gradient(135deg, #38bdf8, #818cf8);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        font-weight: 800;
                    }
                    p.subtitle {
                        color: #64748b;
                        font-size: 0.95rem;
                        margin: 0 0 2rem 0;
                    }
                    .form-group {
                        margin-bottom: 1.25rem;
                        text-align: left;
                    }
                    label {
                        display: block;
                        font-size: 0.85rem;
                        color: #94a3b8;
                        margin-bottom: 0.5rem;
                        font-weight: 500;
                    }
                    input[type='email'], input[type='password'] {
                        width: 100%;
                        box-sizing: border-box;
                        background: #1f2937;
                        border: 1px solid #374151;
                        border-radius: 0.5rem;
                        padding: 0.75rem 1rem;
                        color: #fff;
                        font-size: 0.95rem;
                        transition: border-color 0.2s, box-shadow 0.2s;
                    }
                    input[type='email']:focus, input[type='password']:focus {
                        outline: none;
                        border-color: #6366f1;
                        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
                    }
                    button.btn-primary {
                        width: 100%;
                        background: linear-gradient(135deg, #6366f1, #4f46e5);
                        color: #fff;
                        border: none;
                        border-radius: 0.5rem;
                        padding: 0.85rem;
                        font-size: 0.95rem;
                        font-weight: 600;
                        cursor: pointer;
                        transition: transform 0.1s, opacity 0.2s;
                        margin-top: 0.5rem;
                    }
                    button.btn-primary:hover {
                        opacity: 0.9;
                        transform: translateY(-1px);
                    }
                    .divider {
                        display: flex;
                        align-items: center;
                        color: #475569;
                        font-size: 0.8rem;
                        margin: 1.5rem 0;
                    }
                    .divider::before, .divider::after {
                        content: '';
                        flex: 1;
                        border-bottom: 1px solid #1f2937;
                    }
                    .divider:not(:empty)::before { margin-right: .5em; }
                    .divider:not(:empty)::after { margin-left: .5em; }
                    .oauth-btn {
                        width: 100%;
                        background: #1f2937;
                        color: #fff;
                        border: 1px solid #374151;
                        border-radius: 0.5rem;
                        padding: 0.75rem;
                        font-size: 0.9rem;
                        font-weight: 500;
                        cursor: pointer;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        gap: 0.5rem;
                        transition: background-color 0.2s;
                        text-decoration: none;
                        box-sizing: border-box;
                    }
                    .oauth-btn:hover {
                        background: #374151;
                    }
                    .footer-link {
                        margin-top: 1.5rem;
                        font-size: 0.85rem;
                        color: #94a3b8;
                    }
                    .footer-link a {
                        color: #38bdf8;
                        text-decoration: none;
                    }
                    .footer-link a:hover {
                        text-decoration: underline;
                    }
                    "
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Bem-vindo de volta"</h1>
                    <p class="subtitle">"Entre na sua conta Rullst"</p>
                    
                    { rullst::html::RawHtml(error_html) }

                    <form method="post" action="/login">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label htmlFor="email">"E-mail"</label>
                            <input type="email" id="email" name="email" placeholder="seu@email.com" required="required" />
                        </div>
                        <div class="form-group">
                            <label htmlFor="password">"Senha"</label>
                            <input type="password" id="password" name="password" placeholder="••••••••" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Entrar"</button>
                    </form>

                    <div class="divider">"ou continuar com"</div>

                    <a href="/auth/github/redirect" class="oauth-btn">
                        <svg style="width: 1.25rem; height: 1.25rem; fill: currentColor;" viewBox="0 0 24 24">
                            <path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z" />
                        </svg>
                        "GitHub"
                    </a>

                    <div class="footer-link">
                        "Não tem uma conta? "
                        <a href="/register">"Cadastre-se"</a>
                    </div>
                </div>
            </body>
        </html>
    })
}

pub fn register_page(csrf_token: &str, error: Option<&str>) -> Html<String> {
    let error_html = if let Some(err) = error {
        html! {
            <div style="background: rgba(239, 68, 68, 0.1); border: 1px solid rgba(239, 68, 68, 0.2); color: #f87171; padding: 0.75rem 1rem; border-radius: 0.5rem; margin-bottom: 1.5rem; font-size: 0.9rem; text-align: left;">
                {err}
            </div>
        }
    } else {
        String::new()
    };

    Html(html! {
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <title>"Criar Conta - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <style>
                    "
                    body {
                        background-color: #0b0f19;
                        color: #f1f5f9;
                        font-family: system-ui, -apple-system, sans-serif;
                        margin: 0;
                        display: flex;
                        align-items: center;
                        justify-content: center;
                        min-height: 100vh;
                        padding: 1rem;
                        box-sizing: border-box;
                    }
                    .card {
                        background: #111827;
                        border: 1px solid #1f2937;
                        border-radius: 1rem;
                        padding: 2.5rem;
                        width: 100%;
                        max-width: 420px;
                        box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
                        text-align: center;
                    }
                    h1 {
                        font-size: 2rem;
                        margin: 0 0 0.5rem 0;
                        background: linear-gradient(135deg, #38bdf8, #818cf8);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        font-weight: 800;
                    }
                    p.subtitle {
                        color: #64748b;
                        font-size: 0.95rem;
                        margin: 0 0 2rem 0;
                    }
                    .form-group {
                        margin-bottom: 1.25rem;
                        text-align: left;
                    }
                    label {
                        display: block;
                        font-size: 0.85rem;
                        color: #94a3b8;
                        margin-bottom: 0.5rem;
                        font-weight: 500;
                    }
                    input[type='text'], input[type='email'], input[type='password'] {
                        width: 100%;
                        box-sizing: border-box;
                        background: #1f2937;
                        border: 1px solid #374151;
                        border-radius: 0.5rem;
                        padding: 0.75rem 1rem;
                        color: #fff;
                        font-size: 0.95rem;
                        transition: border-color 0.2s, box-shadow 0.2s;
                    }
                    input[type='text']:focus, input[type='email']:focus, input[type='password']:focus {
                        outline: none;
                        border-color: #6366f1;
                        box-shadow: 0 0 0 2px rgba(99, 102, 241, 0.2);
                    }
                    button.btn-primary {
                        width: 100%;
                        background: linear-gradient(135deg, #6366f1, #4f46e5);
                        color: #fff;
                        border: none;
                        border-radius: 0.5rem;
                        padding: 0.85rem;
                        font-size: 0.95rem;
                        font-weight: 600;
                        cursor: pointer;
                        transition: transform 0.1s, opacity 0.2s;
                        margin-top: 0.5rem;
                    }
                    button.btn-primary:hover {
                        opacity: 0.9;
                        transform: translateY(-1px);
                    }
                    .footer-link {
                        margin-top: 1.5rem;
                        font-size: 0.85rem;
                        color: #94a3b8;
                    }
                    .footer-link a {
                        color: #38bdf8;
                        text-decoration: none;
                    }
                    .footer-link a:hover {
                        text-decoration: underline;
                    }
                    "
                </style>
            </head>
            <body>
                <div class="card">
                    <h1>"Crie sua conta"</h1>
                    <p class="subtitle">"Cadastre-se e aproveite o Rullst"</p>
                    
                    { rullst::html::RawHtml(error_html) }

                    <form method="post" action="/register">
                        <input type="hidden" name="_token" value={csrf_token} />
                        <div class="form-group">
                            <label htmlFor="name">"Nome Completo"</label>
                            <input type="text" id="name" name="name" placeholder="Seu Nome" required="required" />
                        </div>
                        <div class="form-group">
                            <label htmlFor="email">"E-mail"</label>
                            <input type="email" id="email" name="email" placeholder="seu@email.com" required="required" />
                        </div>
                        <div class="form-group">
                            <label htmlFor="password">"Senha"</label>
                            <input type="password" id="password" name="password" placeholder="Mínimo 6 caracteres" required="required" />
                        </div>
                        <button type="submit" class="btn-primary">"Registrar"</button>
                    </form>

                    <div class="footer-link">
                        "Já tem uma conta? "
                        <a href="/login">"Entrar"</a>
                    </div>
                </div>
            </body>
        </html>
    })
}

pub fn dashboard_page(user_name: &str) -> Html<String> {
    Html(html! {
        <html lang="pt-BR">
            <head>
                <meta charset="utf-8" />
                <title>"Painel de Controle - Rullst"</title>
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <style>
                    "
                    body {
                        background-color: #0b0f19;
                        color: #f1f5f9;
                        font-family: system-ui, -apple-system, sans-serif;
                        margin: 0;
                        padding: 2rem;
                        box-sizing: border-box;
                    }
                    .container {
                        max-width: 800px;
                        margin: 4rem auto;
                        background: #111827;
                        border: 1px solid #1f2937;
                        border-radius: 1rem;
                        padding: 3rem;
                        box-shadow: 0 20px 25px -5px rgba(0, 0, 0, 0.5);
                        text-align: center;
                    }
                    h1 {
                        font-size: 2.5rem;
                        margin: 0 0 1rem 0;
                        background: linear-gradient(135deg, #38bdf8, #818cf8);
                        -webkit-background-clip: text;
                        -webkit-text-fill-color: transparent;
                        font-weight: 800;
                    }
                    p.lead {
                        color: #94a3b8;
                        font-size: 1.15rem;
                        line-height: 1.6;
                        margin-bottom: 2rem;
                    }
                    .badge {
                        display: inline-block;
                        padding: 0.5rem 1rem;
                        background: rgba(56, 189, 248, 0.1);
                        border: 1px solid rgba(56, 189, 248, 0.2);
                        color: #38bdf8;
                        border-radius: 9999px;
                        font-weight: 600;
                        font-size: 0.85rem;
                        margin-bottom: 2rem;
                    }
                    .btn-logout {
                        background: linear-gradient(135deg, #ef4444, #dc2626);
                        color: #fff;
                        border: none;
                        border-radius: 0.5rem;
                        padding: 0.75rem 2rem;
                        font-size: 0.95rem;
                        font-weight: 600;
                        cursor: pointer;
                        transition: transform 0.1s, opacity 0.2s;
                        text-decoration: none;
                    }
                    .btn-logout:hover {
                        opacity: 0.9;
                        transform: translateY(-1px);
                    }
                    "
                </style>
            </head>
            <body>
                <div class="container">
                    <span class="badge">"Rullst Autenticação Ativa"</span>
                    <h1>"Olá, "{user_name}"! 👋"</h1>
                    <p class="lead">"Você está em uma área restrita e segura de alta performance. Este painel e toda a sua infraestrutura foram montados automaticamente via CLI."</p>
                    <a href="/logout" class="btn-logout">"Sair da Conta"</a>
                </div>
            </body>
        </html>
    })
}
"##;
    fs::write(&pages_path, pages_template)?;
    println!("{}", "  ✨ Criadas views HTML em 'src/pages/auth.rs'.".green());

    let mod_pages_path = pages_dir.join("mod.rs");
    if !mod_pages_path.exists() {
        fs::write(&mod_pages_path, "")?;
    }
    let mut mod_pages_content = fs::read_to_string(&mod_pages_path)?;
    if !mod_pages_content.contains("pub mod auth;") {
        mod_pages_content.push_str("pub mod auth;\n");
        fs::write(&mod_pages_path, mod_pages_content)?;
    }

    // 5. Criar Auth Controller
    let controllers_dir = Path::new("src/controllers");
    let controller_path = controllers_dir.join("auth_controller.rs");
    let controller_template = r##"use axum::{
    extract::{Form, Query},
    response::{Html, IntoResponse, Redirect, Response},
    http::HeaderMap,
};
use serde::Deserialize;
use crate::models::user::User;
use crate::pages::auth;
use rullst::auth as rullst_auth;

#[derive(Deserialize)]
pub struct RegisterDto {
    pub name: String,
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginDto {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct OAuthCallbackQuery {
    pub code: String,
}

fn get_csrf_token(headers: &HeaderMap) -> String {
    headers.get(axum::http::header::COOKIE)
        .and_then(|v| v.to_str().ok())
        .and_then(|cookie_str| {
            for cookie in cookie_str.split(';') {
                let trimmed = cookie.trim();
                if trimmed.starts_with("rullst_csrf=") {
                    return Some(trimmed["rullst_csrf=".len()..].to_string());
                }
            }
            None
        })
        .unwrap_or_default()
}

pub async fn login_view(headers: HeaderMap) -> impl IntoResponse {
    let token = get_csrf_token(&headers);
    auth::login_page(&token, None)
}

pub async fn login_submit(headers: HeaderMap, Form(payload): Form<LoginDto>) -> Response {
    let token = get_csrf_token(&headers);
    
    let users = match User::all().await {
        Ok(u) => u,
        Err(_) => return auth::login_page(&token, Some("Erro interno ao buscar usuário")).into_response(),
    };
    
    let user = users.into_iter().find(|u| u.email == payload.email);
    
    let Some(u) = user else {
        return auth::login_page(&token, Some("E-mail ou senha incorretos")).into_response();
    };

    let hash = u.password_hash.as_deref().unwrap_or("");
    if !rullst_auth::verify_password(&payload.password, hash) {
        return auth::login_page(&token, Some("E-mail ou senha incorretos")).into_response();
    }

    match rullst_auth::make_login_cookie(u.id) {
        Ok(cookie) => {
            let mut res = Redirect::to("/dashboard").into_response();
            res.headers_mut().append(
                axum::http::header::SET_COOKIE,
                axum::http::HeaderValue::from_str(&cookie).unwrap()
            );
            res
        }
        Err(_) => auth::login_page(&token, Some("Erro ao iniciar sessão")).into_response(),
    }
}

pub async fn register_view(headers: HeaderMap) -> impl IntoResponse {
    let token = get_csrf_token(&headers);
    auth::register_page(&token, None)
}

pub async fn register_submit(headers: HeaderMap, Form(payload): Form<RegisterDto>) -> Response {
    let token = get_csrf_token(&headers);
    
    if payload.password.len() < 6 {
        return auth::register_page(&token, Some("A senha deve ter no mínimo 6 caracteres")).into_response();
    }

    if let Ok(users) = User::all().await {
        if users.iter().any(|u| u.email == payload.email) {
            return auth::register_page(&token, Some("Este endereço de e-mail já está cadastrado")).into_response();
        }
    }

    let hash = match rullst_auth::hash_password(&payload.password) {
        Ok(h) => h,
        Err(_) => return auth::register_page(&token, Some("Erro ao processar senha")).into_response(),
    };

    let mut user = User {
        id: 0,
        name: payload.name,
        email: payload.email,
        password_hash: Some(hash),
        oauth_provider: None,
        oauth_id: None,
        created_at: String::new(),
        updated_at: String::new(),
    };

    if let Err(e) = user.save().await {
        return auth::register_page(&token, Some(&format!("Erro ao criar conta: {}", e))).into_response();
    }

    match rullst_auth::make_login_cookie(user.id) {
        Ok(cookie) => {
            let mut res = Redirect::to("/dashboard").into_response();
            res.headers_mut().append(
                axum::http::header::SET_COOKIE,
                axum::http::HeaderValue::from_str(&cookie).unwrap()
            );
            res
        }
        Err(_) => Redirect::to("/login").into_response(),
    }
}

pub async fn logout() -> Response {
    let cookie = rullst_auth::make_logout_cookie();
    let mut res = Redirect::to("/login").into_response();
    res.headers_mut().append(
        axum::http::header::SET_COOKIE,
        axum::http::HeaderValue::from_str(&cookie).unwrap()
    );
    res
}

pub async fn dashboard(axum::Extension(user_id): axum::Extension<i32>) -> Response {
    if let Ok(users) = User::all().await {
        if let Some(user) = users.into_iter().find(|u| u.id == user_id) {
            return auth::dashboard_page(&user.name).into_response();
        }
    }
    Redirect::to("/login").into_response()
}

pub async fn oauth_github_redirect() -> Response {
    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "dummy_client_id".to_string());
    let redirect_url = std::env::var("GITHUB_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/auth/github/callback".to_string());
    
    if let Some(provider) = rust_socialite::Socialite::driver("github", client_id, String::new(), redirect_url) {
        return Redirect::to(&provider.redirect_url()).into_response();
    }
    
    Redirect::to("/login").into_response()
}

pub async fn oauth_github_callback(Query(query): Query<OAuthCallbackQuery>) -> Response {
    let client_id = std::env::var("GITHUB_CLIENT_ID").unwrap_or_else(|_| "dummy_client_id".to_string());
    let client_secret = std::env::var("GITHUB_CLIENT_SECRET").unwrap_or_else(|_| "dummy_client_secret".to_string());
    let redirect_url = std::env::var("GITHUB_REDIRECT_URL").unwrap_or_else(|_| "http://localhost:3000/auth/github/callback".to_string());

    if let Some(provider) = rust_socialite::Socialite::driver("github", client_id, client_secret, redirect_url) {
        if let Ok(social_user) = provider.get_user(&query.code).await {
            let mut existing_user = None;
            if let Ok(users) = User::all().await {
                existing_user = users.into_iter().find(|u| {
                    u.oauth_provider.as_deref() == Some("github") && u.oauth_id.as_deref() == Some(&social_user.id)
                });
            }

            let user_id = if let Some(u) = existing_user {
                u.id
            } else {
                let mut user = User {
                    id: 0,
                    name: social_user.name.clone().unwrap_or_else(|| "GitHub User".to_string()),
                    email: social_user.email.clone().unwrap_or_else(|| format!("{}@github.com", social_user.id)),
                    password_hash: None,
                    oauth_provider: Some("github".to_string()),
                    oauth_id: Some(social_user.id.clone()),
                    created_at: String::new(),
                    updated_at: String::new(),
                };
                if user.save().await.is_ok() {
                    user.id
                } else {
                    return Redirect::to("/login").into_response();
                }
            };

            if let Ok(cookie) = rullst_auth::make_login_cookie(user_id) {
                let mut res = Redirect::to("/dashboard").into_response();
                res.headers_mut().append(
                    axum::http::header::SET_COOKIE,
                    axum::http::HeaderValue::from_str(&cookie).unwrap()
                );
                return res;
            }
        }
    }

    Redirect::to("/login").into_response()
}
"##;
    fs::write(&controller_path, controller_template)?;
    println!("{}", "  ✨ Criado controller 'src/controllers/auth_controller.rs'.".green());

    let mod_controllers_path = controllers_dir.join("mod.rs");
    if !mod_controllers_path.exists() {
        fs::write(&mod_controllers_path, "")?;
    }
    let mut mod_controllers_content = fs::read_to_string(&mod_controllers_path)?;
    if !mod_controllers_content.contains("pub mod auth_controller;") {
        mod_controllers_content.push_str("pub mod auth_controller;\n");
        fs::write(&mod_controllers_path, mod_controllers_content)?;
    }

    // 6. Registrar módulos em src/main.rs se necessário
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        
        // Registrar módulos necessários se não estiverem presentes
        for module in &["controllers", "models", "middlewares", "pages"] {
            let declaration = format!("pub mod {};", module);
            let alt_declaration = format!("mod {};", module);
            if !main_content.contains(&declaration) && !main_content.contains(&alt_declaration) {
                main_content = format!("pub mod {};\n{}", module, main_content);
            }
        }

        // Tentar injetar automaticamente as dependências necessárias no Cargo.toml do usuário (como o rust-socialite)
        let cargo_toml_path = Path::new("Cargo.toml");
        if cargo_toml_path.exists() {
            let mut cargo_toml_content = fs::read_to_string(cargo_toml_path)?;
            if !cargo_toml_content.contains("rust-socialite") {
                // Tenta achar [dependencies] e injeta a dependência do rust-socialite como caminho local
                // se estivermos na pasta REPOS (procura a pasta rust-socialite no nível irmão)
                let current_dir = std::env::current_dir()?;
                let sibling_path = current_dir.parent().unwrap().join("rust-socialite");
                let dep_str = if sibling_path.exists() {
                    let absolute_path = sibling_path.canonicalize()?.display().to_string().replace("\\", "/");
                    format!("rust-socialite = {{ path = \"{}\" }}\n", absolute_path)
                } else {
                    "rust-socialite = \"0.4.0\"\n".to_string()
                };

                if let Some(pos) = cargo_toml_content.find("[dependencies]") {
                    cargo_toml_content.insert_str(pos + 14, &dep_str);
                    fs::write(cargo_toml_path, cargo_toml_content)?;
                    println!("{}", "  ✨ Adicionada dependência do 'rust-socialite' no seu Cargo.toml.".green());
                }
            }
        }

        fs::write(main_path, main_content)?;
        println!("{}", "  ✨ Injetadas declarações de módulos ('pub mod controllers/models...') no seu src/main.rs.".green());
    }

    println!("\n{}", "🎉 Sistema de autenticação gerado com extremo sucesso!".green().bold());
    println!("{}", "Para concluir a integração:".cyan().bold());
    println!("{}", "  1. Registre as rotas abaixo no macro routes! de seu 'src/main.rs':".cyan());
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "     get(\"/login\" => controllers::auth_controller::login_view),".yellow());
    println!("{}", "     post(\"/login\" => controllers::auth_controller::login_submit),".yellow());
    println!("{}", "     get(\"/register\" => controllers::auth_controller::register_view),".yellow());
    println!("{}", "     post(\"/register\" => controllers::auth_controller::register_submit),".yellow());
    println!("{}", "     get(\"/logout\" => controllers::auth_controller::logout),".yellow());
    println!("{}", "     get(\"/dashboard\" => controllers::auth_controller::dashboard),".yellow());
    println!("{}", "     get(\"/auth/github/redirect\" => controllers::auth_controller::oauth_github_redirect),".yellow());
    println!("{}", "     get(\"/auth/github/callback\" => controllers::auth_controller::oauth_github_callback),".yellow());
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "  2. Para proteger rotas com middleware, aplique a camada no router:".cyan());
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "     let protected_router = routes![\n         get(\"/dashboard\" => controllers::auth_controller::dashboard)\n     ]".yellow());
    println!("{}", "     .layer(axum::middleware::from_fn(middlewares::auth_middleware::auth_middleware));".yellow());
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "  3. Aplique as proteções CSRF e Security Headers globais no seu router principal:".cyan());
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "     let main_router = routes![...]\n         .layer(axum::middleware::from_fn(rullst::security::csrf_middleware))\n         .layer(axum::middleware::from_fn(rullst::security::headers_middleware));".yellow());
    println!("{}", "     -------------------------------------------------------------------------------------".yellow());
    println!("{}", "  4. Execute as migrations:".cyan());
    println!("{}", "     $ cargo rullst db:migrate".yellow());

    Ok(())
}

