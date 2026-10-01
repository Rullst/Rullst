//! Showcase posts seeded into the `community` tenant.

use super::Post;
use rullst_orm::with_tenant;

/// Tenant that receives the seed posts.
pub const SEED_TENANT: &str = "community";

/// Title of the welcome post; seed posts are identified by exact title.
pub const WELCOME_TITLE: &str = "Welcome to The Sovereign SaaS Blog & Publisher";

/// Title of the UI integration overview post.
pub const PATTERNS_TITLE: &str = "Architecture Deep Dive: Four UI Integration Patterns";

const WELCOME_BODY: &str = "Welcome to The Sovereign SaaS Blog & Publisher. Explore the top navigation bar to test the available front-end foundations, the Hybrid ORM, local Rullst Studio (http://127.0.0.1:5555), Nexus CMS (/nexus), Capital Billing, and Security RASP demonstrations.\n\nThe example uses task-local tenant scoping as a development fixture. Production applications must derive membership from authenticated identity and keep cross-tenant negative tests in their own authorization model.";

const PATTERNS_BODY: &str = "This bounded showcase places four UI integration patterns beside the same Rust backend. They are options, not a claim that one abstraction removes every kind of lock-in.\n\n1. ⚡ Server-rendered HTML with optional HTMX:\n- `html!` produces typed server-side markup. This demo loads HTMX from a CDN where partial navigation needs a small browser runtime.\n\n2. 🔴 Server-driven WebSocket UI (`rullst::live`):\n- State mutations execute on the Tokio server while HTMX and its WebSocket extension carry events and apply returned markup in the browser.\n\n3. 🎨 Semantic CSS (`/pico-demo`):\n- Pico.css styles ordinary HTML without a Node.js build pipeline. This showcase still uses small inline browser handlers for its interactive controls.\n\n4. 📄 Embedded file templates (`/templates-demo`):\n- An external HTML file is embedded and populated by a deliberately small fixture. It demonstrates file separation, not the full Tera language or runtime.";

/// Inserts each showcase post into the `community` tenant unless a post with
/// the same title already exists there.
///
/// Seeding never deletes or rewrites rows, so posts written by visitors (in
/// any tenant) survive restarts, and a restart never duplicates a seed post.
pub async fn seed_showcase_posts() -> Result<(), rullst_orm::Error> {
    with_tenant(SEED_TENANT, async {
        for (title, body) in [
            (WELCOME_TITLE, WELCOME_BODY),
            (PATTERNS_TITLE, PATTERNS_BODY),
        ] {
            let exists = Post::query()
                .where_eq("title", title)
                .first()
                .await?
                .is_some();
            if !exists {
                let mut post = Post {
                    id: 0,
                    tenant_id: SEED_TENANT.to_string(),
                    title: title.to_string(),
                    body: body.to_string(),
                };
                post.save().await?;
            }
        }
        Ok(())
    })
    .await
}
