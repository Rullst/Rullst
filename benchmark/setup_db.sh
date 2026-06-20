#!/bin/bash
set -e

# Make sure we're in the benchmark directory
cd "$(dirname "$0")"

echo "Starting PostgreSQL..."
docker compose up -d postgres
echo "Waiting for PostgreSQL to be ready..."
sleep 10 # Give it some time beyond healthcheck

# We will use one of the framework's containers (express) just to migrate and seed
echo "Running Prisma migrate and seed..."
cd express
npm ci
export DATABASE_URL="postgresql://user:password@localhost:5432/db"
npx prisma db push
node seed.js
cd ..

echo "Database setup complete."
