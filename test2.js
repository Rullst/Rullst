const fs = require('fs');
let content = fs.readFileSync('data.js', 'utf16le');
if (content.charCodeAt(0) === 0xFEFF) {
  content = content.slice(1);
}
const window = {};
eval(content);

const data = window.BENCHMARK_DATA;
const entries = data.entries || data;
const suiteName = Object.keys(entries)[0];
const commits = entries[suiteName];

const reversedCommits = [...commits].reverse().slice(0, 10);
let prevEntry = commits[commits.length - 2] || null;

try {
  reversedCommits.forEach((entry, idx) => {
    const date = new Date(entry.commit.timestamp).toLocaleString();
    const msg = entry.commit.message.split('\\n')[0].substring(0, 50);
    const link = 'https://github.com/Rullst/Rullst/commit/' + entry.commit.id;
    
    const mainBench = entry.benches[0];
    let valStr = mainBench ? (mainBench.value + ' ' + mainBench.unit) : '-';
    let changeStr = '<span class=\"badge\">Baseline</span>';
    
    if (mainBench && prevEntry) {
      const prevBench = prevEntry.benches.find(b => b.name === mainBench.name);
      if (prevBench) {
        const diff = mainBench.value - prevBench.value;
        const pct = (diff / prevBench.value) * 100;
        if (pct > 2) {
          changeStr = '<span class=\"badge regression\">?? +' + pct.toFixed(2) + '% (Slower)</span>';
        } else if (pct < -2) {
          changeStr = '<span class=\"badge\">?? ' + pct.toFixed(2) + '% (Faster)</span>';
        } else {
          changeStr = '<span style=\"color: #64748b;\">~ 0.00%</span>';
        }
      }
    }
    prevEntry = [...commits].reverse()[idx + 1] || null;
  });
  console.log('No error in table loop!');
} catch (e) {
  console.error(e);
}
