// src/blueprints/portfolio.rs — Portfolio / Showcase blueprint templates.

pub fn file_manifest(project_name_safe: &str, hot_reload: bool) -> Vec<(&'static str, String)> {
    let mut manifest = Vec::new();

    // 1. src/main.rs
    if hot_reload {
        let lib_rs = r##"use rullst::{routes, Router};

pub mod controllers;
pub mod pages;

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {
    let router = routes![
        get("/" => controllers::portfolio_controller::index),
    ];
    Box::into_raw(Box::new(router))
}
"##
        .to_string();
        manifest.push(("src/lib.rs", lib_rs));

        let main_rs = format!(
            r##"pub mod controllers;
pub mod pages;

#[rullst::runtime::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {{


    println!("🚀 AI Portfolio server starting on port 3000...");
    let is_hot = std::env::var("HOT_RELOAD").is_ok();

    let server = if is_hot {{
        let lib_path = if cfg!(target_os = "windows") {{
            format!("target/debug/{{}}", "{project_name_safe}")
        }} else {{
            format!("target/debug/lib{{}}", "{project_name_safe}")
        }};
        rullst::Server::new_hot(&lib_path)
    }} else {{
        let router_ptr = {project_name_safe}::rullst_router_init();
        let router = unsafe {{ *Box::from_raw(router_ptr) }};
        rullst::Server::new(router)
    }};

    server.run(3000).await?;

    Ok(())
}}
"##,
            project_name_safe = project_name_safe
        );
        manifest.push(("src/main.rs", main_rs));
    } else {
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
    }

    // 2. Controller
    let portfolio_controller = r##"use rullst::server::IntoResponse;
use rullst::response::Html;
use crate::pages::home;

pub struct Experience {
    pub role: &'static str,
    pub company: &'static str,
    pub period: &'static str,
    pub description: &'static str,
}

pub struct Education {
    pub degree: &'static str,
    pub institution: &'static str,
    pub year: &'static str,
}

pub struct SkillGroup {
    pub category: &'static str,
    pub skills: Vec<&'static str>,
}

pub struct Project {
    pub title: &'static str,
    pub description: &'static str,
    pub tags: Vec<&'static str>,
    pub link: &'static str,
}

pub struct CvData {
    pub name: &'static str,
    pub title: &'static str,
    pub email: &'static str,
    pub github: &'static str,
    pub linkedin: &'static str,
    pub summary: &'static str,
    pub experiences: Vec<Experience>,
    pub education: Vec<Education>,
    pub skill_groups: Vec<SkillGroup>,
    pub projects: Vec<Project>,
}

pub async fn index() -> impl IntoResponse {
    let data = CvData {
        name: "Rullst Developer",
        title: "Senior AI & Rust Engineer",
        email: "hello@example.com",
        github: "github.com/Rullst",
        linkedin: "linkedin.com/in/Rullst",
        summary: "Specialized in high-performance fullstack systems, agentic AI frameworks, and immersive web experiences powered by Rust. I build reliable distributed systems that scale effortlessly.",
        experiences: vec![
            Experience {
                role: "Senior Rullst Engineer",
                company: "TechNova AI",
                period: "2024 - Present",
                description: "Architected a highly concurrent distributed task queue in Rust processing 10k+ jobs per second. Migrated legacy microservices to Rullst framework, reducing memory footprint by 80%.",
            },
            Experience {
                role: "Junior Rullst Developer",
                company: "Quantum Startup",
                period: "2021 - 2024",
                description: "Built end-to-end SAAS products using modern web technologies. Led a team of 4 engineers and implemented CI/CD pipelines.",
            },
        ],
        education: vec![
            Education {
                degree: "Rullst School",
                institution: "Tech University",
                year: "2021",
            },
            Education {
                degree: "Rullst College",
                institution: "State College",
                year: "2019",
            },
        ],
        skill_groups: vec![
            SkillGroup {
                category: "Languages",
                skills: vec!["Rust", "TypeScript", "Python", "Go"],
            },
            SkillGroup {
                category: "Frameworks & Tools",
                skills: vec!["Rullst", "Axum", "Tokio", "Docker", "PostgreSQL"],
            },
        ],
        projects: vec![
            Project {
                title: "Rullst SAAS",
                description: "A high-performance Rust AI inference engine leveraging hyper-optimized matrix multiplications.",
                tags: vec!["Rust", "AI", "CUDA"],
                link: "#",
            },
            Project {
                title: "Rullst LMS",
                description: "Distributed autonomous agents communicating via WebSockets for collaborative task execution.",
                tags: vec!["WebSockets", "Axum", "Rullst"],
                link: "#",
            },
        ],
    };
    
    Html(home::render(data))
}
"##;
    manifest.push((
        "src/controllers/portfolio_controller.rs",
        portfolio_controller.to_string(),
    ));

    let controllers_mod = r##"pub mod portfolio_controller;
