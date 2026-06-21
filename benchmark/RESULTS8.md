# Relatório de Benchmarks (Tier 1, 3 e 4)

Devido aos limites de taxa (rate limits) do Docker Hub no ambiente de sandbox, a execução completa e real dos containers e dos testes do `bombardier` não pôde ser finalizada durante esta sessão automatizada para gerar logs literais no arquivo, mas a infraestrutura e os scripts foram criados com sucesso para serem executados no ambiente do usuário.

Abaixo estão as instruções e os arquivos gerados.

## Estrutura Criada

1. **go-gin/**: Aplicação em Go usando o framework Gin e o ORM GORM, com um `Dockerfile` otimizado.
2. **go-fiber/**: Aplicação em Go usando o framework Fiber e o ORM GORM, com um `Dockerfile` otimizado.
3. **aspnet-core/**: Aplicação em C# (ASP.NET Core 10) usando o Entity Framework Core (Npgsql), com um `Dockerfile` otimizado e AOT/PGO ativados para produção.
4. **docker-compose.yml**: Arquivo para orquestrar os três frameworks e o PostgreSQL, garantindo o banco de dados saudável antes de iniciar os web servers.
5. **run_load_tests.sh**: Script de automação que executa o `bombardier` em loop para todos os containers e rotas. Cobre o **Tier 1** (Carga Global: 10s, 125 conexões) e o **Tier 4** (Estresse extremo: 1m/10m, 1000 conexões) focado em procurar memory leaks.
6. **run_resource_tests.sh**: Script que orquestra o **Tier 3**. Captura os status dos containers via `docker stats` no momento de repouso (idle) e, em seguida, dispara um ataque em todos simultaneamente e coleta o status de CPU e RAM no pico da carga.

## Como Executar no seu ambiente

Para gerar os resultados oficiais na sua máquina, execute os scripts a partir da pasta raiz do projeto:

```bash
cd benchmark
./run_load_tests.sh
./run_resource_tests.sh
```

Os resultados detalhados serão salvos na pasta `benchmark/results/`.
