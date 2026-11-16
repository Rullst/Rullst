# Tutorial 03: operações CRUD com Active Record 🗄️

> **Tradução** da página em inglês [`03-active-record-crud.md`](../../tutorials/03-active-record-crud.md), feita a partir do commit [`654944c44`](https://github.com/Rullst/Rullst/commit/654944c445e464c2b55d4720e6dd70105a07e44d). Se as duas versões divergirem, vale a versão em inglês.

O `rullst-orm` deriva um query builder tipado e métodos de persistência a
partir de uma struct Rust. O modelo baseado no SQLx usa uma chave primária
`i32` chamada `id`; o valor zero significa que `save()` insere, e um valor
diferente de zero significa que ele atualiza.

Este tutorial continua no projeto `shop` do
[Tutorial 02](02-cli-generators.md), cujo `cargo rullst make:model Product
--migration` gravou `src/models/product.rs` e
`src/migrations/m<timestamp>_create_products.rs`.

---

## Passo 1: defina um modelo Active Record

Dê à migração gerada as colunas que o modelo vai usar. No `up()` dela:

```rust,ignore
Schema::create("products", |table| {
    table.id();
    table.string("name").not_null();
    table.integer("price_cents").not_null();
    table.timestamps();
}).await
```

Depois, adicione os campos correspondentes ao `src/models/product.rs`:

```rust
use rullst::db::{Orm, RowSnapshot};

#[derive(Debug, Clone, Orm)]
#[orm(table = "products")]
pub struct Product {
    pub id: i32,
    pub name: String,
    pub price_cents: i32,
    // Remembers the loaded values, so `save()` writes only the columns this
    // record changed. Build new records with `snapshot: RowSnapshot::default()`.
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
```

Aplique a migração com `cargo rullst db:migrate` (ou reinicie o
`cargo rullst dev`) antes de usar o modelo. O modelo deixa de fora
`created_at`/`updated_at`, então as inserções mantêm os valores padrão do banco
de dados. Se o `cargo rullst dev` já aplicou a migração quando ela tinha só um
`id`, execute `cargo rullst db:rollback` (ele desfaz o último lote de
migrações, incluindo esta) e depois `cargo rullst db:migrate`; o
[Tutorial 05](05-migrations-and-seeds.md) explica os dois comandos.

---

## Passo 2: faça as operações CRUD

Chame estes métodos a partir de um handler ou seeder `async`, por exemplo em
um controller do [Tutorial 02](02-cli-generators.md).

### Criar

```rust,no_run
# use rullst::db::{Orm, RowSnapshot};
# #[derive(Debug, Clone, Orm)]
# #[orm(table = "products")]
# struct Product { id: i32, name: String, price_cents: i32, #[sqlx(skip)] snapshot: RowSnapshot }
# async fn create_product() -> Result<(), rullst::orm::Error> {
let mut product = Product {
    id: 0,
    name: "Starter kit".to_string(),
    price_cents: 2_500,
    snapshot: RowSnapshot::default(),
};
product.save().await?;
// `product.id` now contains the inserted primary key.
# Ok(())
# }
```

### Ler e filtrar

```rust,no_run
# use rullst::db::{Orm, RowSnapshot};
# #[derive(Debug, Clone, Orm)]
# #[orm(table = "products")]
# struct Product { id: i32, name: String, price_cents: i32, #[sqlx(skip)] snapshot: RowSnapshot }
# async fn read_products() -> Result<(), rullst::orm::Error> {
let product = Product::find(1).await?; // Result<Option<Product>, rullst::orm::Error>
let kits = Product::query()
    .where_like("name", "%kit%")
    .get()
    .await?;
# let _ = (product, kits);
# Ok(())
# }
```

Os valores do builder são vinculados como parâmetros da consulta. Os nomes de
colunas são validados como identificadores, mas ainda assim devem ser
constantes definidas pela aplicação, e não entrada não confiável da
requisição.

### Atualizar

```rust,no_run
# use rullst::db::{Orm, RowSnapshot};
# #[derive(Debug, Clone, Orm)]
# #[orm(table = "products")]
# struct Product { id: i32, name: String, price_cents: i32, #[sqlx(skip)] snapshot: RowSnapshot }
# async fn update_product() -> Result<(), rullst::orm::Error> {
if let Some(mut product) = Product::find(1).await? {
    product.price_cents = 2_000;
    // The snapshot makes this an UPDATE of `price_cents` only.
    product.save().await?;
}
# Ok(())
# }
```

### Excluir

```rust,no_run
# use rullst::db::{Orm, RowSnapshot};
# #[derive(Debug, Clone, Orm)]
# #[orm(table = "products")]
# struct Product { id: i32, name: String, price_cents: i32, #[sqlx(skip)] snapshot: RowSnapshot }
# async fn delete_product() -> Result<(), rullst::orm::Error> {
if let Some(product) = Product::find(1).await? {
    product.delete().await?;
}
# Ok(())
# }
```

O campo `snapshot` que o `make:model` gravou faz o `save()` de um registro
carregado gravar apenas as colunas que ele alterou. Todo modelo escolhe esse
campo ou `#[orm(full_row_save)]` (todas as colunas, então quem grava por último
vence), senão não compila; veja o
[guia do ORM](../../crates/orm.md#concurrent-updates-and-counters) para as duas
opções, o rastreamento de alterações (dirty tracking) e o `update_partial()`.

---

## Exclusão lógica (soft delete)

Um campo `deleted_at: Option<String>` ativa no modelo o contrato padrão de
exclusão lógica. Para outro valor sentinela, configure-o explicitamente e
garanta que a migração use a mesma representação:

```rust
use rullst_orm::{Orm, RowSnapshot};

#[derive(Debug, Clone, Orm)]
#[orm(
    table = "users",
    soft_delete(field = "is_deleted", value = "0", delval = "1")
)]
pub struct SoftUser {
    pub id: i32,
    pub name: String,
    pub is_deleted: i32,
    #[sqlx(skip)]
    pub snapshot: RowSnapshot,
}
```

`query()` aplica os escopos configurados no modelo. `unscoped()` é uma saída
administrativa explícita e não deve ser usado diretamente com dados da
requisição.

`restore()` limpa a marca de exclusão lógica e `force_delete()` remove a linha
permanentemente. Os dois verificam primeiro o tenant e a política
(`can_restore` / `can_force_delete`) e depois rodam em um savepoint da
`Orm::transaction` com escopo da tarefa, ou em uma transação própria, como o
`delete()`:

- `force_delete()` executa os hooks `before_delete`/`after_delete` e os
  observers `deleting`/`deleted`, grava uma entrada de auditoria
  `force_deleted` em modelos auditáveis e, depois do commit, invalida o cache
  de consultas, publica o evento `deleted` no Redis, chama
  `committed(Deleted)` e remove o documento do Scout. Ele não se propaga para
  as relações `cascade_soft_delete`.
- `restore()` lê de novo a linha restaurada, chama com ela os observers
  `updated` e `saved`, grava uma entrada de auditoria `restored` e, depois do
  commit, invalida o cache, publica os eventos `updated`/`saved` no Redis,
  chama `committed(Updated)` e reindexa o documento no Scout. Os hooks de
  salvamento e os observers `saving`/`updating` não são chamados, porque a
  restauração grava apenas a coluna de exclusão lógica. Restaurar uma linha
  que não existe ou que não foi excluída não faz nada, e excluir de novo uma
  linha que já está na lixeira falha sem alterar a data da exclusão.

---

## Principais aprendizados

- `save`, `find`, `all`, `query` e `delete` são gerados para modelos `Orm`
  baseados no SQLx.
- A criação usa uma struct Rust comum, não um mapa JSON.
- Os erros de CRUD são devolvidos como `rullst::orm::Error` (o tipo
  `rullst_orm::Error`); uma linha inexistente é `Ok(None)`.
  Salvar ou excluir um handle cuja linha foi excluída nesse meio-tempo
  devolve `RecordNotFound` em vez de ter sucesso silenciosamente.
- Use uma transação controlada por quem chama e os métodos `*_with_tx` gerados
  quando uma operação de negócio precisar confirmar várias gravações de forma
  atômica.
