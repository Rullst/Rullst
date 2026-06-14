use rullst::server::IntoResponse;
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
