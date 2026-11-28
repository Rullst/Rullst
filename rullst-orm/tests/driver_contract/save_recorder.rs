//! Records what a save sends: the transactions it opens (the
//! `rullst.orm.transaction.begin` span of `Orm::begin_transaction`) and the
//! statements SQLx executes (`sqlx::query` events, which leave out `BEGIN`,
//! `SAVEPOINT`, `COMMIT` and `ROLLBACK`). Only statements on the tables a
//! test names are kept, with PostgreSQL's `$n` placeholders written as `?`.

use std::fmt::Debug;
use std::sync::{Arc, Mutex, PoisonError};

use tracing::field::{Field, Visit};
use tracing::{Event, Subscriber, span};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::prelude::*;

/// What one recorded window saw.
#[derive(Debug, Default, PartialEq)]
pub struct Recorded {
    pub begins: usize,
    pub statements: Vec<String>,
}

#[derive(Clone)]
pub struct Recorder {
    seen: Arc<Mutex<Recorded>>,
    prefixes: &'static [&'static str],
}

impl Recorder {
    /// A recorder of statements whose SQL names a table starting with one of
    /// `prefixes`.
    pub fn new(prefixes: &'static [&'static str]) -> Self {
        Self {
            seen: Arc::default(),
            prefixes,
        }
    }

    /// Installs the recorder as this process's subscriber, which also sees
    /// the statements SQLite runs on its worker threads.
    #[allow(dead_code)]
    pub fn install_global(prefixes: &'static [&'static str]) -> Self {
        let recorder = Self::new(prefixes);
        tracing::subscriber::set_global_default(
            tracing_subscriber::registry().with(recorder.clone()),
        )
        .expect("the recorder is this test binary's only global subscriber");
        recorder
    }

    /// Installs the recorder for the current thread until the guard drops.
    /// It sees what runs on that thread: on a current-thread runtime, every
    /// PostgreSQL and MySQL statement, but not SQLite's.
    #[allow(dead_code)]
    pub fn install_local(
        prefixes: &'static [&'static str],
    ) -> (Self, tracing::subscriber::DefaultGuard) {
        let recorder = Self::new(prefixes);
        let guard =
            tracing::subscriber::set_default(tracing_subscriber::registry().with(recorder.clone()));
        (recorder, guard)
    }

    /// Returns what was recorded since the last call and starts over.
    pub fn take(&self) -> Recorded {
        std::mem::take(&mut *self.seen.lock().unwrap_or_else(PoisonError::into_inner))
    }

    fn keeps(&self, sql: &str) -> bool {
        sql.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .any(|word| self.prefixes.iter().any(|prefix| word.starts_with(prefix)))
    }
}

impl<S: Subscriber> Layer<S> for Recorder {
    fn on_new_span(&self, attributes: &span::Attributes<'_>, _id: &span::Id, _ctx: Context<'_, S>) {
        if attributes.metadata().name() == "rullst.orm.transaction.begin" {
            self.seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .begins += 1;
        }
    }

    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        if event.metadata().target() != "sqlx::query" {
            return;
        }
        let mut statement = Statement::default();
        event.record(&mut statement);
        let sql = statement.sql();
        if self.keeps(&sql) {
            self.seen
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .statements
                .push(sql);
        }
    }
}

/// The fields of one `sqlx::query` event: the first words of the statement,
/// and the whole statement when it is longer.
#[derive(Default)]
struct Statement {
    summary: String,
    full: String,
}

impl Statement {
    fn sql(&self) -> String {
        let text = if self.full.trim().is_empty() {
            &self.summary
        } else {
            &self.full
        };
        let text = text.trim().trim_end_matches(" …");
        let mut sql = String::with_capacity(text.len());
        let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
        let mut chars = text.chars().peekable();
        while let Some(c) = chars.next() {
            if c == '$' && chars.peek().is_some_and(char::is_ascii_digit) {
                while chars.peek().is_some_and(char::is_ascii_digit) {
                    chars.next();
                }
                sql.push('?');
            } else {
                sql.push(c);
            }
        }
        sql
    }
}

impl Visit for Statement {
    fn record_str(&mut self, field: &Field, value: &str) {
        match field.name() {
            "summary" => self.summary = value.to_string(),
            "db.statement" => self.full = value.to_string(),
            _ => {}
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn Debug) {
        let value = format!("{value:?}");
        let value = value.trim_matches('"');
        self.record_str(field, value);
    }
}
