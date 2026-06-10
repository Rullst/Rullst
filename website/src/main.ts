import './style.css'

const sleep = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

async function runTerminalAnimation() {
  const terminal = document.getElementById('terminal-body');
  if (!terminal) return;

  const steps = [
    { type: 'input', text: 'cargo rullst' },
    { type: 'empty', delay: 200 },
    { type: 'output', text: '📜🦀🌐 Rullst Framework v2.0.5', color: '#10b981', delay: 200 },
    { type: 'output', text: 'The emotional productivity framework for Rust.', delay: 200 },
    { type: 'empty', delay: 200 },
    { type: 'menu', text: '? Select Database:', delay: 400 },
    { type: 'option', text: '❯ SQLite (Fast, single-file)', active: true, delay: 500 },
    { type: 'option', text: '  PostgreSQL (Production)', active: false, delay: 100 },
    { type: 'option', text: '  MySQL (Production)', active: false, delay: 100 },
    { type: 'empty', delay: 500 },
    { type: 'output', text: '✔ Database selected: SQLite', color: '#3b82f6', delay: 400 },
    { type: 'empty', delay: 200 },
    { type: 'menu', text: '? Include Authentication Scaffold?', delay: 400 },
    { type: 'option', text: '❯ Yes (JWT + Session)', active: true, delay: 300 },
    { type: 'option', text: '  No', active: false, delay: 100 },
    { type: 'empty', delay: 500 },
    { type: 'output', text: '✔ Auth selected: Yes', color: '#3b82f6', delay: 400 },
    { type: 'empty', delay: 200 },
    { type: 'output', text: '🚀 Scaffolding complete in 0.08s!', color: '#10b981', delay: 400 },
    { type: 'output', text: '  cd my_app', delay: 100 },
    { type: 'output', text: '  cargo rullst dev', delay: 100 }
  ];

  while (true) {
    terminal.innerHTML = '';
    
    // Add prompt line with cursor
    const line = document.createElement('div');
    line.className = 'term-line';
    line.innerHTML = `<span class="term-prompt">~</span> <span class="term-cmd"></span><span class="term-cursor"></span>`;
    terminal.appendChild(line);

    const cmdSpan = line.querySelector('.term-cmd') as HTMLSpanElement;
    const cursorSpan = line.querySelector('.term-cursor') as HTMLSpanElement;

    for (const step of steps) {
      if (step.type === 'input') {
        await sleep(500);
        for (let i = 0; i < step.text!.length; i++) {
          cmdSpan.textContent += step.text![i];
          await sleep(Math.random() * 50 + 50); // random typing speed
        }
        await sleep(400); // Wait before "pressing enter"
        cursorSpan.style.display = 'none'; // Hide cursor on this line
      } 
      else if (step.type === 'output' || step.type === 'menu' || step.type === 'option') {
        const outLine = document.createElement('div');
        outLine.className = 'term-line term-output';
        
        if (step.type === 'menu') {
          outLine.style.color = '#3b82f6';
          outLine.style.fontWeight = 'bold';
        } else if (step.type === 'option') {
          outLine.className = 'term-line term-menu';
          if (step.active) {
            outLine.style.color = '#10b981';
          }
        }
        
        if (step.color) {
          outLine.style.color = step.color;
        }

        outLine.textContent = step.text!;
        terminal.appendChild(outLine);
        await sleep(step.delay || 0);
      }
      else if (step.type === 'empty') {
        const emptyLine = document.createElement('div');
        emptyLine.className = 'term-line';
        emptyLine.innerHTML = '&nbsp;';
        terminal.appendChild(emptyLine);
        await sleep(step.delay || 0);
      }
    }

    // Add final prompt
    const finalLine = document.createElement('div');
    finalLine.className = 'term-line';
    finalLine.innerHTML = `<span class="term-prompt">~/my_startup</span> <span class="term-cursor"></span>`;
    terminal.appendChild(finalLine);

    // Wait 5 seconds before restarting animation
    await sleep(5000);
  }
}

// Copy to clipboard functionality
document.getElementById('copy-install')?.addEventListener('click', (e) => {
  const btn = e.target as HTMLButtonElement;
  navigator.clipboard.writeText('cargo install cargo-rullst');
  const originalText = btn.textContent;
  btn.textContent = 'Copied! 🎉';
  btn.style.background = '#059669';
  setTimeout(() => {
    btn.textContent = originalText;
    btn.style.background = '';
  }, 2000);
});

// Start animation when elements are visible
document.addEventListener('DOMContentLoaded', () => {
  const observer = new IntersectionObserver((entries) => {
    entries.forEach(entry => {
      if (entry.isIntersecting) {
        runTerminalAnimation();
        observer.disconnect();
      }
    });
  });
  
  const terminal = document.getElementById('terminal-body');
  if (terminal) {
    observer.observe(terminal);
  }
});
