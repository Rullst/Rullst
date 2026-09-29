use rullst::db::schema::{Schema, Migration};
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
