//! Reproducible single-connection SQLite comparison with Diesel and SeaORM.
//!
//! Run a quick smoke measurement with:
//! `cargo bench -p rullst-orm --features strict-sqlite --bench orm_comparison -- --quick`
//!
//! The three implementations use typed SQLite drivers, distinct files but the
//! same schema, index, row set, journal/synchronous policy and logical
//! operations. Results include each framework's normal executor/connection
//! behavior and are not universal production-database or application
//! benchmarks.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use diesel::connection::SimpleConnection;
use diesel::prelude::*;
use rullst_orm::schema::{Blueprint, Schema};
use rullst_orm::{FromRow, Orm};
use sea_orm::entity::prelude::*;
use sea_orm::{ActiveModelTrait, ConnectOptions, ConnectionTrait, Database, QuerySelect, Set};
use std::path::{Path, PathBuf};
use std::time::Duration;

const ROW_COUNT: i32 = 100;
const SQLITE_POLICY: &str =
    "PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL; PRAGMA busy_timeout = 5000;";

#[derive(Debug, Clone, FromRow, rullst_orm::Orm)]
#[orm(table = "bench_users")]
struct RullstUser {
    pub id: i32,
    pub name: String,
    pub email: String,
}

mod diesel_schema {
    diesel::table! {
        bench_users (id) {
            id -> Integer,
            name -> Text,
            email -> Text,
        }
    }
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = diesel_schema::bench_users)]
struct DieselUser {
    id: i32,
    name: String,
    email: String,
}

#[derive(Insertable)]
#[diesel(table_name = diesel_schema::bench_users)]
struct NewDieselUser<'a> {
    name: &'a str,
    email: &'a str,
}

