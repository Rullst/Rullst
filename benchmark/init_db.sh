#!/bin/bash
docker compose -f benchmark/docker-compose.yml exec db psql -U user -d benchmark -c "
CREATE TABLE IF NOT EXISTS records (
    id SERIAL PRIMARY KEY,
    name VARCHAR(255) NOT NULL
);
INSERT INTO records (name) VALUES ('Test Record') ON CONFLICT DO NOTHING;
"
