// src/generators/cors_jwt.rs — CORS & JWT Middleware generator.
#![cfg_attr(mutants, mutants::skip)]

use crate::generators::is_rullst_project;
use colored::*;
use std::fs;
use std::io::{Error as IoError, ErrorKind};
use std::path::Path;

const CORS_MIDDLEWARE_TEMPLATE: &str = include_str!("cors_middleware.rs.template");
const TOWER_HTTP_CORS_DEPENDENCY: &str = r#"tower-http = { version = "0.7", features = ["cors"] }"#;

fn ensure_tower_http_cors_dependency(
    cargo_toml: &str,
) -> Result<(String, bool), Box<dyn std::error::Error>> {
    let Some(dependencies_start) = cargo_toml.find("[dependencies]") else {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            "Cargo.toml does not contain a [dependencies] table",
        )
        .into());
    };

    let table_content_start = dependencies_start + "[dependencies]".len();
    let dependencies_end = cargo_toml[table_content_start..]
        .find("\n[")
        .map_or(cargo_toml.len(), |offset| table_content_start + offset + 1);
    let dependencies = &cargo_toml[table_content_start..dependencies_end];

    let mut relative_line_start = 0usize;
    for line in dependencies.split_inclusive('\n') {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let declaration = line_without_newline
            .split_once('#')
            .map_or(line_without_newline, |(value, _)| value)
            .trim();
        let Some((key, value)) = declaration.split_once('=') else {
            relative_line_start += line.len();
            continue;
        };
        if key.trim().trim_matches(['\'', '"']) != "tower-http" {
            relative_line_start += line.len();
            continue;
        }

        if value.contains("\"cors\"") || value.contains("'cors'") {
            return Ok((cargo_toml.to_string(), false));
        }

        let line_start = table_content_start + relative_line_start;
        let line_end = line_start + line.len();
        let updated_line = add_cors_feature_to_dependency(line_without_newline)?;
        let mut updated = cargo_toml.to_string();
        updated.replace_range(
            line_start..line_end,
            &format!(
                "{}{}",
                updated_line,
                if line.ends_with('\n') { "\n" } else { "" }
            ),
        );
        return Ok((updated, true));
    }

    let mut updated = cargo_toml.to_string();
    updated.insert_str(
        table_content_start,
        &format!("\n{}", TOWER_HTTP_CORS_DEPENDENCY),
    );
    Ok((updated, true))
}

fn add_cors_feature_to_dependency(line: &str) -> Result<String, Box<dyn std::error::Error>> {
    let (declaration, comment) = line
        .split_once('#')
        .map_or((line, None), |(value, comment)| (value, Some(comment)));
    let Some((key, value)) = declaration.split_once('=') else {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            "invalid tower-http dependency declaration",
        )
        .into());
    };
    let indentation = &line[..line.len() - line.trim_start().len()];
    let value = value.trim();
    let updated_value = if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        format!("{{ version = {value}, features = [\"cors\"] }}")
    } else if value.starts_with('{') && value.ends_with('}') {
        if let Some(features_start) = value.find("features") {
            let Some(relative_end) = value[features_start..].find(']') else {
                return Err(IoError::new(
                    ErrorKind::InvalidData,
                    "tower-http features must use an inline array",
                )
                .into());
            };
            let features_end = features_start + relative_end;
            let features = &value[features_start..features_end];
            let features_are_empty = features
                .split_once('[')
                .is_some_and(|(_, items)| items.trim().is_empty());
            let separator = if features_are_empty { "" } else { ", " };
            format!(
                "{}{}\"cors\"{}",
                &value[..features_end],
                separator,
                &value[features_end..]
            )
        } else {
            let closing_brace = value.len() - 1;
            let separator = if value[..closing_brace].trim_end().ends_with('{') {
                ""
            } else {
                ", "
            };
            format!(
                "{}{}features = [\"cors\"]{}",
                &value[..closing_brace],
                separator,
                &value[closing_brace..]
            )
        }
    } else {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            "tower-http must use a version string or single-line inline table",
        )
        .into());
    };

    let comment = comment.map_or(String::new(), |comment| format!(" #{comment}"));
    Ok(format!(
        "{indentation}{} = {updated_value}{comment}",
        key.trim()
    ))
}

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

    let cargo_toml_path = Path::new("Cargo.toml");
    let cargo_toml = fs::read_to_string(cargo_toml_path)?;
    let (cargo_toml, dependency_added) = ensure_tower_http_cors_dependency(&cargo_toml)?;
    if dependency_added {
        fs::write(cargo_toml_path, cargo_toml)?;
        println!(
            "{}",
            "  ✨ Enabled tower-http's tested CORS implementation in Cargo.toml.".green()
        );
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
        fs::write(&middleware_path, CORS_MIDDLEWARE_TEMPLATE)?;
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
    println!(
        "{}",
        "  1. Set CORS_ALLOWED_ORIGINS=https://app.example.com,https://admin.example.com".cyan()
    );
    println!(
        "{}",
        "  2. Build the layer: 'let cors = middlewares::cors_middleware::cors_layer_from_env()?;'"
            .cyan()
    );
    println!("{}", "  3. Register it: '.layer(cors)'".cyan());

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

    let token = &auth_str["Bearer ".len()..];
    let secret = match std::env::var("JWT_SECRET") {
        Ok(s) => s,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "JWT_SECRET must be set").into_response(),
    };

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
pub fn generate_token(user_id: &str) -> Result<String, String> {
    let secret = std::env::var("JWT_SECRET").map_err(|_| "JWT_SECRET must be set".to_string())?;
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
    ).map_err(|e| e.to_string())
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
        "     .layer(rullst::server::from_fn(middlewares::jwt_middleware::jwt_middleware))".cyan()
    );
    println!("{}", "  2. Acesse os claims no controller:".cyan());
    println!("{}", "     pub async fn meu_endpoint(rullst::server::Extension(claims): rullst::server::Extension<Claims>) -> impl IntoResponse".cyan());

    Ok(())
}

