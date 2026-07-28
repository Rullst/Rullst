<script setup>
import { ref } from 'vue'

const code = ref(`// Rullst ORM Interactive Sandbox
// Try modifying the query below!

let user = User::query()
    .where_like("email", "%@gmail.com")
    .order_by_desc("created_at")
    .limit(5)
    .get()
    .await?;
`)

const output = ref('')
const isRunning = ref(false)

const runCode = () => {
  isRunning.value = true
  output.value = 'Compiling... 🔨'
  
  setTimeout(() => {
    // Simple simulation logic
    let sql = 'SELECT * FROM users'
    let binds = []
    
    if (code.value.includes('where_like("email"')) {
      const match = code.value.match(/where_like\("([^"]+)", "([^"]+)"\)/)
      if (match) {
        sql += ` WHERE ${match[1]} LIKE $1`
        binds.push(match[2])
      }
    }
    
    if (code.value.includes('order_by_desc')) {
      const match = code.value.match(/order_by_desc\("([^"]+)"\)/)
      if (match) {
        sql += ` ORDER BY ${match[1]} DESC`
      }
    }
    
    if (code.value.includes('limit')) {
      const match = code.value.match(/limit\((\d+)\)/)
      if (match) {
        sql += ` LIMIT ${match[1]}`
      }
    }
    
    output.value = `[Success] Query Executed Successfully! ✅\n\nGenerated SQL:\n> ${sql}\n\nBindings:\n> [${binds.join(', ')}]\n\nResults:\n[Row { id: 1, name: "Alice", email: "alice@gmail.com" }, Row { id: 2, name: "Bob", email: "bob@gmail.com" }]`
    isRunning.value = false
  }, 1000)
}
</script>

<template>
  <div class="sandbox-container glassmorphism">
    <div class="sandbox-header">
      <div class="terminal-dots">
        <span class="dot-red"></span>
        <span class="dot-yellow"></span>
        <span class="dot-green"></span>
      </div>
      <span class="sandbox-title">Rullst ORM Playground</span>
      <button class="run-btn neon-orange" @click="runCode" :disabled="isRunning">
        {{ isRunning ? 'Running...' : '▶ Run Code' }}
      </button>
    </div>
    
    <div class="sandbox-body">
      <div class="editor-pane">
        <textarea v-model="code" class="code-editor" spellcheck="false"></textarea>
      </div>
      <div class="output-pane">
        <div class="output-header">Terminal Output</div>
        <pre class="terminal-output" :class="{ 'text-green': output.includes('Success') }">{{ output || 'Waiting for input...' }}</pre>
      </div>
    </div>
  </div>
</template>

<style scoped>
.sandbox-container {
  margin: 2rem 0;
  border-radius: 12px;
  overflow: hidden;
  display: flex;
  flex-direction: column;
}
.sandbox-header {
  background: rgba(0, 0, 0, 0.6);
  padding: 0.8rem 1rem;
  display: flex;
  align-items: center;
  border-bottom: 1px solid var(--glass-border);
}
.terminal-dots span {
  width: 12px;
  height: 12px;
  border-radius: 50%;
  display: inline-block;
  margin-right: 6px;
}
.sandbox-title {
  margin-left: 1rem;
  font-family: var(--font-mono, monospace);
  font-size: 0.9rem;
  color: #8b949e;
  flex: 1;
}
.run-btn {
  background: var(--accent-orange, #ff6b00);
  color: white;
  border: none;
  padding: 0.4rem 1rem;
  border-radius: 6px;
  font-weight: bold;
  cursor: pointer;
  transition: all 0.2s;
}
.run-btn:hover:not(:disabled) {
  box-shadow: 0 0 10px var(--accent-orange, #ff6b00);
}
.run-btn:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}
.sandbox-body {
  display: flex;
  flex-direction: column;
  min-height: 400px;
}
@media (min-width: 768px) {
  .sandbox-body {
    flex-direction: row;
  }
}
.editor-pane, .output-pane {
  flex: 1;
  display: flex;
  flex-direction: column;
}
.editor-pane {
  border-right: 1px solid var(--glass-border);
}
.code-editor {
  flex: 1;
  background: rgba(13, 17, 23, 0.8);
  color: #c9d1d9;
  font-family: var(--font-mono, monospace);
  font-size: 0.95rem;
  padding: 1.5rem;
  border: none;
  resize: none;
  outline: none;
  line-height: 1.5;
}
.output-pane {
  background: rgba(0, 0, 0, 0.8);
}
.output-header {
  padding: 0.5rem 1rem;
  background: rgba(255, 255, 255, 0.05);
  font-size: 0.8rem;
  color: #8b949e;
  border-bottom: 1px solid var(--glass-border);
  text-transform: uppercase;
  letter-spacing: 1px;
}
.terminal-output {
  padding: 1.5rem;
  margin: 0;
  font-family: var(--font-mono, monospace);
  font-size: 0.9rem;
  color: #8b949e;
  white-space: pre-wrap;
  background: transparent;
  border: none;
}
.text-green {
  color: #27c93f;
}
</style>
