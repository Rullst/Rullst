#!/usr/bin/env python3
"""Compile a standalone ORM consumer and reject unrelated SQLx edges.

The consumer depends on nothing but `rullst-orm`: code that `#[derive(Orm)]`
generates, its `FromRow` included, must reach SQLx, `tracing` and its other
dependencies through `rullst_orm`. The graph must hold only the selected SQLx
driver and no SQLx macro crate (`sqlx-macros`, `sqlx-macros-core`).
"""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile


DRIVERS = {"strict-postgres": "postgres", "strict-mysql": "mysql", "strict-sqlite": "sqlite"}
SOURCE = """use rullst_orm::Orm;
use rullst_orm::sqlx::types::JsonValue;

#[derive(Debug, Clone, Copy, rullst_orm::Enum)]
#[rullst_enum(type_name = "consumer_state", rename_all = "snake_case")]
pub enum State { Active, Disabled }

#[derive(Debug, Clone, Orm)]
#[orm(table = "consumer_records")]
pub struct Record {
    pub id: i32,
    pub name: String,
}

// Auditable, tenant-scoped and soft-deleting, so every generated query span
// (restore and force_delete included) is compiled too.
#[derive(Debug, Clone, Orm)]
#[orm(table = "consumer_notes", tenant_column = "tenant_id", auditable)]
pub struct Note {
    pub id: i32,
    pub tenant_id: String,
    pub body: String,
    pub deleted_at: Option<String>,
}

// Every `#[sqlx(...)]` option the derived `FromRow` reads, decoded from the
// selected driver's concrete row type.
#[derive(Debug, Clone, Default, Orm)]
#[orm(table = "consumer_profiles")]
#[sqlx(default)]
pub struct Profile {
    pub id: i32,
    #[sqlx(json)]
    pub settings: JsonValue,
    #[sqlx(json(nullable))]
    pub extra: Option<JsonValue>,
    #[sqlx(default)]
    pub views: i64,
    #[sqlx(skip)]
    pub cache: String,
}

// Compile real generated entrypoints against the selected concrete executor.
// Live connection/rollback contracts run separately in the database matrix.
pub async fn exercise(record: &mut Record) -> Result<(), rullst_orm::Error> {
    record.save().await?;
    let _ = Record::find(record.id).await?;
    let _ = Record::query().where_like("name", "%fixture%").limit(3).get().await?;
    let mut transaction = Orm::begin_transaction().await?;
    record.save_with_tx(&mut transaction).await?;
    record.update_partial().name("patched".into()).save_with_tx(&mut transaction).await?;
    let _ = Record::find_with_tx(record.id, &mut transaction).await?;
    transaction.rollback().await?;
    record.delete().await
}

pub async fn exercise_note(note: &mut Note) -> Result<(), rullst_orm::Error> {
    note.save().await?;
    let _ = Note::query().paginate(1, 10).await?;
    Note::query().chunk_by_id(10, |_| async { Ok(()) }).await?;
    note.delete().await?;
    note.restore().await?;
    note.force_delete().await
}

pub async fn exercise_profile(profile: &mut Profile) -> Result<Option<Profile>, rullst_orm::Error> {
    profile.save().await?;
    let _ = Profile::query().where_eq("views", 0_i64).get().await?;
    Profile::find(profile.id).await
}

pub async fn exercise_enum() -> Result<State, rullst_orm::Error> {
    rullst_orm::sqlx::query("INSERT INTO consumer_states (state) VALUES (__PLACEHOLDER__)")
        .bind(State::Active).execute(Orm::pool()?).await?;
    Ok(rullst_orm::sqlx::query_scalar::<_, State>("SELECT state FROM consumer_states LIMIT 1")
        .fetch_one(Orm::pool()?).await?)
}
"""


def run(command, directory, *, capture=False):
    return subprocess.run(command, cwd=directory, check=True, text=True, capture_output=capture)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("feature", choices=DRIVERS)
    parser.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    with tempfile.TemporaryDirectory(prefix="rullst-orm-driver-consumer-") as temporary:
        project = Path(temporary)
        (project / "src").mkdir()
        (project / "Cargo.toml").write_text(
            '[package]\nname = "rullst-orm-driver-consumer"\nversion = "0.0.0"\n'
            'edition = "2024"\npublish = false\n[workspace]\n[dependencies]\n'
            f'rullst-orm = {{ path = {json.dumps(str(root / "rullst-orm"))}, '
            f'default-features = false, features = [{json.dumps(args.feature)}] }}\n'
            '[lints.rust]\nunexpected_cfgs = { level = "warn", check-cfg = [\'cfg(feature, values("redis"))\'] }\n',
            encoding="utf-8",
        )
        placeholder = "$1" if args.feature == "strict-postgres" else "?"
        (project / "src/lib.rs").write_text(SOURCE.replace("__PLACEHOLDER__", placeholder), encoding="utf-8")
        # Reuse audited dependency resolutions; Cargo removes unused workspace
        # packages when admitting this separate consumer into its own lockfile.
        shutil.copyfile(root / "Cargo.lock", project / "Cargo.lock")
        network = ["--offline"] if args.offline else []
        run(["cargo", "metadata", "--format-version", "1", *network], project, capture=True)
        graph = run(
            ["cargo", "tree", "--locked", "--edges", "normal,build", "--prefix", "none",
             "--format", "{p}", *network], project, capture=True,
        ).stdout
        packages = {line.split()[0] for line in graph.splitlines() if line.strip()}
        expected = f"sqlx-{DRIVERS[args.feature]}"
        actual = packages.intersection({"sqlx-postgres", "sqlx-mysql", "sqlx-sqlite"})
        if actual != {expected}:
            raise SystemExit(f"{args.feature}: expected only {expected}, found {sorted(actual)}")
        macros = sorted(packages.intersection({"sqlx-macros", "sqlx-macros-core"}))
        if macros:
            raise SystemExit(f"{args.feature}: the ORM must not build SQLx macros, found {macros}")
        # A shared target is optional and caller-owned, never created under the
        # temporary project when CI/local policy supplies CARGO_TARGET_DIR.
        run(["cargo", "clippy", "--locked", "--lib", *network, "--", "-D", "warnings"], project)
        print(f"PASS {args.feature}: generated CRUD/transaction/enum/FromRow consumer, only {expected}, no SQLx macros")


if __name__ == "__main__":
    main()
