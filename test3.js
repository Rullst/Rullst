const fs = require('fs');
let content = fs.readFileSync('data.js', 'utf16le');
if (content.charCodeAt(0) === 0xFEFF) content = content.slice(1);
const window = {};
eval(content);

const data = window.BENCHMARK_DATA;
const entries = data.entries || data;
const suiteName = Object.keys(entries)[0];
const commits = entries[suiteName];

for (let i = 0; i < commits.length; i++) {
  const c = commits[i];
  if (!c.commit) console.log('Missing commit at ' + i);
  else if (!c.commit.message) console.log('Missing message at ' + i);
  else if (!c.commit.timestamp) console.log('Missing timestamp at ' + i);
}
console.log('Checked all commits!');
