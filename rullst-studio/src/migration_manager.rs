// rullst-studio/src/migration_manager.rs — Visual Migration & Seeder Manager for Rullst Studio

use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct MigrationStatusItem {
    pub name: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    pub success: bool,
    pub message: String,
}

/// Renders the "Database Tools" HTML tab for Rullst Studio
pub fn render_migration_manager_html(schema_tables_html: &str) -> String {
    let base = r#"
<div class="max-w-6xl mx-auto p-6 space-y-6">
  <div class="bg-slate-800/80 p-6 rounded-2xl border border-slate-700/60 shadow-xl backdrop-blur-md space-y-4">
    <div>
      <h2 class="text-2xl font-bold text-slate-100 flex items-center gap-2">
        <span>🛠️ Database Tools & Migration Manager</span>
      </h2>
      <p class="text-sm text-slate-400 mt-1">Control schema migrations, rollbacks, and data seeders directly from Rullst Studio.</p>
    </div>

    <!-- Command Action Cards with Descriptions (English) -->
    <div class="grid grid-cols-1 md:grid-cols-3 gap-4 pt-2">
      <div class="p-4 bg-slate-900/90 border border-indigo-500/30 rounded-xl space-y-3 flex flex-col justify-between shadow-md">
        <div>
          <div class="flex items-center gap-2 text-indigo-400 font-bold text-sm">
            <span>🚀 Run Migrations</span>
          </div>
          <p class="text-xs text-slate-300 mt-2 leading-relaxed">
            Executes all pending database migrations (<code class="px-1 py-0.5 bg-slate-950 text-indigo-300 rounded font-mono">db:migrate</code>) to create or update your database schema tables safely.
          </p>
        </div>
        <button onclick="triggerMigration('run')" class="w-full py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-lg text-xs font-semibold transition shadow flex justify-center items-center gap-1.5">
          <span>Run Migrations</span>
        </button>
      </div>

      <div class="p-4 bg-slate-900/90 border border-rose-500/30 rounded-xl space-y-3 flex flex-col justify-between shadow-md">
        <div>
          <div class="flex items-center gap-2 text-rose-400 font-bold text-sm">
            <span>↩️ Rollback Last Batch</span>
          </div>
          <p class="text-xs text-slate-300 mt-2 leading-relaxed">
            Reverts the last batch of executed migrations (<code class="px-1 py-0.5 bg-slate-950 text-rose-300 rounded font-mono">db:rollback</code>), removing the latest schema changes.
          </p>
        </div>
        <button onclick="triggerMigration('rollback')" class="w-full py-2 bg-rose-600/80 hover:bg-rose-500 text-white rounded-lg text-xs font-semibold transition shadow flex justify-center items-center gap-1.5">
          <span>Rollback Batch</span>
        </button>
      </div>

      <div class="p-4 bg-slate-900/90 border border-emerald-500/30 rounded-xl space-y-3 flex flex-col justify-between shadow-md">
        <div>
          <div class="flex items-center gap-2 text-emerald-400 font-bold text-sm">
            <span>🌱 Run Seeders</span>
          </div>
          <p class="text-xs text-slate-300 mt-2 leading-relaxed">
            Populates your database tables with initial sample/mock data (<code class="px-1 py-0.5 bg-slate-950 text-emerald-300 rounded font-mono">db:seed</code>) for rapid testing and development.
          </p>
        </div>
        <button onclick="triggerSeeder()" class="w-full py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-lg text-xs font-semibold transition shadow flex justify-center items-center gap-1.5">
          <span>Run Seeders</span>
        </button>
      </div>
    </div>
  </div>

  <!-- Guidance Notice for Individual Record Management (Nexus CMS) -->
  <div class="bg-slate-900/90 border border-sky-500/30 p-5 rounded-2xl space-y-2 shadow-lg">
    <div class="flex items-center gap-2 text-sky-400 font-bold text-sm">
      <svg class="h-5 w-5 flex-shrink-0" fill="none" viewBox="0 0 24 24" stroke="currentColor" stroke-width="2">
        <path stroke-linecap="round" stroke-linejoin="round" d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
      </svg>
      <span>💡 Looking to Add, Edit, or Delete Individual Database Records?</span>
    </div>
    <p class="text-xs text-slate-300 leading-relaxed">
      <strong>Rullst Studio</strong> is designed for developer schema inspection, SQL queries, and migration management. If you want to manage individual database rows line-by-line (Create, Edit, or Delete records), use <strong>Rullst Nexus CMS</strong> — your auto-generated admin panel with full CRUD, search, and form validation:
    </p>
    <div class="pt-1 flex items-center gap-2">
      <a href="http://127.0.0.1:3000/nexus" target="_blank" class="px-4 py-2 bg-sky-600 hover:bg-sky-500 text-white text-xs font-semibold rounded-xl transition shadow inline-flex items-center gap-1.5">
        <span>⚙️ Open Rullst Nexus CMS (/nexus)</span>
      </a>
      <span class="text-xs text-slate-400 font-mono pl-2">(Default login: <code class="text-sky-300">admin</code> / <code class="text-sky-300">password</code>)</span>
    </div>
  </div>

  <div id="tool-output-card" class="hidden bg-slate-900/90 border border-slate-700/80 p-5 rounded-2xl text-mono text-sm shadow-xl">
    <div id="output-header" class="font-bold text-xs uppercase tracking-wider text-slate-400 mb-2">Operation Output</div>
    <div id="output-content" class="text-slate-200 whitespace-pre-wrap font-mono text-xs"></div>
  </div>

  SCHEMA_TABLES_PLACEHOLDER
</div>

<script>
async function triggerMigration(action) {
  const card = document.getElementById('tool-output-card');
  const content = document.getElementById('output-content');
  card.classList.remove('hidden');
  content.innerText = 'Executing ' + action + '...';

  try {
    const res = await fetch('/_studio/api/migrations/' + action, { method: 'POST' });
    const data = await res.json();
    content.innerText = (data.success ? '✅ ' : '❌ ') + data.message;
  } catch (e) {
    content.innerText = '❌ Error executing operation: ' + e;
  }
}

async function triggerSeeder() {
  const card = document.getElementById('tool-output-card');
  const content = document.getElementById('output-content');
  card.classList.remove('hidden');
  content.innerText = 'Running seeders...';

  try {
    const res = await fetch('/_studio/api/seeders/run', { method: 'POST' });
    const data = await res.json();
    content.innerText = (data.success ? '✅ ' : '❌ ') + data.message;
  } catch (e) {
    content.innerText = '❌ Error executing seeders: ' + e;
  }
}
</script>
"#;

    base.replace("SCHEMA_TABLES_PLACEHOLDER", schema_tables_html)
}

pub async fn handle_run_migrations() -> impl IntoResponse {
    let args = vec!["artisan".to_string(), "migrate".to_string()];
    match rullst_orm::schema::run_artisan_with_args(&args, vec![], vec![]).await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: "Migrations executed successfully!".to_string(),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            message: format!("Migration error: {}", e),
        }),
    }
}

pub async fn handle_rollback_migrations() -> impl IntoResponse {
    let args = vec!["artisan".to_string(), "migrate:rollback".to_string()];
    match rullst_orm::schema::run_artisan_with_args(&args, vec![], vec![]).await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: "Rollback executed successfully!".to_string(),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            message: format!("Rollback error: {}", e),
        }),
    }
}

pub async fn handle_run_seeders() -> impl IntoResponse {
    let args = vec!["artisan".to_string(), "db:seed".to_string()];
    match rullst_orm::schema::run_artisan_with_args(&args, vec![], vec![]).await {
        Ok(_) => Json(ApiResponse {
            success: true,
            message: "Seeders executed successfully!".to_string(),
        }),
        Err(e) => Json(ApiResponse {
            success: false,
            message: format!("Seeder error: {}", e),
        }),
    }
}