#[cfg(test)]
mod cors_tests {
    use super::*;

    #[test]
    fn injects_cors_dependency_idempotently() {
        let original = r#"[package]
name = "demo"

[dependencies]
rullst = "12.0.0"

[workspace]
"#;

        let (updated, changed) = ensure_tower_http_cors_dependency(original).unwrap();
        assert!(changed);
        assert!(updated.contains(TOWER_HTTP_CORS_DEPENDENCY));

        let (unchanged, changed_again) = ensure_tower_http_cors_dependency(&updated).unwrap();
        assert!(!changed_again);
        assert_eq!(unchanged, updated);
        assert_eq!(unchanged.matches("tower-http").count(), 1);
    }

    #[test]
    fn enables_cors_on_an_existing_tower_http_dependency() {
        let version_only = "[dependencies]\ntower-http = \"0.7\"\n";
        let (updated, changed) = ensure_tower_http_cors_dependency(version_only).unwrap();
        assert!(changed);
        assert!(updated.contains("tower-http = { version = \"0.7\", features = [\"cors\"] }"));

        let inline_table =
            "[dependencies]\ntower-http = { version = \"0.7\", features = [\"trace\"] }\n";
        let (updated, changed) = ensure_tower_http_cors_dependency(inline_table).unwrap();
        assert!(changed);
        assert!(updated.contains("features = [\"trace\", \"cors\"]"));

        let empty_features = "[dependencies]\ntower-http = { version = \"0.7\", features = [ ] }\n";
        let (updated, changed) = ensure_tower_http_cors_dependency(empty_features).unwrap();
        assert!(changed);
        assert!(updated.contains("features = [ \"cors\"]"));
    }

    #[test]
    fn generated_cors_template_is_valid_rust_without_panic_paths() {
        syn::parse_file(CORS_MIDDLEWARE_TEMPLATE).unwrap();
        assert!(!CORS_MIDDLEWARE_TEMPLATE.contains(".unwrap("));
        assert!(!CORS_MIDDLEWARE_TEMPLATE.contains(".expect("));
        assert!(!CORS_MIDDLEWARE_TEMPLATE.contains("panic!("));
        assert!(!CORS_MIDDLEWARE_TEMPLATE.contains("mirror_request"));
        assert!(CORS_MIDDLEWARE_TEMPLATE.contains("CORS_ALLOWED_ORIGINS"));
        assert!(CORS_MIDDLEWARE_TEMPLATE.contains(".allow_credentials(self.allow_credentials)"));
        assert!(CORS_MIDDLEWARE_TEMPLATE.contains(".vary([header::ORIGIN])"));
    }
}
