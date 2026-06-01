// src/generators/cors_jwt.rs — CORS & JWT Middleware generator.

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::path::Path;

pub fn create_cors_middleware() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!("{}", "🛠️ Generating CORS middleware...".cyan().bold());

    let middlewares_dir = Path::new("src/middlewares");
    if !middlewares_dir.exists() {
        fs::create_dir_all(middlewares_dir)?;
    }

    let mod_path = middlewares_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    let mut mod_content = fs::read_to_string(&mod_path)?;
    if !mod_content.contains("pub mod cors_middleware;") {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str("pub mod cors_middleware;\n");
        fs::write(&mod_path, mod_content)?;
    }

    let middleware_path = middlewares_dir.join("cors_middleware.rs");
    if middleware_path.exists() {
        println!(
            "{}",
            "⚠️ Warning: CORS middleware 'cors_middleware.rs' already exists. Skipping creation."
                .yellow()
        );
    } else {
        let template = r#"use rullst::server::{
    Request,
    Next,
    Response,
    header, Method, StatusCode,
    Body,
};

/// Middleware CORS robusto e de alta performance.
/// Gerencia cabeçalhos de compartilhamento de recursos de origem cruzada e requisições preflight (OPTIONS).
pub async fn cors_middleware(req: Request, next: Next) -> Response {
    let origin = req.headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("*")
        .to_string();

    // Lida com requisições preflight OPTIONS
    if req.method() == Method::OPTIONS {
        // ATENÇÃO: Nunca permita credenciais se a origem for '*'
        let allow_credentials = if origin == "*" { "false" } else { "true" };
        
        return Response::builder()
            .status(StatusCode::NO_CONTENT)
            .header(header::ACCESS_CONTROL_ALLOW_ORIGIN, &origin)
            .header(header::ACCESS_CONTROL_ALLOW_METHODS, "GET, POST, PUT, DELETE, PATCH, OPTIONS")
            .header(header::ACCESS_CONTROL_ALLOW_HEADERS, "Content-Type, Authorization, X-Requested-With, X-CSRF-Token")
            .header(header::ACCESS_CONTROL_ALLOW_CREDENTIALS, allow_credentials)
            .header(header::ACCESS_CONTROL_MAX_AGE, "86400")
            .body(Body::empty())
            .unwrap();
    }

    let mut response = next.run(req).await;
    let headers = response.headers_mut();

    let allow_credentials = if origin == "*" { "false" } else { "true" };

    headers.insert(
        header::ACCESS_CONTROL_ALLOW_ORIGIN,
        header::HeaderValue::from_str(&origin).unwrap_or_else(|_| header::HeaderValue::from_static("*")),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_METHODS,
        header::HeaderValue::from_static("GET, POST, PUT, DELETE, PATCH, OPTIONS"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        header::HeaderValue::from_static("Content-Type, Authorization, X-Requested-With, X-CSRF-Token"),
    );
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
        header::HeaderValue::from_str(allow_credentials).unwrap(),
    );

    response
}
"#;
        fs::write(&middleware_path, template)?;
    }

    // Attempt to inject "pub mod middlewares;" into src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod middlewares;")
            && !main_content.contains("mod middlewares;")
        {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace(
                    "pub mod controllers;",
                    "pub mod controllers;\npub mod middlewares;",
                );
            } else if main_content.contains("pub mod models;") {
                main_content = main_content
                    .replace("pub mod models;", "pub mod models;\npub mod middlewares;");
            } else {
                main_content = format!("pub mod middlewares;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Adicionado 'pub mod middlewares;' ao src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        "✨ CORS middleware successfully created!".green().bold()
    );
    println!(
        "{}",
        "How to register in your main router (src/main.rs):".cyan()
    );
    println!("{}", "  1. Add: '.layer(rullst::server::from_fn(middlewares::cors_middleware::cors_middleware))'".cyan());

    Ok(())
}

