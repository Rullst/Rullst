<template>
  <div class="sandbox-container">
    <div class="sandbox-header">
      <div class="window-controls">
        <span class="dot red"></span>
        <span class="dot yellow"></span>
        <span class="dot green"></span>
      </div>
      <div class="window-title">Rullst Interactive Sandbox</div>
    </div>
    
    <div class="sandbox-body">
      <div class="sandbox-editor">
        <div class="file-tab">src/islands/counter.rs</div>
        <pre v-pre><code class="language-rust">use rullst::{island, view};
use wasm_bindgen::prelude::*;
use web_sys::HtmlElement;

#[island]
pub fn counter(container: HtmlElement) {
    let doc = web_sys::window().unwrap().document().unwrap();
    let btn = doc.create_element("button").unwrap();
    btn.set_inner_html("Click me! Count: 0");
    
    let mut count = 0;
    let closure = Closure::wrap(Box::new(move || {
        count += 1;
        btn.set_inner_html(&format!("Click me! Count: {}", count));
    }) as Box&lt;dyn FnMut()&gt;);
    
    btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();
    
    container.append_child(&btn).unwrap();
}</code></pre>
      </div>
      
      <div class="sandbox-preview">
        <div class="preview-header">Live Preview (Wasm Island)</div>
        <div class="preview-content">
          <div class="island-container">
            <h3>My Reactive Rust Island 🏝️</h3>
            <button @click="incrementCount" class="interactive-btn">
              Click me! Count: {{ count }}
            </button>
          </div>
          
          <div class="terminal-mock">
            <div class="term-line" v-if="compiled">> cargo rullst build --island</div>
            <div class="term-line success" v-if="compiled">✔ Compiled Wasm Island successfully in 42ms</div>
            <div class="term-line" v-for="log in logs" :key="log.id">> {{ log.text }}</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup>
import { ref } from 'vue'

const count = ref(0)
const compiled = ref(true)
const logs = ref([])
let logId = 0

const incrementCount = () => {
  count.value++
  logs.value.push({
    id: logId++,
    text: `Wasm event triggered: state updated to ${count.value}`
  })
  
  if (logs.value.length > 3) {
    logs.value.shift()
  }
}
</script>

<style scoped>
.sandbox-container {
  border-radius: 12px;
  overflow: hidden;
  background-color: #1e1e20;
  border: 1px solid #333;
  margin-top: 2rem;
  box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
}

.sandbox-header {
  background-color: #252529;
  padding: 12px 16px;
  display: flex;
  align-items: center;
  border-bottom: 1px solid #333;
}

.window-controls {
  display: flex;
  gap: 8px;
}

.dot {
  width: 12px;
  height: 12px;
  border-radius: 50%;
}

.red { background-color: #ff5f56; }
.yellow { background-color: #ffbd2e; }
.green { background-color: #27c93f; }

.window-title {
  margin: 0 auto;
  color: #888;
  font-size: 14px;
  font-weight: 500;
}

.sandbox-body {
  display: flex;
  flex-direction: column;
}

@media (min-width: 768px) {
  .sandbox-body {
    flex-direction: row;
  }
}

.sandbox-editor {
  flex: 1.2;
  border-right: 1px solid #333;
  background-color: #1e1e20;
  overflow-x: auto;
}

.file-tab {
  background-color: #252529;
  color: #ccc;
  font-size: 12px;
  padding: 8px 16px;
  display: inline-block;
  border-top-right-radius: 8px;
}

.sandbox-editor pre {
  margin: 0;
  padding: 16px;
  font-family: "Fira Code", monospace;
  font-size: 13px;
  color: #e2e8f0;
  background: transparent;
}

.sandbox-preview {
  flex: 1;
  display: flex;
  flex-direction: column;
  background-color: #0f172a;
}

.preview-header {
  padding: 12px 16px;
  color: #38bdf8;
  font-size: 13px;
  font-weight: bold;
  border-bottom: 1px solid #1e293b;
}

.preview-content {
  padding: 24px;
  display: flex;
  flex-direction: column;
  gap: 24px;
  height: 100%;
}

.island-container {
  background: rgba(30, 41, 59, 0.7);
  border: 1px solid #334155;
  border-radius: 8px;
  padding: 24px;
  text-align: center;
}

.island-container h3 {
  margin-top: 0;
  color: #f8fafc;
  font-size: 18px;
  margin-bottom: 16px;
}

.interactive-btn {
  background: linear-gradient(135deg, #38bdf8, #818cf8);
  border: none;
  border-radius: 6px;
  padding: 12px 24px;
  color: white;
  font-weight: bold;
  font-size: 15px;
  cursor: pointer;
  transition: transform 0.1s, box-shadow 0.2s;
  box-shadow: 0 4px 14px rgba(56, 189, 248, 0.4);
}

.interactive-btn:active {
  transform: scale(0.96);
}

.terminal-mock {
  margin-top: auto;
  background-color: #000;
  border-radius: 6px;
  padding: 12px;
  font-family: monospace;
  font-size: 12px;
  color: #a1a1aa;
}

.term-line {
  margin-bottom: 4px;
}

.term-line.success {
  color: #4ade80;
}
</style>
