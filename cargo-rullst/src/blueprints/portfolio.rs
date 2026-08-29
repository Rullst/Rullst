// src/blueprints/portfolio.rs — Dynamic Full-Stack Portfolio supporting Active Record, Repository, and Hybrid ORM patterns.
use super::common;

pub fn file_manifest(
    project_name_safe: &str,
    hot_reload: bool,
    orm_pattern: &str,
    frontend_engine: &str,
) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    let is_repo_mode = orm_pattern.contains("Repository") || orm_pattern.contains("Hybrid");
    let repo_mod_decl = if is_repo_mode {
        "pub mod repositories;\n"
    } else {
        ""
    };

    // 1. Router & Main Entrypoints
    if hot_reload {
        let lib_rs = format!(
            r##"use rullst::{{routes, Router}};

pub mod migrations;
pub mod models;
{repo_mod_decl}pub mod controllers;
pub mod pages;

pub fn router() -> Result<Router, Box<dyn std::error::Error>> {{
    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("Portfolio CMS Admin")
        .register::<models::profile::Profile>()
        .register::<models::project::Project>()
        .register::<models::experience::Experience>()
        .register::<models::skill::Skill>()
        .try_build()?;

    Ok(routes![
        get("/" => controllers::portfolio_controller::index),
    ].nest_axum("/nexus", nexus))
}}

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {{
    let router = match router() {{
        Ok(router) => router,
        Err(error) => {{
            eprintln!("Nexus startup configuration error: {{error}}");
            Router::new()
        }}
    }};
    Box::into_raw(Box::new(router))
}}
"##,
            repo_mod_decl = repo_mod_decl
        );
        manifest.push(("src/lib.rs", lib_rs));

        let main_rs = format!(
            r##"pub mod migrations;
pub mod models;
{repo_mod_decl}pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    rullst::artisan!(crate::migrations::get_migrations());

    #[cfg(debug_assertions)]
    {{
        rullst::runtime::spawn(async {{
            if let Err(error) = rullst::studio::run_studio(5555).await {{
                eprintln!("Rullst Studio could not start: {{error}}");
            }}
        }});
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }}

    println!("🚀 AI Portfolio server starting on port 3000 (Engine: {frontend_engine}, ORM: {orm_pattern})...");
    println!("⚙️  Nexus CMS: http://127.0.0.1:3000/nexus (local loopback access in debug; environment credentials in release)");
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            format!("target/debug/{{}}", "{project_name_safe}")
        }} else {{
            format!("target/debug/lib{{}}", "{project_name_safe}")
        }};
        rullst::Server::new_hot(&lib_path)
    }} else {{
        let router = {project_name_safe}::router()?;
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"##,
            project_name_safe = project_name_safe,
            frontend_engine = frontend_engine,
            orm_pattern = orm_pattern,
            repo_mod_decl = repo_mod_decl
        );
        manifest.push(("src/main.rs", main_rs));
    } else {
        let main_rs = format!(
            r##"use rullst::{{routes, Server}};

pub mod migrations;
pub mod models;
{repo_mod_decl}pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{
    rullst::artisan!(crate::migrations::get_migrations());

    #[cfg(debug_assertions)]
    {{
        rullst::runtime::spawn(async {{
            if let Err(error) = rullst::studio::run_studio(5555).await {{
                eprintln!("Rullst Studio could not start: {{error}}");
            }}
        }});
        println!("📊 Rullst Studio running on http://127.0.0.1:5555");
    }}

    let nexus_auth = rullst::nexus::NexusAuthPolicy::local_development_or_basic_from_env()?;
    let nexus = rullst::nexus::Nexus::new()
        .with_auth_policy(nexus_auth)
        .with_brand("Portfolio CMS Admin")
        .register::<models::profile::Profile>()
        .register::<models::project::Project>()
        .register::<models::experience::Experience>()
        .register::<models::skill::Skill>()
        .try_build()?;

    let router = routes![
        get("/" => controllers::portfolio_controller::index),
    ].nest_axum("/nexus", nexus);

    println!("🚀 AI Portfolio server starting on port 3000 (Engine: {frontend_engine}, ORM: {orm_pattern})...");
    println!("⚙️  Nexus CMS: http://127.0.0.1:3000/nexus (local loopback access in debug; environment credentials in release)");
    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}}
"##,
            frontend_engine = frontend_engine,
            orm_pattern = orm_pattern,
            repo_mod_decl = repo_mod_decl
        );
        manifest.push(("src/main.rs", main_rs));
    }

    // 2. Migrations
    let migrations_mod = r##"pub mod m20260701000000_create_portfolio_tables;

