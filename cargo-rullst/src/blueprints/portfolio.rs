// src/blueprints/portfolio.rs — Portfolio / Showcase blueprint templates.

pub fn file_manifest(_project_name_safe: &str) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    // 1. src/main.rs
    let main_rs = r##"use rullst::{routes, Server};

pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let router = routes![
        get("/" => controllers::portfolio_controller::index),
    ];

    println!("🚀 AI Portfolio server starting on port 3000...");
    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
"##
    .to_string();
    manifest.push(("src/main.rs", main_rs));

    // 2. Controller
    let portfolio_controller = r##"use rullst::server::IntoResponse;
use rullst::response::Html;
use crate::pages::home;

pub struct Project {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub image: &'static str,
    pub tags: Vec<&'static str>,
}

pub async fn index() -> impl IntoResponse {
    let projects = vec![
        Project {
            id: "neural-engine",
            title: "Neural Engine Cortex",
            description: "A high-performance Rust AI inference engine leveraging hyper-optimized matrix multiplications.",
            image: "https://images.unsplash.com/photo-1620712943543-bcc4688e7485?q=80&w=800&auto=format&fit=crop",
            tags: vec!["Rust", "AI", "CUDA"],
        },
        Project {
            id: "quantum-ui",
            title: "Quantum UI",
            description: "Next-generation glassmorphism component library for building immersive web experiences.",
            image: "https://images.unsplash.com/photo-1550751827-4bd374c3f58b?q=80&w=800&auto=format&fit=crop",
            tags: vec!["HTML/CSS", "Design", "Rullst"],
        },
        Project {
            id: "agentic-swarm",
            title: "Agentic Swarm Framework",
            description: "Distributed autonomous agents communicating via WebSockets for collaborative task execution.",
            image: "https://images.unsplash.com/photo-1451187580459-43490279c0fa?q=80&w=800&auto=format&fit=crop",
            tags: vec!["Rust", "Axum", "WebSockets"],
        },
    ];
    
    Html(home::render(projects))
}
"##;
    manifest.push((
        "src/controllers/portfolio_controller.rs",
        portfolio_controller.to_string(),
    ));

    let controllers_mod = r##"pub mod portfolio_controller;
"##;
    manifest.push(("src/controllers/mod.rs", controllers_mod.to_string()));

    // 3. Pages
    let home_page = r##"use rullst::html;
use crate::controllers::portfolio_controller::Project;

pub fn render(projects: Vec<Project>) -> String {
    html! {
        <html lang="en" class="dark">
            <head>
                <meta charset="UTF-8" />
                <title>"AI Developer Portfolio"</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;600;800&display=swap" rel="stylesheet" />
                <style>
                    "
                    * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
                    body { background: #050505; color: #f3f4f6; min-height: 100vh; overflow-x: hidden; position: relative; }
                    
                    /* Dynamic glowing background */
                    .glow-blob { position: absolute; border-radius: 50%; filter: blur(100px); z-index: -1; animation: float 10s infinite ease-in-out alternate; }
                    .glow-1 { top: 10%; left: 10%; width: 500px; height: 500px; background: rgba(5, 150, 105, 0.15); }
                    .glow-2 { bottom: 10%; right: 10%; width: 600px; height: 600px; background: rgba(249, 115, 22, 0.12); animation-delay: -5s; }
                    .glow-3 { top: 40%; left: 40%; width: 400px; height: 400px; background: rgba(5, 150, 105, 0.1); animation-delay: -2s; }
                    
                    @keyframes float {
                        0% { transform: translate(0, 0) scale(1); }
                        100% { transform: translate(30px, 50px) scale(1.1); }
                    }

                    .container { max-width: 1200px; margin: 0 auto; padding: 4rem 2rem; z-index: 1; }
                    
                    header { text-align: center; margin-bottom: 6rem; margin-top: 4rem; }
                    .badge { background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); padding: 0.5rem 1.5rem; border-radius: 9999px; font-size: 0.85rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.1em; display: inline-block; margin-bottom: 1.5rem; backdrop-filter: blur(10px); }
                    h1 { font-size: 4.5rem; font-weight: 800; line-height: 1.1; margin-bottom: 1.5rem; background: linear-gradient(135deg, #ffffff 0%, #34d399 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
                    p.sub { color: #9ca3af; font-size: 1.25rem; max-width: 600px; margin: 0 auto; line-height: 1.6; }

                    .projects-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(350px, 1fr)); gap: 2.5rem; }
                    
                    .project-card { background: rgba(17, 24, 39, 0.4); backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 24px; overflow: hidden; transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1); cursor: pointer; }
                    .project-card:hover { transform: translateY(-10px); border-color: rgba(5, 150, 105, 0.4); box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4); }
                    .project-img-wrapper { width: 100%; height: 220px; overflow: hidden; }
                    .project-img { width: 100%; height: 100%; object-fit: cover; transition: transform 0.5s ease; }
                    .project-card:hover .project-img { transform: scale(1.05); }
                    
                    .project-content { padding: 2rem; }
                    .project-title { font-size: 1.5rem; font-weight: 600; color: #ffffff; margin-bottom: 0.75rem; }
                    .project-desc { color: #9ca3af; font-size: 1rem; line-height: 1.6; margin-bottom: 1.5rem; }
                    
                    .tags { display: flex; flex-wrap: wrap; gap: 0.5rem; }
                    .tag { background: rgba(249, 115, 22, 0.15); color: #fed7aa; padding: 0.25rem 0.75rem; border-radius: 6px; font-size: 0.8rem; font-weight: 500; border: 1px solid rgba(249, 115, 22, 0.2); }
                    "
                </style>
            </head>
            <body>
                <div class="glow-blob glow-1"></div>
                <div class="glow-blob glow-2"></div>
                <div class="glow-blob glow-3"></div>

                <div class="container">
                    <header>
                        <div class="badge">"Available for Hire"</div>
                        <h1>"Building the Future with AI & Rust"</h1>
                        <p class="sub">"I specialize in high-performance fullstack systems, agentic AI frameworks, and immersive web experiences powered by Rullst."</p>
                    </header>

                    <div class="projects-grid">
                        { rullst::html::RawHtml::new(projects.into_iter().map(|p| html! {
                            <div class="project-card">
                                <div class="project-img-wrapper">
                                    <img class="project-img" src={p.image} alt={p.title} />
                                </div>
                                <div class="project-content">
                                    <h2 class="project-title">{p.title}</h2>
                                    <p class="project-desc">{p.description}</p>
                                    <div class="tags">
                                        { rullst::html::RawHtml::new(p.tags.into_iter().map(|tag| html! {
                                            <span class="tag">{tag}</span>
                                        }).collect::<Vec<_>>().join("")) }
                                    </div>
                                </div>
                            </div>
                        }).collect::<Vec<_>>().join("")) }
                    </div>
                </div>
            </body>
        </html>
    }
}
"##;
    manifest.push(("src/pages/home.rs", home_page.to_string()));

    let pages_mod = r##"pub mod home;
"##;
    manifest.push(("src/pages/mod.rs", pages_mod.to_string()));

    manifest
}