mod sea_user {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "bench_users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        pub email: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

struct Harness {
    runtime: tokio::runtime::Runtime,
    diesel: diesel::SqliteConnection,
    sea: sea_orm::DatabaseConnection,
}

fn database_path(name: &str) -> PathBuf {
    Path::new("target").join(format!("orm-comparison-{name}.db"))
}

fn reset_database(path: &Path) {
    if path.exists() {
        std::fs::remove_file(path).expect("remove previous comparison database");
    }
}

fn sqlite_url(path: &Path) -> String {
    format!("sqlite:{}?mode=rwc", path.display())
}

async fn setup_rullst(path: &Path) {
    reset_database(path);
    Orm::init_with_options(&sqlite_url(path), 1, 5)
        .await
        .expect("initialize Rullst comparison database");
    rullst_orm::sqlx::query(SQLITE_POLICY)
        .execute(Orm::pool().expect("Rullst comparison pool"))
        .await
        .expect("apply Rullst SQLite policy");
    Schema::create("bench_users", |table: &mut Blueprint| {
        table.id();
        table.string("name").not_null();
        table.string("email").not_null();
    })
    .await
    .expect("create Rullst comparison schema");
    rullst_orm::sqlx::query("CREATE UNIQUE INDEX bench_users_email_idx ON bench_users(email)")
        .execute(Orm::pool().expect("Rullst comparison pool"))
        .await
        .expect("create Rullst comparison index");
    for id in 1..=ROW_COUNT {
        let mut user = RullstUser {
            id: 0,
            name: format!("User{id}"),
            email: format!("user{id}@example.test"),
        };
        user.save().await.expect("seed Rullst comparison row");
    }
}

fn setup_diesel(path: &Path) -> diesel::SqliteConnection {
    reset_database(path);
    let mut connection = diesel::SqliteConnection::establish(&path.display().to_string())
        .expect("initialize Diesel comparison database");
    connection
        .batch_execute(&format!(
            "{SQLITE_POLICY} CREATE TABLE bench_users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, email TEXT NOT NULL); CREATE UNIQUE INDEX bench_users_email_idx ON bench_users(email);"
        ))
        .expect("create Diesel comparison schema");
    connection
        .transaction::<_, diesel::result::Error, _>(|connection| {
            for id in 1..=ROW_COUNT {
                let name = format!("User{id}");
                let email = format!("user{id}@example.test");
                diesel::insert_into(diesel_schema::bench_users::table)
                    .values(NewDieselUser {
                        name: &name,
                        email: &email,
                    })
                    .execute(connection)?;
            }
            Ok(())
        })
        .expect("seed Diesel comparison rows");
    connection
}

async fn setup_sea(path: &Path) -> sea_orm::DatabaseConnection {
    reset_database(path);
    let mut options = ConnectOptions::new(sqlite_url(path));
    options
        .max_connections(1)
        .min_connections(1)
        .sqlx_logging(false);
    let connection = Database::connect(options)
        .await
        .expect("initialize SeaORM comparison database");
    connection
        .execute_unprepared(&format!(
            "{SQLITE_POLICY} CREATE TABLE bench_users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL, email TEXT NOT NULL); CREATE UNIQUE INDEX bench_users_email_idx ON bench_users(email);"
        ))
        .await
        .expect("create SeaORM comparison schema");
    let rows = (1..=ROW_COUNT).map(|id| sea_user::ActiveModel {
        id: sea_orm::ActiveValue::NotSet,
        name: Set(format!("User{id}")),
        email: Set(format!("user{id}@example.test")),
    });
    sea_user::Entity::insert_many(rows)
        .exec(&connection)
        .await
        .expect("seed SeaORM comparison rows");
    connection
}

fn setup() -> Harness {
    std::fs::create_dir_all("target").expect("create benchmark target directory");
    let runtime = tokio::runtime::Runtime::new().expect("comparison Tokio runtime");
    runtime.block_on(setup_rullst(&database_path("rullst")));
    let diesel = setup_diesel(&database_path("diesel"));
    let sea = runtime.block_on(setup_sea(&database_path("seaorm")));
    Harness {
        runtime,
        diesel,
        sea,
    }
}

fn benchmark_reads(c: &mut Criterion, harness: &mut Harness) {
    use diesel_schema::bench_users::dsl as diesel_users;
    use sea_user::{Column as SeaColumn, Entity as SeaEntity};

    let mut group = c.benchmark_group("orm_comparison/sqlite");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(30);

    group.bench_function(BenchmarkId::new("find_by_id", "rullst"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            std::hint::black_box(RullstUser::find(50).await.expect("Rullst find"));
        });
    });
    group.bench_function(BenchmarkId::new("find_by_id", "diesel"), |bencher| {
        bencher.iter(|| {
            let row = diesel_users::bench_users
                .find(50)
                .select(DieselUser::as_select())
                .first::<DieselUser>(&mut harness.diesel)
                .expect("Diesel find");
            std::hint::black_box((row.id, row.name, row.email));
        });
    });
    group.bench_function(BenchmarkId::new("find_by_id", "seaorm"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            std::hint::black_box(
                SeaEntity::find_by_id(50)
                    .one(&harness.sea)
                    .await
                    .expect("SeaORM find"),
            );
        });
    });

    group.bench_function(BenchmarkId::new("filter_email", "rullst"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            std::hint::black_box(
                RullstUser::query()
                    .where_eq("email", "user50@example.test")
                    .first()
                    .await
                    .expect("Rullst filtered read"),
            );
        });
    });
    group.bench_function(BenchmarkId::new("filter_email", "diesel"), |bencher| {
        bencher.iter(|| {
            let row = diesel_users::bench_users
                .filter(diesel_users::email.eq("user50@example.test"))
                .select(DieselUser::as_select())
                .first::<DieselUser>(&mut harness.diesel)
                .expect("Diesel filtered read");
            std::hint::black_box((row.id, row.name, row.email));
        });
    });
    group.bench_function(BenchmarkId::new("filter_email", "seaorm"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            std::hint::black_box(
                SeaEntity::find()
                    .filter(SeaColumn::Email.eq("user50@example.test"))
                    .one(&harness.sea)
                    .await
                    .expect("SeaORM filtered read"),
            );
        });
    });

    group.finish();
}