pub fn get_migrations() -> Vec<Box<dyn rullst::db::schema::Migration>> {
    vec![
        Box::new(m20260701000000_create_portfolio_tables::CreatePortfolioTables),
    ]
}
"##;
    manifest.push(("src/migrations/mod.rs", migrations_mod.to_string()));

    let migration_impl = r##"use rullst::db::schema::{Schema, Migration};
use rullst::db::async_trait;

pub struct CreatePortfolioTables;

#[async_trait]
impl Migration for CreatePortfolioTables {
    fn name(&self) -> &'static str {
        "m20260701000000_create_portfolio_tables"
    }

    async fn up(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::create("profiles", |table| {
            table.id();
            table.string("name").not_null();
            table.string("title").not_null();
            table.string("subtitle").not_null();
            table.string("email").not_null();
            table.string("website").not_null();
            table.string("avatar_url").not_null();
            table.string("github_url").not_null();
            table.string("linkedin_url").not_null();
            table.timestamps();
        }).await?;

        Schema::create("projects", |table| {
            table.id();
            table.string("title").not_null();
            table.string("description").not_null();
            table.string("url").not_null();
            table.string("tags").not_null();
            table.integer("is_featured").not_null();
            table.timestamps();
        }).await?;

        Schema::create("experiences", |table| {
            table.id();
            table.string("role").not_null();
            table.string("company").not_null();
            table.string("period").not_null();
            table.string("description").not_null();
            table.timestamps();
        }).await?;

        Schema::create("skills", |table| {
            table.id();
            table.string("name").not_null();
            table.string("category").not_null();
            table.timestamps();
        }).await?;

        let pool = rullst::db::Orm::pool()?;

        rullst::db::sqlx::query(
            "INSERT INTO profiles (id, name, title, subtitle, email, website, avatar_url, github_url, linkedin_url, created_at, updated_at) VALUES 
             (1, 'Vene Light', 'Senior Rust & AI Systems Engineer', 'Specializing in hyper-concurrent web backends, LLM inference pipelines, and high-throughput Rust architectures.', 'rullst@veneloius.de', 'https://rullst.github.io/', 'https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png', 'https://github.com/Rullst', 'https://linkedin.com', datetime('now'), datetime('now'))"
        ).execute(pool).await?;

        rullst::db::sqlx::query(
            "INSERT INTO projects (id, title, description, url, tags, is_featured, created_at, updated_at) VALUES 
             (1, 'Rullst AI Engine', 'High-performance Rust AI inference engine leveraging hyper-optimized matrix operations.', 'https://github.com/Rullst/Rullst', 'Rust, AI, Tokio', 1, datetime('now'), datetime('now')),
             (2, 'Nexus Auto-CMS', 'Zero-config auto-generated Admin CMS for Rust ORM models.', 'https://github.com/Rullst/Rullst', 'Rust, HTMX, Axum', 1, datetime('now'), datetime('now'))"
        ).execute(pool).await?;

        rullst::db::sqlx::query(
            "INSERT INTO experiences (id, role, company, period, description, created_at, updated_at) VALUES 
             (1, 'Senior Rust Engineer', 'TechNova AI', '2024 - Present', 'Architected a highly concurrent distributed task queue in Rust processing 10k+ jobs per second.', datetime('now'), datetime('now')),
             (2, 'Full-Stack Developer', 'Quantum Systems', '2021 - 2024', 'Built scalable SaaS applications and high-throughput backend services using Rust and TypeScript.', datetime('now'), datetime('now'))"
        ).execute(pool).await?;

        rullst::db::sqlx::query(
            "INSERT INTO skills (id, name, category, created_at, updated_at) VALUES 
             (1, 'Rust', 'Languages', datetime('now'), datetime('now')),
             (2, 'Python', 'Languages', datetime('now'), datetime('now')),
             (3, 'Rullst Framework', 'Frameworks', datetime('now'), datetime('now')),
             (4, 'SQLite / SQLx', 'Database', datetime('now'), datetime('now')),
             (5, 'Docker & K8s', 'DevOps', datetime('now'), datetime('now'))"
        ).execute(pool).await?;

        Ok(())
    }

    async fn down(&self) -> Result<(), rullst_orm::error::RullstError> {
        Schema::drop_if_exists("skills").await?;
        Schema::drop_if_exists("experiences").await?;
        Schema::drop_if_exists("projects").await?;
        Schema::drop_if_exists("profiles").await?;
        Ok(())
    }
}
"##;
    manifest.push((
        "src/migrations/m20260701000000_create_portfolio_tables.rs",
        migration_impl.to_string(),
    ));

    // 3. Models
    let models_mod = r##"pub mod profile;
