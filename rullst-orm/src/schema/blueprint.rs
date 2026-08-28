use super::column::{Column, ColumnDefault};
use super::validation::validate_identifier;
use crate::Error;

pub struct Blueprint {
    pub columns: Vec<Column>,
}

impl Default for Blueprint {
    fn default() -> Self {
        Self::new()
    }
}

impl Blueprint {
    pub fn new() -> Self {
        Self { columns: vec![] }
    }

    pub fn id(&mut self) -> &mut Column {
        self.columns.push(Column {
            name: "id".to_string(),
            col_type: "INTEGER".to_string(),
            is_nullable: false,
            is_primary_key: true,
            is_auto_increment: true,
            default_value: None,
        });
        let len = self.columns.len();
        &mut self.columns[len - 1]
    }

    fn add_column(&mut self, name: &str, col_type: &str) -> &mut Column {
        let col = Column::new(name, col_type);
        self.columns.push(col);
        let len = self.columns.len();
        &mut self.columns[len - 1]
    }

    pub fn string(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "TEXT")
    }

    pub fn integer(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "INTEGER")
    }

    pub fn big_integer(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "BIGINT")
    }

    pub fn float(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "REAL")
    }

    pub fn boolean(&mut self, name: &str) -> &mut Column {
        self.add_column(name, "INTEGER")
    }

    pub fn vector(&mut self, name: &str, dimensions: usize) -> &mut Column {
        let col_type = format!("VECTOR({})", dimensions);
        self.add_column(name, &col_type)
    }

    pub fn enum_col(&mut self, name: &str, variants: Vec<&str>) -> &mut Column {
        // Enforce enum values using a CHECK constraint for safe cross-DB compatibility
        let check_clause = variants
            .iter()
            .map(|v| format!("'{}'", v.replace('\'', "''")))
            .collect::<Vec<_>>()
            .join(", ");
        let col_type = format!("TEXT CHECK({} IN ({}))", name, check_clause);
        self.add_column(name, &col_type)
    }

    pub fn timestamps(&mut self) {
        let mut created = Column::new("created_at", "TEXT");
        created.default(ColumnDefault::CurrentTimestamp);
        self.columns.push(created);

        let mut updated = Column::new("updated_at", "TEXT");
        updated.default(ColumnDefault::CurrentTimestamp);
        self.columns.push(updated);
    }

    pub fn soft_deletes(&mut self) {
        let col = Column::new("deleted_at", "TEXT");
        self.columns.push(col);
        let len = self.columns.len();
        self.columns[len - 1].nullable();
    }

    #[cfg_attr(test, mutants::skip)]
    pub fn build(&self) -> Result<String, Error> {
        self.build_for_driver(crate::Orm::driver()?)
    }

    pub(crate) fn build_for_driver(&self, driver: &str) -> Result<String, Error> {
        let mut defs = vec![];
        for col in &self.columns {
            // Defensive re-validation: column names must always be safe
            // identifiers regardless of how the Column was constructed.
            validate_identifier(&col.name)?;

            let mut col_type_str = col.col_type.clone();
            if driver == "postgres" && col.is_auto_increment {
                if col.col_type == "INTEGER" || col.col_type == "INT" {
                    col_type_str = "SERIAL".to_string();
                } else if col.col_type == "BIGINT" {
                    col_type_str = "BIGSERIAL".to_string();
                }
            }

            let mut def = format!("{} {}", col.name, col_type_str);
            if col.is_primary_key {
                def.push_str(" PRIMARY KEY");
            }
            if col.is_auto_increment {
                if driver == "sqlite" {
                    def.push_str(" AUTOINCREMENT");
                } else if driver == "mysql" {
                    def.push_str(" AUTO_INCREMENT");
                }
            }
            if !col.is_nullable && !col.is_primary_key {
                def.push_str(" NOT NULL");
            }
            if let Some(default) = &col.default_value {
                use std::fmt::Write;
                let _ = write!(def, " DEFAULT {}", default.to_sql());
            }
            defs.push(def);
        }
        Ok(defs.join(",\n    "))
    }
}
