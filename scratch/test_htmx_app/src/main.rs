use rullst::{html, routes, Server, response::{Html, IntoResponse}};
use rullst::htmx::{HtmxRequest, render_page};
use rust_eloquent::sqlx::FromRow;

pub mod migrations;

// 1. Defina o seu modelo de banco de dados usando o ORM rust-eloquent embutido!
#[derive(Debug, Clone, FromRow, rust_eloquent::Eloquent)]
#[eloquent(table = "users")]
pub struct User {
    pub id: i32,
    pub name: String,
}

// Rota principal: usa o SSR híbrido com render_page
async fn home(htmx: HtmxRequest) -> impl IntoResponse {
    let name = "Rullst";
    
    // Exemplo de uso do ORM: Buscar usuários ativos do banco
    let db_status = match User::all().await {
        Ok(users) => format!("Banco conectado! Total de usuários cadastrados: {}", users.len()),
        Err(e) => format!("Banco offline ou não configurado: {}", e),
    };

    let content = html! {
        <div class="flex flex-col items-center justify-center min-h-screen bg-slate-950 text-slate-100 p-6 font-sans">
            <div class="max-w-xl text-center space-y-6">
                <h1 class="text-5xl font-extrabold tracking-tight bg-gradient-to-r from-sky-400 via-indigo-400 to-purple-500 bg-clip-text text-transparent">
                    "Bem-vindo ao " {name}
                </h1>
                
                <p class="text-slate-400 text-lg">
                    "O framework fullstack definitivo para Rust. Focado em Segurança, Manutenção e Velocidade."
                </p>

                <div class="inline-block px-4 py-2 bg-slate-900 border border-slate-800 rounded-lg text-sm text-sky-400 font-mono">
                    {db_status}
                </div>

                <div class="bg-slate-900/50 backdrop-blur-md p-6 rounded-xl border border-slate-800 space-y-4">
                    <h2 class="text-xl font-bold text-slate-200">"Interatividade HTMX sem JS personalizado!"</h2>
                    <div id="counter-box" class="flex flex-col items-center gap-3">
                        <button hx-post="/clicked" 
                                hx-target="#counter-box" 
                                hx-swap="outerHTML" 
                                class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                            "Clique aqui para incrementar"
                        </button>
                        <p class="text-sm text-slate-400">"Cliques recebidos no servidor: 0"</p>
                    </div>
                </div>
            </div>
        </div>
    };

    render_page(&htmx, "Bem-vindo ao Rullst", content)
}

// Estado para o contador
use std::sync::atomic::{AtomicUsize, Ordering};
static CLICK_COUNT: AtomicUsize = AtomicUsize::new(0);

// Endpoint HTMX reativo
async fn clicked() -> impl IntoResponse {
    let current_clicks = CLICK_COUNT.fetch_add(1, Ordering::SeqCst) + 1;
    
    // Retorna apenas a parcial / fragmento que substitui o elemento counter-box
    Html(html! {
        <div id="counter-box" class="flex flex-col items-center gap-3">
            <button hx-post="/clicked" 
                    hx-target="#counter-box" 
                    hx-swap="outerHTML" 
                    class="px-6 py-2.5 bg-gradient-to-r from-sky-500 to-indigo-600 hover:from-sky-400 hover:to-indigo-500 text-white font-medium rounded-lg shadow-lg hover:shadow-indigo-500/20 active:scale-95 transition duration-150 ease-in-out cursor-pointer">
                "Clique aqui para incrementar"
            </button>
            <p class="text-sm text-emerald-400 font-medium">"Cliques recebidos no servidor: " {current_clicks.to_string()}</p>
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
        post("/clicked" => clicked),
    ];

    Server::new(router)
        .run(3000)
        .await?;

    Ok(())
}