pub mod project;
pub mod experience;
pub mod skill;
"##;
    manifest.push(("src/models/mod.rs", models_mod.to_string()));

    let profile_model = r##"use rullst::db::{Orm, FromRow, Nexus};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "profiles")]
pub struct Profile {
    pub id: i32,
    pub name: String,
    pub title: String,
    pub subtitle: String,
    pub email: String,
    pub website: String,
    pub avatar_url: String,
    pub github_url: String,
    pub linkedin_url: String,
}
"##;
    manifest.push(("src/models/profile.rs", profile_model.to_string()));

    let project_model = r##"use rullst::db::{Orm, FromRow, Nexus};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "projects")]
pub struct Project {
    pub id: i32,
    pub title: String,
    pub description: String,
    pub url: String,
    pub tags: String,
    pub is_featured: i32,
}
"##;
    manifest.push(("src/models/project.rs", project_model.to_string()));

    let experience_model = r##"use rullst::db::{Orm, FromRow, Nexus};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "experiences")]
pub struct Experience {
    pub id: i32,
    pub role: String,
    pub company: String,
    pub period: String,
    pub description: String,
}
"##;
    manifest.push(("src/models/experience.rs", experience_model.to_string()));

    let skill_model = r##"use rullst::db::{Orm, FromRow, Nexus};

#[derive(Debug, Clone, FromRow, Orm, Nexus)]
#[orm(table = "skills")]
pub struct Skill {
    pub id: i32,
    pub name: String,
    pub category: String,
}
"##;
    manifest.push(("src/models/skill.rs", skill_model.to_string()));

    // 4. Repositories (if Repository Pattern or Hybrid Mode selected)
    if is_repo_mode {
        let repo_mod = r##"pub mod profile_repository;
pub mod project_repository;
pub mod experience_repository;
pub mod skill_repository;
"##;
        manifest.push(("src/repositories/mod.rs", repo_mod.to_string()));

        let profile_repo = r##"use crate::models::profile::Profile;

pub struct ProfileRepository;

impl ProfileRepository {
    pub async fn get() -> Profile {
        Profile::find(1).await.unwrap_or(None).unwrap_or(Profile {
            id: 1,
            name: "Vene Light".to_string(),
            title: "Senior Rust & AI Systems Engineer".to_string(),
            subtitle: "Specializing in hyper-concurrent web backends, LLM inference pipelines, and high-throughput Rust architectures.".to_string(),
            email: "rullst@veneloius.de".to_string(),
            website: "https://rullst.github.io/".to_string(),
            avatar_url: "https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png".to_string(),
            github_url: "https://github.com/Rullst".to_string(),
            linkedin_url: "https://linkedin.com".to_string(),
        })
    }
}
"##;
        manifest.push((
            "src/repositories/profile_repository.rs",
            profile_repo.to_string(),
        ));

        let project_repo = r##"use crate::models::project::Project;

pub struct ProjectRepository;

impl ProjectRepository {
    pub async fn all() -> Vec<Project> {
        Project::all().await.unwrap_or_default()
    }
}
"##;
        manifest.push((
            "src/repositories/project_repository.rs",
            project_repo.to_string(),
        ));

        let exp_repo = r##"use crate::models::experience::Experience;

pub struct ExperienceRepository;

impl ExperienceRepository {
    pub async fn all() -> Vec<Experience> {
        Experience::all().await.unwrap_or_default()
    }
}
"##;
        manifest.push((
            "src/repositories/experience_repository.rs",
            exp_repo.to_string(),
        ));

        let skill_repo = r##"use crate::models::skill::Skill;

pub struct SkillRepository;

impl SkillRepository {
    pub async fn all() -> Vec<Skill> {
        Skill::all().await.unwrap_or_default()
    }
}
"##;
        manifest.push((
            "src/repositories/skill_repository.rs",
            skill_repo.to_string(),
        ));
    }

    // 5. Controller
    let portfolio_controller = if is_repo_mode {
        r##"use rullst::server::IntoResponse;
use rullst::response::Html;
use crate::repositories::profile_repository::ProfileRepository;
use crate::repositories::project_repository::ProjectRepository;
use crate::repositories::experience_repository::ExperienceRepository;
use crate::repositories::skill_repository::SkillRepository;
use crate::pages::home;

pub async fn index() -> impl IntoResponse {
    let profile = ProfileRepository::get().await;
    let projects = ProjectRepository::all().await;
    let experiences = ExperienceRepository::all().await;
    let skills = SkillRepository::all().await;

    Html(home::render(&profile, &projects, &experiences, &skills))
}
"##
        .to_string()
    } else {
        r##"use rullst::server::IntoResponse;
use rullst::response::Html;
use crate::models::profile::Profile;
use crate::models::project::Project;
use crate::models::experience::Experience;
use crate::models::skill::Skill;
use crate::pages::home;

pub async fn index() -> impl IntoResponse {
    let profile = Profile::find(1).await.unwrap_or(None).unwrap_or(Profile {
        id: 1,
        name: "Vene Light".to_string(),
        title: "Senior Rust & AI Systems Engineer".to_string(),
        subtitle: "Specializing in hyper-concurrent web backends, LLM inference pipelines, and high-throughput Rust architectures.".to_string(),
        email: "rullst@veneloius.de".to_string(),
        website: "https://rullst.github.io/".to_string(),
        avatar_url: "https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png".to_string(),
        github_url: "https://github.com/Rullst".to_string(),
        linkedin_url: "https://linkedin.com".to_string(),
    });
    let projects = Project::all().await.unwrap_or_default();
    let experiences = Experience::all().await.unwrap_or_default();
    let skills = Skill::all().await.unwrap_or_default();

    Html(home::render(&profile, &projects, &experiences, &skills))
}
"##.to_string()
    };

    manifest.push((
        "src/controllers/portfolio_controller.rs",
        portfolio_controller,
    ));

    let controllers_mod = r##"pub mod portfolio_controller;