pub fn create_jwt_middleware() -> Result<(), Box<dyn std::error::Error>> {
    if !is_rullst_project() {
        println!(
            "{}",
            "❌ Error: This command must be executed in the root of a valid Rullst project."
                .red()
                .bold()
        );
        std::process::exit(1);
    }

    println!("{}", "🛠️ Generating JWT middleware...".cyan().bold());

    // 1. Injetar jsonwebtoken e chrono no Cargo.toml do usuário
    let cargo_toml_path = Path::new("Cargo.toml");
    if cargo_toml_path.exists() {
        let mut cargo_toml_content = fs::read_to_string(cargo_toml_path)?;
        let mut modified = false;
        if !cargo_toml_content.contains("jsonwebtoken") {
            if let Some(pos) = cargo_toml_content.find("[dependencies]") {
                cargo_toml_content.insert_str(pos + 14, "jsonwebtoken = \"9.3\"\n");
                modified = true;
            }
        }
        if !cargo_toml_content.contains("chrono") {
            if let Some(pos) = cargo_toml_content.find("[dependencies]") {
                cargo_toml_content.insert_str(
                    pos + 14,
                    "chrono = { version = \"0.4\", features = [\"serde\"] }\n",
                );
                modified = true;
            }
        }
        if modified {
            fs::write(cargo_toml_path, cargo_toml_content)?;
            println!(
                "{}",
                "  ✨ Added 'jsonwebtoken' and 'chrono' dependencies to your Cargo.toml.".green()
            );
        }
    }

    let middlewares_dir = Path::new("src/middlewares");
    if !middlewares_dir.exists() {
        fs::create_dir_all(middlewares_dir)?;
    }

    let mod_path = middlewares_dir.join("mod.rs");
    if !mod_path.exists() {
        fs::write(&mod_path, "")?;
    }

    let mut mod_content = fs::read_to_string(&mod_path)?;
    if !mod_content.contains("pub mod jwt_middleware;") {
        if !mod_content.is_empty() && !mod_content.ends_with('\n') {
            mod_content.push('\n');
        }
        mod_content.push_str("pub mod jwt_middleware;\n");
        fs::write(&mod_path, mod_content)?;
    }

    let middleware_path = middlewares_dir.join("jwt_middleware.rs");
    if middleware_path.exists() {
        println!(
            "{}",
            "⚠️ Warning: JWT middleware 'jwt_middleware.rs' already exists. Skipping creation."
                .yellow()
        );
    } else {
        let template = r#"use rullst::server::{
    Request,
    Next,
    Response, IntoResponse,
    header, StatusCode,
};
use serde::{Deserialize, Serialize};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // Subject (id do usuário)
    pub exp: usize,  // Timestamp de expiração
}

/// JWT Authentication Middleware.
/// Extrai o cabeçalho 'Authorization: Bearer <token>', valida e injeta os claims nas extensões da requisição.
pub async fn jwt_middleware(mut req: Request, next: Next) -> Response {
    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok());

    let Some(auth_str) = auth_header else {
        return (StatusCode::UNAUTHORIZED, "Missing Authorization Header").into_response();
    };

    if !auth_str.starts_with("Bearer ") {
        return (StatusCode::UNAUTHORIZED, "Invalid Authorization Header Format").into_response();
    }

    let token = &auth_str["Bearer ".len()...];
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| panic!("JWT_SECRET must be set"));

    match decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(token_data) => {
            // Insere os claims nas extensões da requisição para acesso nos controllers
            req.extensions_mut().insert(token_data.claims);
            next.run(req).await
        }
        Err(_) => (StatusCode::UNAUTHORIZED, "Invalid or Expired Token").into_response(),
    }
}

/// Helper para gerar um novo token JWT com duração de 1 dia.
pub fn generate_token(user_id: &str) -> Result<String, jsonwebtoken::errors::Error> {
    let secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| panic!("JWT_SECRET must be set"));
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(1))
        .expect("valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id.to_string(),
        exp: expiration,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}
"#;
        fs::write(&middleware_path, template)?;
    }

    // Attempt to inject "pub mod middlewares;" into src/main.rs if needed
    let main_path = Path::new("src/main.rs");
    if main_path.exists() {
        let mut main_content = fs::read_to_string(main_path)?;
        if !main_content.contains("pub mod middlewares;")
            && !main_content.contains("mod middlewares;")
        {
            if main_content.contains("pub mod controllers;") {
                main_content = main_content.replace(
                    "pub mod controllers;",
                    "pub mod controllers;\npub mod middlewares;",
                );
            } else if main_content.contains("pub mod models;") {
                main_content = main_content
                    .replace("pub mod models;", "pub mod models;\npub mod middlewares;");
            } else {
                main_content = format!("pub mod middlewares;\n{}", main_content);
            }
            fs::write(main_path, main_content)?;
            println!(
                "{}",
                "ℹ️ Adicionado 'pub mod middlewares;' ao src/main.rs.".cyan()
            );
        }
    }

    println!(
        "{}",
        "✨ JWT middleware successfully created!".green().bold()
    );
    println!("{}", "How to use:".cyan());
    println!(
        "{}",
        "  1. Add the layer to your protected router (src/main.rs):".cyan()
    );
    println!(
        "{}",
        "     .layer(rullst::server::from_fn(middlewares::jwt_middleware::jwt_middleware))"
            .cyan()
    );
    println!("{}", "  2. Acesse os claims no controller:".cyan());
    println!("{}", "     pub async fn meu_endpoint(rullst::server::Extension(claims): rullst::server::Extension<Claims>) -> impl IntoResponse".cyan());

    Ok(())
}
