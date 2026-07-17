const fs = require('fs');
let content = fs.readFileSync('data.js', 'utf16le');
if (content.charCodeAt(0) === 0xFEFF) {
  content = content.slice(1);
}
// evaluate it
const window = {};
eval(content);

const data = window.BENCHMARK_DATA;
const entries = data.entries || data;
const suiteName = Object.keys(entries)[0];
const commits = entries[suiteName];

const datasets = {};
const labels = [];
const recentCommits = commits.slice(-30);
console.log('recentCommits length: ', recentCommits.length);

recentCommits.forEach(entry => {
  labels.push(new Date(entry.commit.timestamp).toLocaleDateString() + ' ' + entry.commit.id.substring(0, 7));
  entry.benches.forEach(bench => {
    if (!datasets[bench.name]) datasets[bench.name] = [];
  });
});

console.log('labels:', labels.length);
console.log('datasets:', Object.keys(datasets));

let prevEntry = commits[commits.length - 2] || null;
console.log('Test successful');