"##;
    manifest.push(("src/controllers/mod.rs", controllers_mod.to_string()));

    // 6. Pages View
    let engine_badge = common::frontend_engine_badge(frontend_engine);
    let engine_imports = "use rullst::html;";
    let render_fn_code = r#"pub fn render(profile: &Profile, projects: &[Project], experiences: &[Experience], skills: &[Skill]) -> String {
    html! {
        <html lang="en">
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>"Rullst Developer — AI & Rust Portfolio"</title>
                <link rel="icon" type="image/png" href="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" />
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&display=swap" rel="stylesheet" />
                <style>{ rullst::html::RawHtml(cv_styles()) }</style>
            </head>
            <body>
                <div class="bg-grid"></div>
                <div class="scanlines"></div>
                <div class="glow-blob glow-1"></div>
                <div class="glow-blob glow-2"></div>
                
                <div class="layout">
                    { rullst::html::RawHtml(render_sidebar(profile, skills)) }
                    { rullst::html::RawHtml(render_content(projects, experiences)) }
                </div>
            </body>
        </html>
    }
}"#;

    let home_page = format!(
        r##"// Frontend Adapter: {frontend_engine}
{engine_imports}
use crate::models::profile::Profile;
use crate::models::project::Project;
use crate::models::experience::Experience;
use crate::models::skill::Skill;

fn cv_styles() -> String {{
    r#"
    * {{ box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }}
    
    :root {{
        --bg-color: #050505;
        --sidebar-bg: rgba(15, 15, 20, 0.6);
        --accent: #00ffcc;
        --accent-glow: rgba(0, 255, 204, 0.2);
        --text-main: #f3f4f6;
        --text-muted: #9ca3af;
        --border-color: rgba(255, 255, 255, 0.08);
        --glass-bg: rgba(25, 25, 30, 0.4);
    }}

    body {{ background: var(--bg-color); color: var(--text-main); line-height: 1.6; }}
    
    .bg-grid {{
        position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: -3;
        background-image: 
            linear-gradient(to right, rgba(255,255,255,0.03) 1px, transparent 1px),
            linear-gradient(to bottom, rgba(255,255,255,0.03) 1px, transparent 1px);
        background-size: 40px 40px;
        mask-image: radial-gradient(circle at center, black, transparent 80%);
        -webkit-mask-image: radial-gradient(circle at center, black, transparent 80%);
        animation: gridMove 20s linear infinite;
    }}
    
    @keyframes gridMove {{
        0% {{ transform: translateY(0); }}
        100% {{ transform: translateY(40px); }}
    }}

    .scanlines {{
        position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: -1;
        background: linear-gradient(to bottom, rgba(255,255,255,0), rgba(255,255,255,0) 50%, rgba(0,0,0,0.15) 50%, rgba(0,0,0,0.15));
        background-size: 100% 4px; pointer-events: none;
    }}

    .glow-blob {{ position: fixed; border-radius: 50%; filter: blur(120px); z-index: -2; animation: pulseGlow 8s infinite alternate; }}
    .glow-1 {{ top: -10%; left: -10%; width: 50vw; height: 50vh; background: rgba(0, 255, 204, 0.08); }}
    .glow-2 {{ bottom: -10%; right: -10%; width: 50vw; height: 50vh; background: rgba(138, 43, 226, 0.08); }}
    
    @keyframes pulseGlow {{
        0% {{ transform: scale(1); opacity: 0.8; }}
        100% {{ transform: scale(1.1); opacity: 1; }}
    }}

    .layout {{ display: flex; min-height: 100vh; max-width: 1400px; margin: 0 auto; padding: 2rem; gap: 3rem; }}
    
    .sidebar {{
        width: 350px; flex-shrink: 0; position: sticky; top: 2rem; height: calc(100vh - 4rem);
        background: var(--sidebar-bg); border: 1px solid var(--border-color); border-radius: 24px;
        padding: 2.5rem; display: flex; flex-direction: column; gap: 2rem;
        backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px);
        box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5); overflow-y: auto;
    }}
    
    .profile-img {{ width: 140px; height: auto; max-height: 120px; border-radius: 12px; margin-bottom: 1rem; object-fit: contain; }}
    h1 {{ font-size: 2.2rem; font-weight: 800; line-height: 1.1; margin-bottom: 0.5rem; background: linear-gradient(135deg, #fff 0%, #aaa 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }}
    h2.role {{ color: var(--accent); font-size: 1.1rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 1rem; }}
    .summary {{ color: var(--text-muted); font-size: 0.95rem; }}

    .contact-info {{ display: flex; flex-direction: column; gap: 1rem; margin-top: 1rem; }}
    .contact-item {{ display: flex; align-items: center; gap: 0.75rem; font-size: 0.9rem; color: var(--text-muted); }}

    .skill-cat {{ font-size: 0.85rem; font-weight: 600; color: #fff; text-transform: uppercase; margin-bottom: 0.5rem; letter-spacing: 0.05em; }}
    .tags {{ display: flex; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 1.5rem; }}
    .tag {{ background: rgba(255, 255, 255, 0.05); color: #ddd; padding: 0.35rem 0.75rem; border-radius: 6px; font-size: 0.8rem; font-weight: 500; border: 1px solid var(--border-color); }}

    .content {{ flex-grow: 1; display: flex; flex-direction: column; gap: 4rem; padding-bottom: 4rem; }}
    .section-title {{ font-size: 2rem; font-weight: 800; display: flex; align-items: center; gap: 1rem; margin-bottom: 2rem; }}

    .timeline {{ position: relative; padding-left: 2rem; }}
    .timeline::before {{ content: ''; position: absolute; left: 0; top: 0; bottom: 0; width: 2px; background: var(--border-color); }}
    
    .timeline-item {{ position: relative; margin-bottom: 3rem; }}
    .timeline-item::before {{
        content: ''; position: absolute; left: -2.35rem; top: 0.3rem; width: 12px; height: 12px;
        border-radius: 50%; background: var(--bg-color); border: 2px solid var(--accent);
    }}
    
    .exp-period {{ display: inline-block; font-size: 0.85rem; color: var(--accent); background: var(--accent-glow); padding: 0.2rem 0.6rem; border-radius: 4px; font-weight: 600; margin-bottom: 0.5rem; }}
    .exp-role {{ font-size: 1.3rem; font-weight: 700; margin-bottom: 0.2rem; }}
    .exp-company {{ font-size: 1rem; color: #bbb; font-weight: 500; margin-bottom: 1rem; }}
    .exp-desc {{ color: var(--text-muted); font-size: 1rem; }}

    .projects-grid {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; }}
    .project-card {{ background: var(--glass-bg); border: 1px solid var(--border-color); border-radius: 16px; padding: 1.5rem; }}
    .project-title {{ font-size: 1.2rem; font-weight: 700; margin-bottom: 0.5rem; }}
    .project-desc {{ font-size: 0.95rem; color: var(--text-muted); margin-bottom: 1.5rem; }}
    .project-link {{ display: inline-flex; align-items: center; gap: 0.5rem; color: var(--accent); text-decoration: none; font-size: 0.9rem; font-weight: 600; }}

    .cms-btn {{ display: inline-block; margin-top: 1rem; background: #10b981; color: #000; padding: 0.6rem 1.2rem; border-radius: 8px; font-weight: 700; text-decoration: none; font-size: 0.9rem; }}
    .cms-btn:hover {{ background: #34d399; }}

    .engine-badge {{ display: inline-block; background: rgba(0, 255, 204, 0.1); border: 1px solid rgba(0, 255, 204, 0.3); color: #00ffcc; font-size: 0.75rem; font-weight: 600; padding: 0.25rem 0.6rem; border-radius: 20px; margin-top: 0.5rem; }}
    "#.to_string()
}}

fn render_sidebar(profile: &Profile, skills: &[Skill]) -> String {{
    html! {{
        <aside class="sidebar">
            <div style="text-align: center;">
                <img src={{&profile.avatar_url}} alt={{&profile.name}} class="profile-img" />
                <h1>{{&profile.name}}</h1>
                <h2 class="role">{{&profile.title}}</h2>
                <div class="engine-badge">"{engine_badge}"</div>
                <p class="summary">{{&profile.subtitle}}</p>
                <a href="/nexus" target="_blank" class="cms-btn">"⚙️ Manage via Nexus CMS"</a>
                <a href="http://127.0.0.1:5555" target="_blank" class="cms-btn">"📊 Open local Studio"</a>
            </div>
            
            <div class="contact-info">
                <div class="contact-item">"📧 "{{&profile.email}}</div>
                <div class="contact-item">"🌐 "<a href={{&profile.website}} target="_blank" style="color: var(--accent);">{{&profile.website}}</a></div>
                <div class="contact-item">"💻 "<a href={{&profile.github_url}} target="_blank" style="color: var(--text-muted);">{{&profile.github_url}}</a></div>
                <div class="contact-item">"💼 "<a href={{&profile.linkedin_url}} target="_blank" style="color: var(--text-muted);">{{&profile.linkedin_url}}</a></div>
            </div>

            <div>
                <div class="skill-cat">"Technical Skills"</div>
                <div class="tags">
                    {{ rullst::html::RawHtml::new(skills.iter().map(|s| format!("<span class=\"tag\">{{}}</span>", s.name)).collect::<Vec<_>>().join("")) }}
                </div>
            </div>
        </aside>
    }}
}}

fn render_content(projects: &[Project], experiences: &[Experience]) -> String {{
    html! {{
        <main class="content">
            <section>
                <h2 class="section-title">"Experience"</h2>
                <div class="timeline">
                    {{ rullst::html::RawHtml::new(experiences.iter().map(|e| format!(
                        "<div class=\"timeline-item\">\
                            <div class=\"exp-period\">{{}}</div>\
                            <h3 class=\"exp-role\">{{}}</h3>\
                            <div class=\"exp-company\">{{}}</div>\
                            <p class=\"exp-desc\">{{}}</p>\
                        </div>", e.period, e.role, e.company, e.description
                    )).collect::<Vec<_>>().join("")) }}
                </div>
            </section>

            <section>
                <h2 class="section-title">"Projects Showcase"</h2>
                <div class="projects-grid">
                    {{ rullst::html::RawHtml::new(projects.iter().map(|p| format!(
                        "<div class=\"project-card\">\
                            <h3 class=\"project-title\">{{}}</h3>\
                            <p class=\"project-desc\">{{}}</p>\
                            <div class=\"tags\"><span class=\"tag\">{{}}</span></div>\
                            <a href=\"{{}}\" target=\"_blank\" class=\"project-link\">View Project &rarr;</a>\
                        </div>",
                        p.title, p.description, p.tags, p.url
                    )).collect::<Vec<_>>().join("")) }}
                </div>
            </section>
        </main>
    }}
}}

{render_fn_code}
"##,
        frontend_engine = frontend_engine,
        engine_imports = engine_imports,
        engine_badge = engine_badge,
        render_fn_code = render_fn_code
    );
    manifest.push(("src/pages/home.rs", home_page));

    let pages_mod = r##"pub mod home;
"##;
    manifest.push(("src/pages/mod.rs", pages_mod.to_string()));

    manifest
}
