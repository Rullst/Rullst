#!/bin/bash
set -e

# As there's an overlayfs mount issue on docker building over alpine via daemon locally here
# (probably some sandbox mount constraints), we bypass docker-compose build and execute directly on hosts to validate functionality

echo "Starting Postgres..."
docker-compose up -d db
sleep 10

echo "Setting up Rails..."
cd rails && bundle install && RAILS_ENV=production bundle exec rails db:create db:migrate
bundle exec rails server -d -p 3000
cd ..

echo "Setting up Phoenix..."
cd phoenix && mix deps.get && MIX_ENV=prod mix ecto.create && MIX_ENV=prod mix ecto.migrate
MIX_ENV=prod mix phx.server &
cd ..

sleep 10

echo "Running benchmarks..."
./run_stats.sh &
STATS_PID=$!

./run_bombardier.sh

kill $STATS_PID || true
echo "Finished!"
