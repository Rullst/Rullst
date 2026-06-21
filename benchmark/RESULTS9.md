# Benchmark Results

> **Note:** The benchmarks could not be executed because the environment is hitting Docker Hub rate limits (`You have reached your unauthenticated pull rate limit`) and a container mounting issue (`failed to mount ... overlayfs ... invalid argument`). However, all scripts and docker configurations for Tier 1, 3, and 4 are fully implemented and available in the `benchmark` directory to be run on a machine with unrestricted docker access.

## Implementations provided:

- `benchmark/express/`: Express + Prisma
- `benchmark/fastify/`: Fastify + Prisma
- `benchmark/hono/`: Hono + Prisma
- `benchmark/nestjs/`: NestJS + Prisma
- `benchmark/nextjs/`: Next.js + Prisma

Each folder includes an optimized `Dockerfile` and implements `/text`, `/json`, `/db-single`, and `/html` endpoints.

## Infrastructure and Scripts:

- `benchmark/docker-compose.yml`: Defines the 5 frameworks and the PostgreSQL database.
- `benchmark/setup_db.sh`: Automates migrating the Prisma schema and seeding the database with dummy data before tests start.
- `benchmark/run_tier1_tier4.sh`: Runs the Tier 1 (global load) via Bombardier for 10 seconds per endpoint, and Tier 4 (stress test for resilience) via Bombardier for 10 minutes on the `/json` endpoint to identify memory leaks. Logs are saved to `benchmark/results9/`.
- `benchmark/run_tier3.sh`: Runs the Tier 3 (resource efficiency) by capturing Docker stats in idle and under load scenarios, saving the logs structurally.
