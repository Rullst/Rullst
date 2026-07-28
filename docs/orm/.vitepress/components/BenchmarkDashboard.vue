<script setup>
import { onMounted, ref } from 'vue'
import Chart from 'chart.js/auto'

const containerRef = ref(null)
const errorMsg = ref('')
const isLoading = ref(true)

onMounted(() => {
  const script = document.createElement('script')
  script.src = 'https://rullst.github.io/rullst-orm/bench/data.js'
  script.onload = () => {
    if (window.BENCHMARK_DATA) {
      renderCharts(window.BENCHMARK_DATA)
    } else {
      errorMsg.value = 'No benchmark data found.'
      isLoading.value = false
    }
  }
  script.onerror = () => {
    errorMsg.value = 'Failed to load benchmark data. Make sure CI has run successfully.'
    isLoading.value = false
  }
  document.head.appendChild(script)

  function renderCharts(data) {
    isLoading.value = false
    const container = containerRef.value
    if (!container) return

    const entries = data.entries || data
    
    for (const [suiteName, history] of Object.entries(entries)) {
      if (!history || history.length === 0) continue
      
      const latestRun = history[history.length - 1]
      const testNames = latestRun.benches.map(b => b.name)
      
      testNames.forEach(testName => {
        const card = document.createElement('div')
        card.className = 'glassmorphism'
        card.style.padding = '2rem'
        card.style.textAlign = 'center'
        card.style.marginBottom = '2rem'
        
        const title = document.createElement('h3')
        title.innerText = testName
        title.style.color = 'var(--accent-blue)'
        title.style.marginBottom = '1.5rem'
        card.appendChild(title)
        
        const canvas = document.createElement('canvas')
        canvas.style.width = '100%'
        canvas.style.maxHeight = '300px'
        card.appendChild(canvas)
        container.appendChild(card)
        
        const labels = []
        const dataPoints = []
        
        history.forEach(run => {
          const bench = run.benches.find(b => b.name === testName)
          if (bench) {
            const date = new Date(run.date)
            labels.push(`${date.getMonth()+1}/${date.getDate()} - ${run.commit.id.substring(0,6)}`)
            dataPoints.push(bench.value)
          }
        })
        
        const unit = latestRun.benches.find(b => b.name === testName)?.unit || 'ns/iter'

        new Chart(canvas, {
          type: 'line',
          data: {
            labels: labels,
            datasets: [{
              label: `${testName} (${unit})`,
              data: dataPoints,
              borderColor: '#ff6b00',
              backgroundColor: 'rgba(255, 107, 0, 0.2)',
              borderWidth: 2,
              pointBackgroundColor: '#007acc',
              pointBorderColor: '#007acc',
              pointRadius: 4,
              fill: true,
              tension: 0.3
            }]
          },
          options: {
            responsive: true,
            plugins: {
              legend: { display: false },
              tooltip: {
                backgroundColor: 'rgba(0, 0, 0, 0.8)',
                titleColor: '#007acc',
                bodyColor: '#fff',
                borderColor: 'rgba(255, 255, 255, 0.1)',
                borderWidth: 1
              }
            },
            scales: {
              y: {
                beginAtZero: true,
                grid: { color: 'rgba(255, 255, 255, 0.05)' },
                ticks: { color: '#a0aec0' }
              },
              x: {
                grid: { display: false },
                ticks: { color: '#a0aec0', maxRotation: 45, minRotation: 45 }
              }
            }
          }
        })
      })
    }
  }
})
</script>

<template>
  <div>
    <div v-if="isLoading" class="loading" style="padding: 2rem; text-align: center; color: var(--accent-orange);">
      Loading benchmark data from CI... ⏳
    </div>
    <div v-if="errorMsg" class="error" style="color: #ff5f56; text-align: center; padding: 2rem;">
      {{ errorMsg }}
    </div>
    <div ref="containerRef" class="bench-grid" style="display: grid; grid-template-columns: repeat(auto-fit, minmax(300px, 1fr)); gap: 2rem; margin-top: 2rem;">
    </div>
  </div>
</template>
