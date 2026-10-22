//! Rustdoc of the generated `save()`.

/// Rustdoc of the generated `save()` of a model without a `RowSnapshot`
/// field: a full-row write, and what to use for concurrent edits instead.
pub(super) const SAVE_DOC: &str = "Inserts this model when `id` is `0`, otherwise updates its row.

An update writes every persisted column of this handle, so the last writer wins: \
a change made to the row since this handle was loaded is overwritten, even in a \
column this code never touched. Declare a `#[sqlx(skip)]` field of type \
`rullst_orm::RowSnapshot` to make `save()` write only the columns this handle \
changed. To change some fields of a row that other requests \
may edit at the same time, use `update_partial()`, which merges the selected fields \
into the freshly read row (locked with `FOR UPDATE` on PostgreSQL and MySQL/MariaDB). \
For a counter, run one atomic statement such as \
`Orm::raw(\"UPDATE users SET xp = xp + ? WHERE id = ?\")` \
(`$1`, `$2` on PostgreSQL); raw SQL skips the model's tenant scope, policy, hooks, \
audit and cache invalidation.

An insert also sends every field, so database column defaults do not apply, \
except to fields marked `#[orm(db_default)]`, which inserts and updates never write.";

/// Rustdoc of the generated `save()` of a model with a `RowSnapshot` field.
pub(super) const TRACKED_SAVE_DOC: &str = "Inserts this model when `id` is `0`, otherwise updates \
the columns this handle changed.

An update writes only the columns whose value differs from the one this handle was \
loaded or last saved with (see `dirty_columns()`), so concurrent edits of other \
columns persist; the columns it did not write are then refreshed from the stored row. \
With nothing changed it writes nothing, and runs nothing after the `saving` \
observers; without a `before_save` hook, policy or observers it does not even open \
a transaction. A handle that was not loaded from the database, or after \
`mark_all_dirty()`, writes every column. Two writers of the same column still race \
and the last writer wins: for a read-modify-write such as a counter, use \
`update_partial()`, which merges into the freshly read row (locked with \
`FOR UPDATE` on PostgreSQL and MySQL/MariaDB), or one atomic statement such as \
`Orm::raw(\"UPDATE users SET xp = xp + ? WHERE id = ?\")` \
(`$1`, `$2` on PostgreSQL); raw SQL skips the model's tenant scope, policy, hooks, \
audit and cache invalidation. After a rollback of a caller-owned transaction, \
reload the record or call `mark_all_dirty()`.

An insert also sends every field, so database column defaults do not apply, \
except to fields marked `#[orm(db_default)]`, which inserts and updates never write.";
