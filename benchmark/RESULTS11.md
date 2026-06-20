# Benchmark Results: Rails and Phoenix

Este documento descreve como rodar os testes de benchmark (Tier 1, Tier 3 e Tier 4) requisitados.
Foram implementados dois contêineres otimizados para produção:
- `rails_app`: Uma aplicação API-only em Ruby on Rails contendo os 4 endpoints (`/text`, `/json`, `/db-single` via ActiveRecord, e `/html` renderizando HTML bruto via `html_safe`).
- `phoenix_app`: Uma aplicação Elixir com Phoenix, rodando os 4 endpoints e configurada com `MIX_ENV=prod`.

## Scripts

1. **`run_bombardier.sh`**: Responsável por fazer o teste de carga usando o contêiner `alpine/bombardier`.
   - Roda a **Carga Global (Tier 1)** para todos os endpoints (`/text`, `/json`, `/db-single`, `/html`) com `-c 100 -d 10s`.
   - Roda o **Estresse Extremo (Tier 4)** no endpoint `/text` com `-c 5000 -d 2m` (reduzido para não estourar tempo de PR, mas configurável no script).

2. **`run_stats.sh`**: Responsável pela **Eficiência de Recursos (Tier 3)**.
   - Enquanto as aplicações estão rodando (em repouso ou sob carga pelo `run_bombardier.sh`), este script em background coleta `CPU` e `Memória RAM` através do comando `docker stats --no-stream` e salva os resultados em `results/stats.csv`.

3. **`run_all.sh`**: Script unificado que inicializa as bases de dados, roda as migrations (Rails e Ecto), dispara o monitoramento e na sequência realiza o benchmark.

## Instruções de uso:

```bash
cd benchmark
./run_all.sh
```

Os resultados detalhados (RPS, Latência e Logs de Stats) estarão disponíveis dentro da pasta `benchmark/results`.
