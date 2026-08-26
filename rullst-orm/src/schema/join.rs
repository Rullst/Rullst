use super::validation::{ALLOWED_OPERATORS, validate_identifier, validate_table_name};

pub struct JoinClause {
    pub table: String,
    pub conditions: Vec<String>,
    pub bindings: Vec<crate::RullstValue>,
    pub errors: Vec<crate::Error>,
}

impl JoinClause {
    pub fn new(table: impl Into<String>) -> Self {
        let table = table.into();
        let mut clause = Self {
            table,
            conditions: vec![],
            bindings: vec![],
            errors: vec![],
        };
        if let Err(error) = validate_table_name(&clause.table) {
            clause.errors.push(error);
        }
        clause
    }

    /// Adds a column-to-column JOIN condition.
    ///
    /// This prevents SQL injection — column names should always be hardcoded, never
    /// derived from user input. Returns errors internally rather than panicking.
    pub fn on(&mut self, first: &str, operator: &str, second: &str) -> &mut Self {
        if let Err(e) = validate_identifier(first) {
            self.errors.push(crate::Error::Validation(format!(
                "JoinClause::on — invalid identifier for `first`: {:?}",
                e
            )));
        }
        if let Err(e) = validate_identifier(second) {
            self.errors.push(crate::Error::Validation(format!(
                "JoinClause::on — invalid identifier for `second`: {:?}",
                e
            )));
        }
        if !ALLOWED_OPERATORS.contains(&operator) {
            self.errors.push(crate::Error::Validation(format!(
                "JoinClause::on — invalid operator '{}'. Allowed: {:?}",
                operator, ALLOWED_OPERATORS
            )));
        }
        self.conditions
            .push(format!("{} {} {}", first, operator, second));
        self
    }

    pub fn on_eq<T: Into<crate::RullstValue>>(&mut self, column: &str, value: T) -> &mut Self {
        if let Err(e) = validate_identifier(column) {
            self.errors.push(crate::Error::Validation(format!(
                "JoinClause::on_eq — invalid identifier for `column`: {:?}",
                e
            )));
        }
        self.conditions.push(format!("{} = ?", column));
        self.bindings.push(value.into());
        self
    }

    pub fn to_sql(&self) -> String {
        self.conditions.join(" AND ")
    }
}
