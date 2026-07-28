---
title: Performance Dashboard
---

# Performance Dashboard

Real-time continuous benchmarking. Every commit is measured against historical data to prevent performance regressions.

<script setup>
import BenchmarkDashboard from './.vitepress/components/BenchmarkDashboard.vue'
</script>

<BenchmarkDashboard />

<div class="glassmorphism" style="margin-top: 3rem; padding: 2rem; border-left: 4px solid var(--accent-orange);">
  <h3 style="color: var(--accent-orange); margin-bottom: 1rem; margin-top: 0;">How to read this?</h3>
  <p>
    The charts above represent execution time in <strong>nanoseconds (ns)</strong> per iteration. 
    Lower is better. 
  </p>
  <ul>
    <li style="margin-bottom: 0.8rem;"><strong>cpu/validate_identifier/short:</strong> Measures the overhead of our internal SQL injection defense when validating small column names.</li>
    <li><strong>orm_bench:</strong> Measures the raw throughput of the schema builder and eager-loading engines.</li>
  </ul>
</div>
