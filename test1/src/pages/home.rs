use rullst::html;
use crate::controllers::portfolio_controller::Project;

fn home_styles() -> String {
    r#"
    * { box-sizing: border-box; margin: 0; padding: 0; font-family: 'Outfit', sans-serif; }
    body { background: #050505; color: #f3f4f6; min-height: 100vh; overflow-x: hidden; position: relative; }
    
    /* Dynamic glowing background */
    .glow-blob { position: absolute; border-radius: 50%; filter: blur(100px); z-index: -1; animation: float 10s infinite ease-in-out alternate; }
    .glow-1 { top: 10%; left: 10%; width: 500px; height: 500px; background: rgba(249, 115, 22, 0.15); }
    .glow-2 { bottom: 10%; right: 10%; width: 600px; height: 600px; background: rgba(5, 150, 105, 0.15); animation-delay: -5s; }
    .glow-3 { top: 50%; left: 40%; width: 400px; height: 400px; background: rgba(249, 115, 22, 0.1); animation-delay: -2s; }
    
    @keyframes float {
        0% { transform: translate(0, 0) scale(1); }
        100% { transform: translate(30px, 50px) scale(1.1); }
    }

    .container { max-width: 1200px; margin: 0 auto; padding: 4rem 2rem; z-index: 1; }
    
    header { text-align: center; margin-bottom: 4rem; margin-top: 4rem; }
    .badge { background: rgba(255, 255, 255, 0.05); border: 1px solid rgba(255, 255, 255, 0.1); padding: 0.5rem 1.5rem; border-radius: 9999px; font-size: 0.85rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.1em; display: inline-block; margin-bottom: 1.5rem; backdrop-filter: blur(10px); }
    h1 { font-size: 4.5rem; font-weight: 800; line-height: 1.1; margin-bottom: 1.5rem; background: linear-gradient(135deg, #34d399 0%, #f97316 100%); -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
    p.sub { color: #9ca3af; font-size: 1.25rem; max-width: 600px; margin: 0 auto; line-height: 1.6; }

    .section-title { font-size: 2.5rem; font-weight: 800; color: #fff; margin-bottom: 2rem; border-bottom: 2px solid rgba(255,255,255,0.1); padding-bottom: 0.5rem; display: inline-block; }
    
    /* Skills Section */
    .skills-container { display: flex; flex-wrap: wrap; gap: 1rem; margin-bottom: 6rem; justify-content: center; }
    .skill-pill { background: rgba(52, 211, 153, 0.1); border: 1px solid rgba(52, 211, 153, 0.2); color: #34d399; padding: 0.75rem 1.5rem; border-radius: 999px; font-weight: 600; transition: all 0.3s ease; cursor: default; }
    .skill-pill:hover { background: rgba(52, 211, 153, 0.2); transform: translateY(-3px); box-shadow: 0 10px 20px rgba(52, 211, 153, 0.15); }

    .projects-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(350px, 1fr)); gap: 2.5rem; margin-bottom: 6rem; }
    
    .project-card { background: rgba(17, 24, 39, 0.4); backdrop-filter: blur(16px); -webkit-backdrop-filter: blur(16px); border: 1px solid rgba(255, 255, 255, 0.08); border-radius: 24px; overflow: hidden; transition: all 0.4s cubic-bezier(0.4, 0, 0.2, 1); cursor: pointer; }
    .project-card:hover { transform: translateY(-10px); border-color: rgba(52, 211, 153, 0.4); box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4); }
    .project-img-wrapper { width: 100%; height: 220px; overflow: hidden; }
    .project-img { width: 100%; height: 100%; object-fit: cover; transition: transform 0.5s ease; }
    .project-card:hover .project-img { transform: scale(1.05); }
    
    .project-content { padding: 2rem; }
    .project-title { font-size: 1.5rem; font-weight: 600; color: #ffffff; margin-bottom: 0.75rem; }
    .project-desc { color: #9ca3af; font-size: 1rem; line-height: 1.6; margin-bottom: 1.5rem; }
    
    .tags { display: flex; flex-wrap: wrap; gap: 0.5rem; }
    .tag { background: rgba(249, 115, 22, 0.15); color: #fed7aa; padding: 0.25rem 0.75rem; border-radius: 6px; font-size: 0.8rem; font-weight: 500; border: 1px solid rgba(249, 115, 22, 0.2); }
    
    footer { text-align: center; padding: 4rem 2rem; border-top: 1px solid rgba(255,255,255,0.05); color: #6b7280; font-size: 0.9rem; margin-top: 2rem; }
    .contact-btn { display: inline-block; margin-top: 2rem; padding: 1rem 2.5rem; background: #f97316; color: #050505; font-weight: 800; border-radius: 999px; text-decoration: none; transition: all 0.3s ease; font-size: 1.1rem; }
    .contact-btn:hover { background: #ea580c; transform: scale(1.05); box-shadow: 0 10px 25px rgba(249, 115, 22, 0.3); }
    "#.to_string()
}

fn home_header() -> String {
    html! {
        <header>
            <div class="badge">"Available for Hire"</div>
            <h1>"Building the Future with AI & Rust"</h1>
            <p class="sub">"I specialize in high-performance fullstack systems, agentic AI frameworks, and immersive web experiences powered by Rullst."</p>
            <a href="mailto:hello@example.com" class="contact-btn">"Let's Build Something"</a>
        </header>
    }
}

fn home_skills() -> String {
    html! {
        <div>
            <div style="text-align: center;">
                <h2 class="section-title">"Core Technologies"</h2>
            </div>
            <div class="skills-container">
                <div class="skill-pill">"Rust 🦀"</div>
                <div class="skill-pill">"Rullst Framework"</div>
                <div class="skill-pill">"Axum / Tokio"</div>
                <div class="skill-pill">"LLM Prompt Engineering"</div>
                <div class="skill-pill">"WebAssembly (Wasm)"</div>
                <div class="skill-pill">"HTMX & TailwindCSS"</div>
                <div class="skill-pill">"PostgreSQL & Redis"</div>
            </div>
        </div>
    }
}

fn home_projects(projects: Vec<Project>) -> String {
    html! {
        <div>
            <div style="text-align: center;">
                <h2 class="section-title">"Featured Projects"</h2>
            </div>
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
    }
}

fn home_footer() -> String {
    html! {
        <footer>
            <p>"© 2026 AI Developer. Built with the speed of Rust and Rullst Framework."</p>
        </footer>
    }
}

pub fn render(projects: Vec<Project>) -> String {
    html! {
        <html lang="en" class="dark">
            <head>
                <meta charset="UTF-8" />
                <title>"AI Developer Portfolio"</title>
                <link rel="icon" type="image/png" href="/static/favicon.png" />
                <link href="https://fonts.googleapis.com/css2?family=Outfit:wght@300;400;600;800&display=swap" rel="stylesheet" />
                <style>
                    { rullst::html::RawHtml(home_styles()) }
                </style>
            </head>
            <body>
                <div class="glow-blob glow-1"></div>
                <div class="glow-blob glow-2"></div>
                <div class="glow-blob glow-3"></div>

                <div class="container">
                    { rullst::html::RawHtml(home_header()) }
                    { rullst::html::RawHtml(home_skills()) }
                    { rullst::html::RawHtml(home_projects(projects)) }
                </div>

                { rullst::html::RawHtml(home_footer()) }
            </body>
        </html>
    }
}
