// rullst-studio/src/ai_playground.rs — Interactive AI & RAG Test Bench for Rullst Studio

use axum::{Json, response::IntoResponse};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct PromptRequest {
    pub prompt: String,
    pub system_context: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PromptResponse {
    pub success: bool,
    pub response: String,
    pub provider: String,
}

/// Renders the AI Playground HTML tab for Rullst Studio
pub fn render_ai_playground_html() -> String {
    r#"
<div class="max-w-6xl mx-auto p-6 space-y-6">
  <div class="bg-slate-800/80 p-6 rounded-2xl border border-slate-700/60 shadow-xl backdrop-blur-md">
    <div class="flex justify-between items-center mb-4">
      <h2 class="text-2xl font-bold text-slate-100 flex items-center gap-2">
        <span>🤖 Rullst AI & RAG Playground</span>
      </h2>
      <span class="px-3 py-1 bg-indigo-500/10 text-indigo-400 border border-indigo-500/30 rounded-full text-xs font-mono">rullst-ai engine</span>
    </div>
    <p class="text-sm text-slate-400">Test AI prompts, embeddings, and context injections in real-time.</p>
  </div>

  <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
    <div class="bg-slate-900/90 border border-slate-700/80 p-5 rounded-2xl space-y-4">
      <h3 class="text-md font-semibold text-slate-200">Input Prompt</h3>
      <div>
        <label class="block text-xs font-medium text-slate-400 mb-1">System Context (Optional)</label>
        <textarea id="ai-system" rows="2" placeholder="You are an expert assistant for Rullst framework..." class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-slate-200 text-sm focus:outline-none focus:border-indigo-500"></textarea>
      </div>
      <div>
        <label class="block text-xs font-medium text-slate-400 mb-1">User Prompt</label>
        <textarea id="ai-prompt" rows="4" placeholder="How do I write a migration in Rullst?" class="w-full px-3 py-2 bg-slate-950 border border-slate-800 rounded-xl text-slate-200 text-sm focus:outline-none focus:border-indigo-500"></textarea>
      </div>
      <button onclick="sendAiPrompt()" class="w-full py-2.5 bg-indigo-600 hover:bg-indigo-500 text-white rounded-xl font-medium transition shadow-lg flex justify-center items-center gap-2">
        <span>✨ Send to AI Client</span>
      </button>
    </div>

    <div class="bg-slate-900/90 border border-slate-700/80 p-5 rounded-2xl flex flex-col justify-between">
      <div>
        <div class="flex justify-between items-center mb-3">
          <h3 class="text-md font-semibold text-slate-200">AI Response</h3>
          <span id="ai-provider-badge" class="text-xs font-mono text-slate-500">Provider: Idle</span>
        </div>
        <div id="ai-response-box" class="min-h-[160px] p-4 bg-slate-950 border border-slate-800 rounded-xl text-slate-300 text-sm whitespace-pre-wrap font-sans">
          Awaiting input prompt...
        </div>
      </div>
    </div>
  </div>
</div>

<script>
async function sendAiPrompt() {
  const prompt = document.getElementById('ai-prompt').value.trim();
  const system = document.getElementById('ai-system').value.trim();
  const box = document.getElementById('ai-response-box');
  const badge = document.getElementById('ai-provider-badge');

  if (!prompt) return;

  box.innerText = 'Thinking...';
  badge.innerText = 'Provider: Querying...';

  try {
    const res = await fetch('/_studio/api/ai/prompt', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ prompt: prompt, system_context: system || null })
    });
    const data = await res.json();
    box.innerText = data.response;
    badge.innerText = 'Provider: ' + data.provider;
  } catch (e) {
    box.innerText = '❌ Error querying AI: ' + e;
    badge.innerText = 'Provider: Error';
  }
}
</script>
"#.to_string()
}

pub async fn handle_ai_prompt(Json(payload): Json<PromptRequest>) -> impl IntoResponse {
    let response_text = format!(
        "Received Prompt: '{}'\nSystem Context: '{}'\n\n[rullst-ai]: Integration active and ready for AI provider responses.",
        payload.prompt,
        payload.system_context.as_deref().unwrap_or("None")
    );

    Json(PromptResponse {
        success: true,
        response: response_text,
        provider: "Rullst AI Engine (Local)".to_string(),
    })
}
