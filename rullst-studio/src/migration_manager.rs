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
pub fn render_migration_manager_html() -> String {
    r#"
<div class="max-w-6xl mx-auto p-6 space-y-6">
  <div class="flex justify-between items-center bg-slate-800/80 p-6 rounded-2xl border border-slate-700/60 shadow-xl backdrop-blur-md">
    <div>
      <h2 class="text-2xl font-bold text-slate-100 flex items-center gap-2">
        <span>🛠️ Database Tools & Migration Manager</span>
      </h2>
      <p class="text-sm text-slate-400 mt-1">Control migrations, rollbacks, and seeders directly from Rullst Studio.</p>
    </div>
    <div class="flex space-x-3">
      <button onclick="triggerMigration('run')" class="px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl font-medium transition shadow-lg flex items-center gap-2">
        <span>🚀 Run Migrations</span>
      </button>
      <button onclick="triggerMigration('rollback')" class="px-4 py-2 bg-rose-600/80 hover:bg-rose-500 text-white rounded-xl font-medium transition shadow-lg flex items-center gap-2">
        <span>↩️ Rollback Last Batch</span>
      </button>
      <button onclick="triggerSeeder()" class="px-4 py-2 bg-emerald-600 hover:bg-emerald-500 text-white rounded-xl font-medium transition shadow-lg flex items-center gap-2">
        <span>🌱 Run Seeders</span>
      </button>
    </div>
  </div>

  <div id="tool-output-card" class="hidden bg-slate-900/90 border border-slate-700/80 p-5 rounded-2xl text-mono text-sm">
    <div id="output-header" class="font-bold text-xs uppercase tracking-wider text-slate-400 mb-2">Operation Status</div>
    <div id="output-content" class="text-slate-200 whitespace-pre-wrap"></div>
  </div>
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
  content.innerText = 'Executing seeders...';

  try {
    const res = await fetch('/_studio/api/seeders/run', { method: 'POST' });
    const data = await res.json();
    content.innerText = (data.success ? '✅ ' : '❌ ') + data.message;
  } catch (e) {
    content.innerText = '❌ Error executing seeders: ' + e;
  }
}
</script>
"#.to_string()
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
