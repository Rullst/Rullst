# Tutorial 05: migrações e seeders do banco de dados 🗄️

> **Tradução** da página em inglês [`05-migrations-and-seeds.md`](../../tutorials/05-migrations-and-seeds.md), feita a partir do commit [`1e58b0e89`](https://github.com/Rullst/Rullst/commit/1e58b0e8933f3a257077a45d44056988d52058e6). Se as duas versões divergirem, vale a versão em inglês.

O Rullst usa módulos Rust de migração com timestamp em projetos cujo banco
principal é do SQLx. Projetos com o Turso como banco principal recebem, em vez
disso, instruções `TursoMigration` explícitas e reversíveis. Este tutorial
mostra o caminho do SQLx: o `Schema` funciona apenas com SQLite, PostgreSQL e
MySQL/MariaDB, como lista o [Suporte a bancos de dados](../../database-support.md).

---

## Passo 1: crie uma migração

Execute isto no projeto `shop` do [Tutorial 02](02-cli-generators.md), que já
tem as tabelas `products` e `categories`:

```bash
cargo rullst make:migration create_catalog_items_table
```

O comando cria `src/migrations/m<timestamp>_create_catalog_items_table.rs` e
gera de novo o `src/migrations/mod.rs`. Edite os métodos `up` e `down`
gerados:

```rust,no_run
use rullst_orm::{async_trait, schema::{ForeignKeyAction, Migration, Schema}};

pub struct MigrationImpl;

#[async_trait]
impl Migration for MigrationImpl {
    fn name(&self) -> &'static str {
        "m20260901000000_create_catalog_items_table"
    }

    async fn up(&self) -> Result<(), rullst_orm::Error> {
        Schema::create("catalog_items", |table| {
            table.id();
            table.string("name").not_null();
            // MySQL/MariaDB cannot index a TEXT column, so the indexed SKU
            // has a bounded type.
            table.string("sku").not_null().unique().col_type = "VARCHAR(64)".to_string();
            table
                .foreign_id("category_id", "categories")
                .on_delete(ForeignKeyAction::Restrict);
            table.integer("price_cents").not_null();
            table.check("price_cents >= 0");
            table.date("available_on");
            table.index(&["category_id", "available_on"]);
            table.timestamps();
        })
        .await
    }

    async fn down(&self) -> Result<(), rullst_orm::Error> {
        Schema::drop_if_exists("catalog_items").await
    }
}
```

Mantenha em `name()` o timestamp e o nome gerados; o executor usa esse valor
estável para registrar o estado da migração. A chave estrangeira precisa da
tabela referenciada: `categories` vem de `cargo rullst make:resource Category`,
e esta migração, gerada depois, tem um timestamp posterior e roda depois dela.

`unique()`, `index(...)`, `foreign_id(...)`/`foreign(...)` e `check(...)`
declaram restrições e índices junto com a tabela (veja o
[guia do ORM](../../crates/orm.md#constraints-indexes-and-foreign-keys) para todas
as opções e os nomes gerados). O PostgreSQL e o SQLite criam os índices com
`CREATE INDEX IF NOT EXISTS` logo depois da tabela; o MySQL/MariaDB os declara
dentro do `CREATE TABLE`.

`table.timestamps()` cria as colunas `created_at`/`updated_at`, que aceitam
nulo, guardam instantes em UTC e têm como padrão a hora atual: `TIMESTAMPTZ` no
PostgreSQL, `DATETIME(6)` no MySQL/MariaDB e texto RFC 3339 no SQLite. Leia-as
como campos `rullst::orm::Timestamp` do modelo. `date()`, `time()`,
`timestamp()` e `timestamp_tz()` adicionam colunas individuais de data/hora
para campos `rullst::orm::Date`, `Time`, `DateTime` e `Timestamp`, que
envolvem os tipos do crate `jiff`. Elas
precisam de uma feature de driver estrita (`strict-postgres`, `strict-mysql` ou
`strict-sqlite`), que o `cargo rullst new` escolhe: o pool `Any` do SQLx, com
vários drivers, não consegue ler valores de data/hora, então com ele
`timestamps()` mantém colunas `TEXT` para campos `String`, e os outros helpers
de data/hora falham no PostgreSQL e no MySQL/MariaDB.

Para alterar uma tabela existente, use `Schema::table`:

```rust,no_run
use rullst_orm::schema::{ForeignKeyAction, Schema};

# async fn up() -> Result<(), rullst_orm::Error> {
Schema::table("products", |table| {
    table.integer("category_id");
    table
        .foreign("category_id")
        .references("categories", "id")
        .on_delete(ForeignKeyAction::SetNull);
    table.unique_index(&["name", "category_id"]);
})
.await
# }
```

O SQLite não consegue adicionar restrições a uma tabela existente: lá, uma
chave estrangeira só funciona em uma coluna que aceita nulo adicionada na
mesma chamada, e um CHECK, uma chave estrangeira em uma coluna existente ou uma
coluna `NOT NULL` sem valor padrão devolvem `Error::UnsupportedSchemaChange`
antes de qualquer coisa rodar.

`table.float(...)` corresponde a um campo `f64` do modelo: ele gera
`DOUBLE PRECISION` no PostgreSQL, `DOUBLE` no MySQL/MariaDB e `REAL` no SQLite.
Colunas do PostgreSQL criadas por versões anteriores do Rullst eram `REAL` de
precisão simples; migre-as explicitamente
(`ALTER TABLE products ALTER COLUMN price TYPE DOUBLE PRECISION`) antes de
contar com a precisão do `f64`.

`table.boolean(...)` corresponde a um campo `bool` do modelo. O PostgreSQL
recebe um `BOOLEAN` nativo (um padrão inteiro `0`/`1` vira `FALSE`/`TRUE`); o
MySQL/MariaDB e o SQLite mantêm uma flag `INTEGER` 0/1. Colunas do PostgreSQL
criadas por versões anteriores eram `INTEGER`; migre-as com
`ALTER TABLE t ALTER COLUMN c TYPE BOOLEAN USING c <> 0`.

Padrões de texto (`ColumnDefault::Text`) e as variantes de
`table.enum_col(...)` são incluídos na DDL como literais entre aspas simples,
com as aspas simples duplicadas. `Schema::create` recusa esse texto quando ele
contém uma barra invertida ou um caractere de controle, porque o MySQL/MariaDB
tratam, por padrão, a barra invertida dentro de um literal entre aspas como
caractere de escape.

---

## Passo 2: execute, inspecione e desfaça migrações

```bash
cargo rullst db:migrate
cargo rullst db:status
cargo rullst db:rollback
```

`db:rollback` executa o `down()` do último lote registrado, em ordem inversa.
O executor de migrações do SQLx **não** envolve o lote inteiro em uma única
transação de banco de dados. Por padrão, ele executa cada `up()`/`down()` e
depois o registra na tabela `migrations` em instruções separadas, então uma
queda ou uma conexão perdida no meio deixa uma alteração aplicada mas não
registrada. Uma migração que retorna `true` em `within_transaction()` (API do
pré-lançamento v13) executa o `up()`/`down()` e o registro dentro de uma única
`Orm::transaction`. O template gravado pelo próprio comando artisan
`make:migration` do ORM ativa isso; adicione você mesmo o método às migrações
geradas pelo `cargo rullst make:migration`:

```rust,ignore
fn within_transaction(&self) -> bool {
    true
}
```

Só ficam cobertas as instruções que entram nessa transação: chamadas do
`Schema`, métodos gerados dos modelos, `rullst_orm::execute_query!` e SQL bruto
enviado com `Orm::raw(sql).execute()` (ou `.map_to()`). O `Orm::raw` é o
caminho de SQL bruto que permanece na transação:

```rust,ignore
async fn up(&self) -> Result<(), rullst_orm::Error> {
    Schema::create("tags", |table| {
        table.id();
        table.string("name").not_null();
    })
    .await?;
    // Runs in the migration's transaction, so it sees the new table and rolls
    // back with it.
    Orm::raw("CREATE UNIQUE INDEX tags_name_key ON tags (name)")
        .execute()
        .await?;
    Ok(())
}
```

`Orm::pool()` e `Orm::read_pool()` entregariam outra conexão, que não enxerga
as tabelas ainda não confirmadas da transação e não é desfeita junto com ela.
Desde a 13.0.0-alpha.3 eles devolvem `Error::Validation`, com o nome da
migração, enquanto o `up()`/`down()` de uma migração assim roda, então a
migração falha e é desfeita sem registro, em vez de ficar aplicada pela metade
(o `Orm::begin_transaction()` falha da mesma forma). Mantenha o padrão `false`
em uma migração que envia consultas como
`sqlx::query(sql).execute(Orm::pool()?)`. Um pool obtido antes de a migração
rodar, o pool de outro crate ou uma tarefa iniciada a partir do `up()` não são
detectados e continuam rodando fora da transação. O PostgreSQL e o SQLite
desfazem a DDL junto com o registro; o MySQL e o MariaDB confirmam cada
instrução DDL implicitamente, então lá só as alterações de dados feitas depois
da última instrução DDL são atômicas com o registro. Instruções que se recusam
a rodar em um bloco de transação, como o `CREATE INDEX CONCURRENTLY` do
PostgreSQL, também precisam de `false`. Faça toda migração reversível e teste
as duas direções em cada banco de dados suportado.

Esses comandos usam o mesmo banco de dados que o servidor em execução: o
`DATABASE_URL` do processo, depois o `DATABASE_URL` do `./.env` (sem nunca
sobrescrever o ambiente do processo) e depois o `[database].url` do
`Rullst.toml`. Se nenhum estiver definido, o comando falha em vez de criar um
arquivo SQLite novo, e um banco de dados que não pode ser inicializado também
faz o comando falhar com status de saída 1.

A linha de controle de cada migração é removida assim que o `down()` dela tem
sucesso. Se um `down()` posterior falhar, o rollback para com esse erro: as
migrações que ele já desfez deixam de constar como aplicadas, enquanto a
migração que falhou e o resto do lote continuam registrados. Corrija a causa e
execute `db:rollback` de novo para continuar a partir desse ponto.

`db:migrate` e `db:rollback` mantêm uma trava do executor durante toda a
execução no PostgreSQL (`pg_advisory_lock`) e no MySQL/MariaDB (`GET_LOCK`),
em uma conexão dedicada fora do pool. Assim, réplicas que iniciam ao mesmo
tempo aplicam cada migração pendente uma única vez: um segundo executor
espera e depois encontra a migração já registrada. A trava é liberada quando a
execução termina ou o processo dela sai. Uma tabela `migrations` recém-criada
também declara o nome da migração como `UNIQUE`. O SQLite não tem trava
consultiva entre processos, então rode um único executor de migrações por
banco SQLite.

---

## Passo 3: defina e registre um seeder

Crie o `src/seeds.rs` com um seeder para o modelo `Product` do
[Tutorial 03](03-active-record-crud.md) e declare `mod seeds;` no
`src/main.rs`:

```rust,ignore
use rullst::db::{async_trait, RowSnapshot, Seeder};
use crate::models::product::Product;

pub struct CatalogSeeder;

#[async_trait]
impl Seeder for CatalogSeeder {
    async fn run(&self) -> Result<(), rullst::orm::Error> {
        // Idempotent: a second `db:seed` finds the row and adds nothing.
        let existing = Product::query()
            .where_eq("name", "Starter kit")
            .first()
            .await?;
        if existing.is_some() {
            return Ok(());
        }
        let mut product = Product {
            id: 0,
            name: "Starter kit".to_string(),
            price_cents: 2_500,
            snapshot: RowSnapshot::default(),
        };
        product.save().await
    }
}

pub fn get_seeders() -> Vec<Box<dyn Seeder>> {
    vec![Box::new(CatalogSeeder)]
}
```

Registre as migrações e os seeders antes de iniciar o servidor: no `main()`,
substitua o `rullst::artisan!(crate::migrations::get_migrations());` do
starter por

```rust,ignore
rullst::artisan!(
    crate::migrations::get_migrations(),
    crate::seeds::get_seeders()
);
```

Sem o `artisan!`, o `Server::run` ainda reconhece `db:migrate`, `db:rollback`,
`db:status` e `db:seed`, mas não tem um registro contra o qual executá-los: o
processo sai com status 1 e pede o `rullst::artisan!`, em vez de informar
"Nothing to migrate." como sucesso.

Depois, execute:

```bash
cargo rullst db:migrate
cargo rullst db:seed
```

O `db:seed` primeiro confere as migrações registradas com a tabela
`migrations`. Enquanto houver uma pendente, ele não executa nenhum seeder e
falha, em vez de deixar um seeder falhar em uma tabela que ainda não existe:

```text
Validation error: 1 migration(s) pending, starting with `m20261012_create_users`: run `db:migrate` before `db:seed`
```

Os seeders são executados em sequência. Se o comando puder rodar mais de uma
vez, faça os seeders de desenvolvimento/CI idempotentes, como acima. Nunca faça
commit de senhas reais nem de segredos de provedores; para registros de
autenticação, gere o hash de uma senha de teste com o `rullst-auth` ou crie
uma fixture sem login.

---

## Principais aprendizados

- As migrações são módulos Rust, não arquivos SQL `up/down` separados.
- Os nomes e a ordem das migrações são gerados de forma determinística a
  partir dos timestamps.
- Existe controle por lotes, mas isso não implica rollback transacional do
  lote inteiro.
- O `db:seed` executa apenas os seeders registrados explicitamente com o
  `artisan!`, e só depois que todas as migrações registradas foram aplicadas.
