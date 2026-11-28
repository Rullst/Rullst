# Tutorial 02: automação e geradores da CLI ⚡

> **Tradução** da página em inglês [`02-cli-generators.md`](../../tutorials/02-cli-generators.md), feita a partir do commit [`654944c44`](https://github.com/Rullst/Rullst/commit/654944c445e464c2b55d4720e6dd70105a07e44d). Se as duas versões divergirem, vale a versão em inglês.

**Seu objetivo:** gerar uma pequena parte da aplicação, encontrar os arquivos
resultantes e entender o que ainda precisa do seu código. Você precisa da CLI
instalada no [Tutorial 01](01-hello-world.md#6-continue-com-a-cli)
(`cargo rullst --version` mostra `rullst 13.0.0-rc.2`).

O Rullst oferece geradores de código opinativos (`make:*`) para controllers,
modelos, migrações e recursos. Os geradores atuais registram os módulos Rust
gerados quando possível; eles não adicionam rotas à aplicação.
`make:controller` e `make:model` também atualizam o mapa do projeto gerado
(`.llms.txt`, `.rullst/context-map.json`) e o `diagram.md`; versões desses
arquivos escritas à mão são mantidas. Revise cada arquivo gerado e monte você
mesmo as rotas desejadas.

---

## Crie o projeto

Crie um starter Blank com SQLite e entre nele:

```bash
cargo rullst new shop --default --blueprint blank --database sqlite \
  --skip-initial-migration
cd shop
```

`--default` pula as perguntas do assistente. O projeto tem `Cargo.toml`,
`Rullst.toml`, `.env`, `src/main.rs` (as rotas, em `router()`) e uma primeira
migração de `users`. Execute todos os comandos abaixo a partir deste
diretório; os próximos tutoriais usam o mesmo projeto, a não ser que digam o
contrário. Para ver a página de boas-vindas, execute `cargo rullst dev` e abra
`http://127.0.0.1:3000`; a primeira compilação pode levar vários minutos.

---

## 🛠️ Comandos essenciais de scaffolding

Execute os três comandos em ordem: cada um adiciona arquivos diferentes.

### 1. Gere um controller
```bash
cargo rullst make:controller ProductsController
```
Cria `src/controllers/products_controller.rs` com handlers provisórios
`index`, `show`, `store`, `update` e `delete` e registra o módulo
(`pub mod controllers;` no `src/main.rs`). Monte a página dele adicionando uma
linha à lista `routes![...]` em `router()`:

```rust,ignore
get("/products" => controllers::products_controller::index),
```

Depois que o `cargo rullst dev` recompilar, `http://127.0.0.1:3000/products`
mostra a página provisória gerada.

### 2. Gere um modelo e uma migração
```bash
cargo rullst make:model Product --migration
```
Cria o modelo em `src/models/product.rs` e uma migração Rust com timestamp
(`src/migrations/m<timestamp>_create_products.rs`), registrada pelo
`src/migrations/mod.rs`. O modelo começa com um campo `id` e um campo
`snapshot`, a migração com `id` e timestamps; o
[Tutorial 03](03-active-record-crud.md) adiciona as colunas. O
`cargo rullst dev` aplica as migrações pendentes ao iniciar;
`cargo rullst db:migrate` as aplica sem iniciar o servidor
([Tutorial 05](05-migrations-and-seeds.md)).

### 3. Gere o ponto de partida de um recurso full-stack
```bash
cargo rullst make:resource Category
```
Gera, em um só comando, um modelo (`src/models/category.rs`), a migração
dele, um controller (`src/controllers/category_controller.rs`) e views HTML
provisórias (`views/category/index.html` e `views/category/form.html`). Use um
nome que ainda não tenha modelo: para um modelo ou tabela existente, o comando
pula a migração de criação da tabela. O controller gerado não é uma
implementação de CRUD completa e autorizada.

---

## 🔍 Inspeção estática de código pela CLI

Inspecione declarações reconhecíveis do código-fonte sem iniciar um servidor:

```bash
# Inspect recognizable routes! declarations
cargo rullst inspect routes

# Print ORM models and field types
cargo rullst inspect models
```

Adicione `--json` para um documento versionado e legível por máquinas (veja a
[referência da CLI](../../cli_reference.md)).

---

## 💡 Principais aprendizados

- Os geradores de scaffolding mantêm a consistência da arquitetura entre as
  pessoas do time.
- `cargo rullst inspect` faz uma inspeção limitada do texto do código-fonte.
  A saída de rotas reconhece entradas explícitas de uma linha em `routes!`
  (`get`, `post`, `put` e `delete`); a saída de modelos lista as declarações
  públicas de `src/models`. Não é uma expansão completa de macros nem uma
  análise semântica de Rust.
- Os geradores nunca adicionam rotas: monte você mesmo cada handler em
  `router()`.
