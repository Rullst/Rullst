# Crie sua primeira API REST com o Rullst

> **Tradução** da página em inglês [`rest-api-quickstart.md`](../../tutorials/rest-api-quickstart.md), feita a partir do commit [`654944c44`](https://github.com/Rullst/Rullst/commit/654944c445e464c2b55d4720e6dd70105a07e44d). Se as duas versões divergirem, vale a versão em inglês.

**Seu resultado:** uma resposta JSON tipada que você pode verificar a partir
de outro terminal. Comece por aqui quando o seu frontend for uma aplicação
separada, um cliente mobile ou uma integração.
[Explore todos os caminhos para iniciantes](../../start-here.md) ou o
[índice completo da trilha de aprendizado de APIs](../../api-reference.md#httprest-a-connected-learning-path).

Este guia rápido cria um endpoint JSON executável, sem banco de dados nem
provedor de IA. Ele usa o pré-lançamento `13.0.0-rc.2` da v13; a linha v12
chegou ao fim da vida útil (veja o [registro da versão](../../v12.md)). Você
precisa do Rust 1.96.0 ou mais recente, e recomendamos a versão estável mais
recente do Rust (o [Tutorial 01](01-hello-world.md#1-instale-o-rust-e-o-cargo)
mostra como instalá-lo).

## 1. Crie a aplicação

```bash
cargo new first_rullst_api
cd first_rullst_api
cargo add rullst@=13.0.0-rc.2
cargo add tokio --features macros,rt-multi-thread
cargo add serde --features derive
```

O requisito exato mantém este tutorial no pré-lançamento da v13 (o Cargo nunca
escolhe um pré-lançamento sozinho, e a linha v12 chegou ao fim da vida útil em
16 de outubro de 2026). Faça commit do `Cargo.lock` gerado, para que as
próximas compilações com `--locked` usem as mesmas versões de pacotes
resolvidas; ele não fixa nenhuma branch Git em movimento sem relação com isso
nem atualiza a aplicação automaticamente.

## 2. Adicione uma rota JSON

Substitua o `src/main.rs` por:

```rust,no_run
use rullst::{Server, ServerError, routes, server::Json};
use serde::Serialize;

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    framework: &'static str,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        framework: "Rullst",
    })
}

#[tokio::main]
async fn main() -> Result<(), ServerError> {
    let app = routes![
        get("/api/health" => health),
    ];

    Server::new(app).run(3000).await
}
```

A resposta é serializada a partir de um valor Rust tipado. O `Server`
adiciona os cabeçalhos seguros e a verificação de CSRF em todos os ambientes;
transporte, proxy, autenticação, autorização e a política de entrada
específica da aplicação continuam sendo responsabilidade do deploy, então uma
resposta JSON sozinha não é uma fronteira de segurança de produção.

## 3. Execute e verifique

Inicie a aplicação:

```bash
cargo run
```

Em outro terminal, faça uma requisição à rota:

```bash
curl -i http://127.0.0.1:3000/api/health
```

O corpo é:

```json
{"status":"ok","framework":"Rullst"}
```

Pare o servidor com `Ctrl+C`.

## 4. Prefere uma base de API gerada?

Depois de [instalar a CLI da v13](../1-getting-started.md#1-instalação), ela
pode gerar uma API headless diretamente. Use um diretório diferente do exemplo
escrito à mão acima:

```bash
cargo rullst new generated_api --default --api --no-database \
  --skip-initial-migration
cd generated_api
cargo run
```

Este é o starter API (o starter Blank sem HTML). As rotas dele são registradas
em `router()`, no `src/main.rs`, e não copiadas do handler de saúde acima:
`GET /` responde com uma mensagem de boas-vindas em JSON
(`curl http://127.0.0.1:3000/`), `POST /api/messages` exige
`Authorization: Bearer <API_TOKEN>` com o token aleatório gravado no `.env`, e
`/health` e `/ready` atendem às sondas do contêiner. Use
`cargo rullst make:controller project --api` para controllers JSON
adicionais. Rotas de dados parametrizadas geradas ainda exigem autorização
explícita por dono ou por tenant; veja [RBAC e proteção contra IDOR](../../tutorials/13-rbac-authorization.md).
