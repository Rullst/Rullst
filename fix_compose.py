import os

frameworks = [
    'actix', 'aspnet-core', 'axum', 'dioxus', 'django', 'express', 'fastapi', 'fastify',
    'go-fiber', 'go-gin', 'hono', 'laravel', 'leptos', 'nestjs', 'nextjs', 'phoenix',
    'poem', 'quarkus', 'rails', 'rocket', 'rullst', 'salvo', 'springboot', 'symfony', 'warp'
]

yml = '''version: \'3.8\'

services:
  db:
    image: postgres:15-alpine
    environment:
      POSTGRES_USER: postgres
      POSTGRES_PASSWORD: postgres
      POSTGRES_DB: bench
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U postgres"]
      interval: 5s
      timeout: 5s
      retries: 5
'''

port = 3001
for fw in frameworks:
    context = f'./{fw}' if os.path.exists(f'benchmark/{fw}/Dockerfile') else '..'
    dockerfile = 'Dockerfile' if os.path.exists(f'benchmark/{fw}/Dockerfile') else f'benchmark/{fw}/Dockerfile'
    
    yml += f'''
  {fw}:
    build:
      context: {context}
      dockerfile: {dockerfile}
    environment:
      - DATABASE_URL=postgres://postgres:postgres@db:5432/bench
      - PORT=3000
    depends_on:
      db:
        condition: service_healthy
    ports:
      - "{port}:3000"
'''
    port += 1

with open('benchmark/docker-compose.yml', 'w') as f:
    f.write(yml)
print('Fixed docker-compose.yml with python!')
