#!/usr/bin/env python3
"""Compile a standalone ORM consumer and reject unrelated SQLx driver edges."""

import argparse
import json
from pathlib import Path
import shutil
import subprocess
import tempfile
import tomllib


DRIVERS = {"strict-postgres": "postgres", "strict-mysql": "mysql", "strict-sqlite": "sqlite"}
SOURCE = """use rullst_orm::{FromRow, Orm};

#[derive(Debug, Clone, Copy, rullst_orm::Enum)]
#[rullst_enum(type_name = "consumer_state", rename_all = "snake_case")]
pub enum State { Active, Disabled }

#[derive(Debug, Clone, FromRow, Orm)]
#[orm(table = "consumer_records")]
pub struct Record {
    pub id: i32,
    pub name: String,
}

// Compile real generated entrypoints against the selected concrete executor.
// Live connection/rollback contracts run separately in the database matrix.
pub async fn exercise(record: &mut Record) -> Result<(), rullst_orm::Error> {
    record.save().await?;
    let _ = Record::find(record.id).await?;
    let _ = Record::query().where_like("name", "%fixture%").limit(3).get().await?;
    let mut transaction = Orm::begin_transaction().await?;
    record.save_with_tx(&mut transaction).await?;
    let _ = Record::find_with_tx(record.id, &mut transaction).await?;
    transaction.rollback().await?;
    record.delete().await
}

pub async fn exercise_enum() -> Result<State, rullst_orm::Error> {
    sqlx::query("INSERT INTO consumer_states (state) VALUES (__PLACEHOLDER__)")
        .bind(State::Active).execute(Orm::pool()?).await?;
    Ok(sqlx::query_scalar::<_, State>("SELECT state FROM consumer_states LIMIT 1")
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
    workspace = tomllib.loads((root / "Cargo.toml").read_text(encoding="utf-8"))["workspace"]["dependencies"]
    macro_dependencies = ""
    for name in ["sqlx", "tokio", "tracing"]:
        entry = workspace[name]
        version = entry["version"] if isinstance(entry, dict) else entry
        macro_dependencies += f'{name} = {{ version = {json.dumps(version)}, default-features = false }}\n'
    with tempfile.TemporaryDirectory(prefix="rullst-orm-driver-consumer-") as temporary:
        project = Path(temporary)
        (project / "src").mkdir()
        (project / "Cargo.toml").write_text(
            '[package]\nname = "rullst-orm-driver-consumer"\nversion = "0.0.0"\n'
            'edition = "2024"\npublish = false\n[workspace]\n[dependencies]\n'
            f'rullst-orm = {{ path = {json.dumps(str(root / "rullst-orm"))}, '
            f'default-features = false, features = [{json.dumps(args.feature)}] }}\n'
            + macro_dependencies
            + '[lints.rust]\nunexpected_cfgs = { level = "warn", check-cfg = [\'cfg(feature, values("redis"))\'] }\n',
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
        # A shared target is optional and caller-owned, never created under the
        # temporary project when CI/local policy supplies CARGO_TARGET_DIR.
        run(["cargo", "clippy", "--locked", "--lib", *network, "--", "-D", "warnings"], project)
        print(f"PASS {args.feature}: generated CRUD/transaction/enum consumer, only {expected}")


if __name__ == "__main__":
    main()
