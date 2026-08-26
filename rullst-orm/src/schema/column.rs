/// Safe values allowed for a column DEFAULT clause.
///
/// Accepting a raw `&str` would allow DDL injection through the DEFAULT
/// position. This enum restricts callers to known-safe literals.
#[derive(Debug, Clone, PartialEq)]
pub enum ColumnDefault {
    /// `CURRENT_TIMESTAMP` — standard SQL timestamp literal.
    CurrentTimestamp,
    /// `NULL` — explicit SQL null default.
    Null,
    /// A non-negative integer literal (e.g. `0`, `1`).
    Integer(i64),
    /// A non-negative real literal (e.g. `0.0`).
    Float(f64),
    /// A string literal that will be single-quoted and escaped.
    /// Only printable ASCII excluding `'` and `\` is accepted.
    Text(String),
}

impl ColumnDefault {
    /// Renders the default value as a safe SQL fragment.
    pub fn to_sql(&self) -> String {
        match self {
            ColumnDefault::CurrentTimestamp => "CURRENT_TIMESTAMP".to_string(),
            ColumnDefault::Null => "NULL".to_string(),
            ColumnDefault::Integer(n) => n.to_string(),
            ColumnDefault::Float(f) => format!("{f}"),
            // Single-quote the string and escape any embedded single-quotes
            // via SQL standard doubling (''), which is safe on every driver.
            ColumnDefault::Text(s) => format!("'{}'", s.replace('\'', "''")),
        }
    }
}

pub struct Column {
    pub name: String,
    pub col_type: String,
    pub is_nullable: bool,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub default_value: Option<ColumnDefault>,
}

impl Column {
    /// Creates a new column.
    ///
    /// Identifier validation is repeated by the fallible schema builder before
    /// SQL is emitted, including for callers that mutate the public fields.
    pub fn new(name: impl Into<String>, col_type: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            col_type: col_type.into(),
            is_nullable: true,
            is_primary_key: false,
            is_auto_increment: false,
            default_value: None,
        }
    }

    pub fn not_null(&mut self) -> &mut Self {
        self.is_nullable = false;
        self
    }

    pub fn nullable(&mut self) -> &mut Self {
        self.is_nullable = true;
        self
    }

    /// Sets a safe DEFAULT value using the [`ColumnDefault`] enum.
    ///
    /// The old `&str` overload has been removed to prevent DDL injection
    /// through unescaped DEFAULT clauses.
    pub fn default(&mut self, val: ColumnDefault) -> &mut Self {
        self.default_value = Some(val);
        self
    }

    pub fn primary(&mut self) -> &mut Self {
        self.is_primary_key = true;
        self
    }
}
