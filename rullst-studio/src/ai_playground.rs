// rullst-studio/src/ai_playground.rs — explicit AI integration boundary.

/// Renders the current Studio AI integration boundary without fabricating a
/// provider response or inferring that an application mounted guardrails.
pub fn render_ai_playground_html() -> String {
    r#"
<div class="max-w-6xl mx-auto p-6 space-y-6">
  <div class="bg-slate-800/80 p-6 rounded-2xl border border-slate-700/60 shadow-xl backdrop-blur-md">
    <h2 class="text-2xl font-bold text-slate-100">AI integration</h2>
    <p class="text-sm text-slate-400 mt-3">
      No AI client is connected to this Studio instance. Studio does not infer
      provider reachability or guardrail enforcement from environment variables,
      and it never returns a fabricated model response.
    </p>
  </div>

  <div class="bg-slate-900/90 border border-amber-500/30 p-5 rounded-2xl space-y-3 shadow-lg">
    <h3 class="text-amber-400 font-bold text-sm">Supported application path</h3>
    <p class="text-xs text-slate-300 leading-relaxed">
      Construct <code class="text-amber-300">rullst_ai::AiClient</code> in the
      application and expose only an authenticated, authorized and rate-limited
      application endpoint. The high-level client supports OpenAI, Anthropic,
      Gemini, DeepSeek and Ollama plus deterministic mock credentials for tests.
      An environment variable alone does not prove that a provider is reachable.
    </p>
    <p class="text-xs text-slate-400">
      A future Studio adapter may accept an explicitly supplied client and audit
      sink. Until that contract exists, prompt execution is deliberately unavailable.
    </p>
  </div>
</div>
"#
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ai_surface_is_explicitly_unconnected_and_has_no_fake_prompt_route() {
        let html = render_ai_playground_html();
        assert!(html.contains("No AI client is connected"));
        assert!(html.contains("deliberately unavailable"));
        assert!(!html.contains("Connection successful"));
        assert!(!html.contains("/_studio/api/ai/prompt"));
    }
}