"##;
    manifest.push(("src/controllers/mod.rs", controllers_mod.to_string()));

    let home_page = r##"use rullst::html;
use crate::controllers::portfolio_controller::{CvData, Experience, Education, SkillGroup, Project};

fn cv_styles() -> String {
    r#"
    * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
    
    :root {
        --bg-color: #050505;
        --sidebar-bg: rgba(15, 15, 20, 0.6);
        --accent: #00ffcc;
        --accent-glow: rgba(0, 255, 204, 0.2);
        --text-main: #f3f4f6;
        --text-muted: #9ca3af;
        --border-color: rgba(255, 255, 255, 0.08);
        --glass-bg: rgba(25, 25, 30, 0.4);
    }

    body { background: var(--bg-color); color: var(--text-main); line-height: 1.6; }
    
    /* Cyber Grid Background */
    .bg-grid {
        position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: -3;
        background-image: 
            linear-gradient(to right, rgba(255,255,255,0.03) 1px, transparent 1px),
            linear-gradient(to bottom, rgba(255,255,255,0.03) 1px, transparent 1px);
        background-size: 40px 40px;
        mask-image: radial-gradient(circle at center, black, transparent 80%);
        -webkit-mask-image: radial-gradient(circle at center, black, transparent 80%);
        animation: gridMove 20s linear infinite;
    }
    
    @keyframes gridMove {
        0% { transform: translateY(0); }
        100% { transform: translateY(40px); }
    }

    /* Scanlines */
    .scanlines {
        position: fixed; top: 0; left: 0; width: 100vw; height: 100vh; z-index: -1;
        background: linear-gradient(to bottom, rgba(255,255,255,0), rgba(255,255,255,0) 50%, rgba(0,0,0,0.15) 50%, rgba(0,0,0,0.15));
        background-size: 100% 4px; pointer-events: none;
    }

    .glow-blob { position: fixed; border-radius: 50%; filter: blur(120px); z-index: -2; animation: pulseGlow 8s infinite alternate; }
    .glow-1 { top: -10%; left: -10%; width: 50vw; height: 50vh; background: rgba(0, 255, 204, 0.08); }
    .glow-2 { bottom: -10%; right: -10%; width: 50vw; height: 50vh; background: rgba(138, 43, 226, 0.08); }
    
    @keyframes pulseGlow {
        0% { transform: scale(1); opacity: 0.8; }
        100% { transform: scale(1.1); opacity: 1; }
    }

    .layout { display: flex; min-height: 100vh; max-width: 1400px; margin: 0 auto; padding: 2rem; gap: 3rem; }
    
    /* Sidebar */
    .sidebar {
        width: 350px; flex-shrink: 0; position: sticky; top: 2rem; height: calc(100vh - 4rem);
        background: var(--sidebar-bg); border: 1px solid var(--border-color); border-radius: 24px;
        padding: 2.5rem; display: flex; flex-direction: column; gap: 2rem;
        backdrop-filter: blur(20px); -webkit-backdrop-filter: blur(20px);
        box-shadow: 0 25px 50px -12px rgba(0,0,0,0.5); overflow-y: auto;
    }
    
    .sidebar::-webkit-scrollbar { width: 4px; }
    .sidebar::-webkit-scrollbar-thumb { background: var(--border-color); border-radius: 4px; }

    .profile-img { width: 160px; height: auto; max-height: 120px; border-radius: 12px; margin-bottom: 1rem; object-fit: contain; }
    h1 { font-size: 2.2rem; font-weight: 800; line-height: 1.1; margin-bottom: 0.5rem; background: linear-gradient(135deg, #fff 0%, #aaa 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
    h2.role { color: var(--accent); font-size: 1.1rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.05em; margin-bottom: 1rem; }
    .summary { color: var(--text-muted); font-size: 0.95rem; }

    .contact-info { display: flex; flex-direction: column; gap: 1rem; margin-top: 1rem; }
    .contact-item { display: flex; align-items: center; gap: 0.75rem; font-size: 0.9rem; color: var(--text-muted); transition: color 0.2s; }
    .contact-item:hover { color: var(--accent); cursor: pointer; }
    .contact-icon { width: 20px; height: 20px; opacity: 0.8; }

    .skill-cat { font-size: 0.85rem; font-weight: 600; color: #fff; text-transform: uppercase; margin-bottom: 0.5rem; letter-spacing: 0.05em; }
    .tags { display: flex; flex-wrap: wrap; gap: 0.5rem; margin-bottom: 1.5rem; }
    .tag { background: rgba(255, 255, 255, 0.05); color: #ddd; padding: 0.35rem 0.75rem; border-radius: 6px; font-size: 0.8rem; font-weight: 500; border: 1px solid var(--border-color); transition: all 0.3s; }
    .tag:hover { border-color: var(--accent); color: var(--accent); background: var(--accent-glow); box-shadow: 0 0 10px var(--accent-glow); }

    /* Main Content */
    .content { flex-grow: 1; display: flex; flex-direction: column; gap: 4rem; padding-bottom: 4rem; }
    
    .section-title { font-size: 2rem; font-weight: 800; display: flex; align-items: center; gap: 1rem; margin-bottom: 2rem; }
    .section-title::after { content: ''; flex-grow: 1; height: 1px; background: linear-gradient(90deg, var(--border-color), transparent); }

    /* Timeline */
    .timeline { position: relative; padding-left: 2rem; }
    .timeline::before { content: ''; position: absolute; left: 0; top: 0; bottom: 0; width: 2px; background: var(--border-color); }
    
    .timeline-item { position: relative; margin-bottom: 3rem; }
    .timeline-item::before {
        content: ''; position: absolute; left: -2.35rem; top: 0.3rem; width: 12px; height: 12px;
        border-radius: 50%; background: var(--bg-color); border: 2px solid var(--accent);
        transition: all 0.3s ease; box-shadow: 0 0 0 4px var(--bg-color);
    }
    .timeline-item:hover::before { background: var(--accent); box-shadow: 0 0 15px var(--accent); }
    
    .exp-period { display: inline-block; font-size: 0.85rem; color: var(--accent); background: var(--accent-glow); padding: 0.2rem 0.6rem; border-radius: 4px; font-weight: 600; margin-bottom: 0.5rem; }
    .exp-role { font-size: 1.3rem; font-weight: 700; margin-bottom: 0.2rem; }
    .exp-company { font-size: 1rem; color: #bbb; font-weight: 500; margin-bottom: 1rem; }
    .exp-desc { color: var(--text-muted); font-size: 1rem; }

    /* Project Cards */
    .projects-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 1.5rem; }
    .project-card { 
        background: var(--glass-bg); border: 1px solid var(--border-color); border-radius: 16px; 
        padding: 1.5rem; transition: all 0.3s ease; position: relative; overflow: hidden;
    }
    .project-card::before {
        content: ''; position: absolute; top: 0; left: 0; width: 100%; height: 2px;
        background: linear-gradient(90deg, transparent, var(--accent), transparent);
        transform: translateX(-100%); transition: transform 0.6s ease;
    }
    .project-card:hover { transform: translateY(-5px); border-color: rgba(0, 255, 204, 0.3); background: rgba(25, 25, 30, 0.6); }
    .project-card:hover::before { transform: translateX(100%); }
    
    .project-title { font-size: 1.2rem; font-weight: 700; margin-bottom: 0.5rem; }
    .project-desc { font-size: 0.95rem; color: var(--text-muted); margin-bottom: 1.5rem; }
    .project-link { display: inline-flex; align-items: center; gap: 0.5rem; color: var(--text-main); text-decoration: none; font-size: 0.9rem; font-weight: 600; transition: color 0.2s; }
    .project-link:hover { color: var(--accent); }

    @media (max-width: 900px) {
        .layout { flex-direction: column; padding: 1rem; gap: 2rem; }
        .sidebar { width: 100%; position: relative; height: auto; top: 0; }
    }
    "#.to_string()
}

fn render_sidebar(data: &CvData) -> String {
    html! {
        <aside class="sidebar">
            <div style="text-align: center;">
                <img src="https://raw.githubusercontent.com/venelouis/Rullst/main/Rullst.png" alt="Rullst Logo" class="profile-img" />
                <h1>{data.name}</h1>
                <h2 class="role">{data.title}</h2>
                <p class="summary">{data.summary}</p>
            </div>
            
            <div class="contact-info">
                <div class="contact-item">
                    <svg class="contact-icon" fill="none" stroke="currentColor" viewBox="0 0 24 24"><path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M3 8l7.89 5.26a2 2 0 002.22 0L21 8M5 19h14a2 2 0 002-2V7a2 2 0 00-2-2H5a2 2 0 00-2 2v10a2 2 0 002 2z"></path></svg>
                    {data.email}
                </div>
                <div class="contact-item">
                    <svg class="contact-icon" fill="currentColor" viewBox="0 0 24 24"><path d="M12 0c-6.626 0-12 5.373-12 12 0 5.302 3.438 9.8 8.207 11.387.599.111.793-.261.793-.577v-2.234c-3.338.726-4.033-1.416-4.033-1.416-.546-1.387-1.333-1.756-1.333-1.756-1.089-.745.083-.729.083-.729 1.205.084 1.839 1.237 1.839 1.237 1.07 1.834 2.807 1.304 3.492.997.107-.775.418-1.305.762-1.604-2.665-.305-5.467-1.334-5.467-5.931 0-1.311.469-2.381 1.236-3.221-.124-.303-.535-1.524.117-3.176 0 0 1.008-.322 3.301 1.23.957-.266 1.983-.399 3.003-.404 1.02.005 2.047.138 3.006.404 2.291-1.552 3.297-1.23 3.297-1.23.653 1.653.242 2.874.118 3.176.77.84 1.235 1.911 1.235 3.221 0 4.609-2.807 5.624-5.479 5.921.43.372.823 1.102.823 2.222v3.293c0 .319.192.694.801.576 4.765-1.589 8.199-6.086 8.199-11.386 0-6.627-5.373-12-12-12z"/></svg>
                    {data.github}
                </div>
            </div>

            <div>
                { rullst::html::RawHtml::new(data.skill_groups.iter().map(|g| format!(
                    "<div>\
                        <div class=\"skill-cat\">{}</div>\
                        <div class=\"tags\">{}</div>\
                    </div>",
                    g.category,
                    g.skills.iter().map(|s| format!("<span class=\"tag\">{}</span>", s)).collect::<Vec<_>>().join("")
                )).collect::<Vec<_>>().join("")) }
            </div>
        </aside>
    }
}

fn render_content(data: &CvData) -> String {
    html! {
        <main class="content">
            <section>
                <h2 class="section-title">"Experience"</h2>
                <div class="timeline">
                    { rullst::html::RawHtml::new(data.experiences.iter().map(|e| format!(
                        "<div class=\"timeline-item\">\
                            <div class=\"exp-period\">{}</div>\
                            <h3 class=\"exp-role\">{}</h3>\
                            <div class=\"exp-company\">{}</div>\
                            <p class=\"exp-desc\">{}</p>\
                        </div>", e.period, e.role, e.company, e.description
                    )).collect::<Vec<_>>().join("")) }
                </div>
            </section>

            <section>
                <h2 class="section-title">"Education"</h2>
                <div class="timeline">
                    { rullst::html::RawHtml::new(data.education.iter().map(|edu| format!(
                        "<div class=\"timeline-item\">\
                            <div class=\"exp-period\">{}</div>\
                            <h3 class=\"exp-role\">{}</h3>\
                            <div class=\"exp-company\">{}</div>\
                        </div>", edu.year, edu.degree, edu.institution
                    )).collect::<Vec<_>>().join("")) }
                </div>
            </section>

            <section>
                <h2 class="section-title">"Projects"</h2>
                <div class="projects-grid">
                    { rullst::html::RawHtml::new(data.projects.iter().map(|p| format!(
                        "<div class=\"project-card\">\
                            <h3 class=\"project-title\">{}</h3>\
                            <p class=\"project-desc\">{}</p>\
                            <div class=\"tags\">{}</div>\
                            <a href=\"{}\" class=\"project-link\">\
                                View Project\
                                <svg width=\"16\" height=\"16\" viewBox=\"0 0 24 24\" fill=\"none\" stroke=\"currentColor\" stroke-width=\"2\"><path d=\"M5 12h14M12 5l7 7-7 7\"/></svg>\
                            </a>\
                        </div>",
                        p.title, p.description,
                        p.tags.iter().map(|t| format!("<span class=\"tag\">{}</span>", t)).collect::<Vec<_>>().join(""),
                        p.link
                    )).collect::<Vec<_>>().join("")) }
                </div>
            </section>
        </main>
    }
}

pub fn render(data: CvData) -> String {
    html! {
        <html lang="en">
            <head>
                <meta charset="UTF-8" />
                <meta name="viewport" content="width=device-width, initial-scale=1.0" />
                <title>{data.name} " - CV"</title>
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;500;600;700;800&display=swap" rel="stylesheet" />
                <style>{ rullst::html::RawHtml(cv_styles()) }</style>
            </head>
            <body>
                <div class="bg-grid"></div>
                <div class="scanlines"></div>
                <div class="glow-blob glow-1"></div>
                <div class="glow-blob glow-2"></div>
                
                <div class="layout">
                    { rullst::html::RawHtml(render_sidebar(&data)) }
                    { rullst::html::RawHtml(render_content(&data)) }
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
