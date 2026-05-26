use crate::ws::WebSocket;
use async_trait::async_trait;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;
use serde_json::Value;

/// Rullst Live Component (Server-Driven UI)
/// Inspirado no Phoenix LiveView e Laravel Livewire, permitindo que você escreva
/// componentes interativos totalmente em Rust, atualizados em tempo real via WebSockets.
#[async_trait]
pub trait LiveComponent: Send + Sync + Default + 'static {
    /// Chamado na primeira renderização (tanto no carregamento HTTP inicial quanto ao abrir a conexão WebSocket).
    async fn mount(&mut self) {}

    /// Processa eventos JSON originados do frontend via WebSocket.
    /// O HTMX enviará por padrão um payload JSON contendo cabeçalhos e valores submetidos (hx-vals, formulários).
    async fn handle_event(&mut self, _payload: Value) {}

    /// Renderiza o estado atual do componente em uma String HTML.
    /// OBRIGATÓRIO: O root da string renderizada DEVE possuir um atributo `id` exclusivo
    /// para que o HTMX saiba exatamente qual nó do DOM deve ser atualizado.
    fn render(&self) -> String;
}

/// Handler genérico do Axum para lidar com a rota WebSocket de um componente Rullst Live.
/// Ele instanciará o componente, chamará o `mount` e entrará no loop de escuta de eventos.
pub async fn live_ws_handler<C: LiveComponent>(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|socket| async move {
        let mut rullst_ws = WebSocket::new(socket);
        let mut component = C::default();

        // Monta o estado inicial na sessão do WebSocket
        component.mount().await;

        // Loop contínuo recebendo eventos do frontend (HTMX ws-ext)
        while let Some(Ok(msg)) = rullst_ws.recv().await {
            // O HTMX envia mensagens em formato JSON com headers e valores de input
            if let Ok(payload) = serde_json::from_str::<Value>(&msg) {
                // Repassa o evento para o ciclo de vida do componente
                component.handle_event(payload).await;

                // Re-renderiza o HTML após a possível mutação de estado
                let html = component.render();

                // Dispara o novo HTML via WebSocket. O HTMX fará o hot-swap automaticamente usando o ID do root.
                if let Err(e) = rullst_ws.send_html(html).await {
                    eprintln!("Rullst Live WS Error: {}", e);
                    break; // Cliente desconectado ou falha na rede
                }
            }
        }
    })
}

/// Utilitário para facilitar a montagem de um componente Live em uma página HTTP normal.
pub struct Live;

impl Live {
    /// Gera a tag wrapper `<div>` que ativa a extensão `hx-ext="ws"` do HTMX.
    /// Ele faz o pré-render (`mount` + `render`) para garantir SSR otimizado para SEO na primeira carga.
    pub async fn mount<C: LiveComponent>(ws_path: &str) -> String {
        let mut comp = C::default();
        comp.mount().await;
        let html = comp.render();

        // HTML escape ws_path to prevent path/attribute injection
        let safe_path = ws_path
            .replace('&', "&amp;")
            .replace('"', "&quot;")
            .replace('\'', "&#x27;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");

        // Encapsula o componente em uma div invisível que instrui o HTMX a abrir o WebSocket
        format!(
            "<div hx-ext=\"ws\" ws-connect=\"{}\">\n{}\n</div>",
            safe_path, html
        )
    }
}