fn benchmark_collections(c: &mut Criterion, harness: &mut Harness) {
    use diesel_schema::bench_users::dsl as diesel_users;
    use sea_user::Entity as SeaEntity;

    let mut group = c.benchmark_group("orm_comparison/sqlite");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(30);

    group.bench_function(BenchmarkId::new("count", "rullst"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            std::hint::black_box(RullstUser::query().count().await.expect("Rullst count"));
        });
    });
    group.bench_function(BenchmarkId::new("count", "diesel"), |bencher| {
        bencher.iter(|| {
            std::hint::black_box(
                diesel_users::bench_users
                    .count()
                    .get_result::<i64>(&mut harness.diesel)
                    .expect("Diesel count"),
            );
        });
    });
    group.bench_function(BenchmarkId::new("count", "seaorm"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            std::hint::black_box(
                SeaEntity::find()
                    .count(&harness.sea)
                    .await
                    .expect("SeaORM count"),
            );
        });
    });

    group.bench_function(BenchmarkId::new("list_10", "rullst"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            std::hint::black_box(
                RullstUser::query()
                    .order_by("id")
                    .limit(10)
                    .get()
                    .await
                    .expect("Rullst list"),
            );
        });
    });
    group.bench_function(BenchmarkId::new("list_10", "diesel"), |bencher| {
        bencher.iter(|| {
            let rows = diesel_users::bench_users
                .order(diesel_users::id.asc())
                .limit(10)
                .select(DieselUser::as_select())
                .load::<DieselUser>(&mut harness.diesel)
                .expect("Diesel list");
            std::hint::black_box(rows);
        });
    });
    group.bench_function(BenchmarkId::new("list_10", "seaorm"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            std::hint::black_box(
                SeaEntity::find()
                    .order_by_id_asc()
                    .limit(10)
                    .all(&harness.sea)
                    .await
                    .expect("SeaORM list"),
            );
        });
    });

    group.finish();
}

fn benchmark_insert_delete(c: &mut Criterion, harness: &mut Harness) {
    use diesel_schema::bench_users::dsl as diesel_users;

    let mut group = c.benchmark_group("orm_comparison/sqlite");
    group.warm_up_time(Duration::from_secs(2));
    group.measurement_time(Duration::from_secs(5));
    group.sample_size(30);

    group.bench_function(BenchmarkId::new("insert_delete", "rullst"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            let mut user = RullstUser {
                id: 0,
                name: "Transient".to_string(),
                email: "transient-rullst@example.test".to_string(),
            };
            user.save().await.expect("Rullst insert");
            user.delete().await.expect("Rullst delete");
        });
    });
    group.bench_function(BenchmarkId::new("insert_delete", "diesel"), |bencher| {
        bencher.iter(|| {
            let inserted_id = diesel::insert_into(diesel_users::bench_users)
                .values(NewDieselUser {
                    name: "Transient",
                    email: "transient-diesel@example.test",
                })
                .returning(diesel_users::id)
                .get_result::<i32>(&mut harness.diesel)
                .expect("Diesel insert");
            diesel::delete(diesel_users::bench_users.find(inserted_id))
                .execute(&mut harness.diesel)
                .expect("Diesel delete");
        });
    });
    group.bench_function(BenchmarkId::new("insert_delete", "seaorm"), |bencher| {
        bencher.to_async(&harness.runtime).iter(|| async {
            let inserted = sea_user::ActiveModel {
                id: sea_orm::ActiveValue::NotSet,
                name: Set("Transient".to_string()),
                email: Set("transient-seaorm@example.test".to_string()),
            }
            .insert(&harness.sea)
            .await
            .expect("SeaORM insert");
            sea_user::Entity::delete_by_id(inserted.id)
                .exec(&harness.sea)
                .await
                .expect("SeaORM delete");
        });
    });

    group.finish();
}

fn benchmark_comparison(c: &mut Criterion) {
    let mut harness = setup();
    benchmark_reads(c, &mut harness);
    benchmark_collections(c, &mut harness);
    benchmark_insert_delete(c, &mut harness);
}

criterion_group!(benches, benchmark_comparison);
criterion_main!(benches);
