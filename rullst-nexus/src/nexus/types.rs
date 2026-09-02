use std::sync::Arc;

/// Kind of field for UI forms and schema definitions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldKind {
    /// Single-line text input.
    Text,
    /// Multi-line textarea input.
    Textarea,
    /// Email input with validation.
    Email,
    /// URL input with validation.
    Url,
    /// Integer or float number input.
    Number,
    /// A boolean checkbox.
    Boolean,
    /// Date picker (YYYY-MM-DD).
    Date,
    /// Date + time picker (YYYY-MM-DDTHH:MM).
    DateTime,
    /// A password field that hides its value.
    Password,
    /// A JSON object displayed as textarea.
    Json,
    /// A foreign key pointing to another table.
    ForeignKey {
        /// Target table name (e.g. "categories").
        table: &'static str,
        /// Column of target table to use as option label (e.g. "name").
        label_col: &'static str,
    },
    /// A dropdown menu for Enum values.
    Enum { options: Vec<&'static str> },
}

/// Describes a single field/column in a model's schema for the Nexus Panel.
#[derive(Debug, Clone)]
pub struct FieldMeta {
    /// Database/struct column name (e.g. "created_at").
    pub name: &'static str,
    /// Human-readable label shown in the UI (e.g. "Created At").
    pub label: &'static str,
    /// Semantic type that determines which input widget to render.
    pub kind: FieldKind,
    /// If true, hides this field from list/table views (still visible on edit forms).
    pub hidden: bool,
    /// If true, the field is displayed but cannot be modified via the edit form.
    pub readonly: bool,
}

impl FieldMeta {
    pub fn new(name: &'static str, label: &'static str, kind: FieldKind) -> Self {
        Self {
            name,
            label,
            kind,
            hidden: false,
            readonly: false,
        }
    }

    pub fn hidden(mut self) -> Self {
        self.hidden = true;
        self
    }

    pub fn readonly(mut self) -> Self {
        self.readonly = true;
        self
    }
}

/// The core reflection trait that unlocks Nexus Panel integration for any model.
pub trait NexusModel: Send + Sync + 'static {
    /// The database table name (e.g. "users").
    fn nexus_table() -> &'static str;
    /// A human-readable plural label for the collection (e.g. "Users").
    fn nexus_label() -> &'static str;
    /// An icon symbol for the sidebar nav link (e.g. "&#128104;&#8205;&#128187;").
    fn nexus_icon() -> &'static str {
        "&#128196;"
    }
    /// Primary key column name (default: "id").
    fn nexus_pk() -> &'static str {
        "id"
    }
    /// Optional text column used to scope every Nexus read and mutation to the
    /// authenticated [`rullst_core::security::TenantContext`].
    ///
    /// Models without tenant-owned rows should keep the default. A scoped
    /// model fails closed when the request has no trusted tenant context.
    fn nexus_tenant_column() -> Option<&'static str> {
        None
    }
    /// The field schema array describing column kinds and metadata.
    fn nexus_fields() -> Vec<FieldMeta>;
}

/// Internal representation of a registered model used by the Nexus Panel engine.
#[derive(Clone)]
pub struct RegistryEntry {
    pub table: &'static str,
    pub label: &'static str,
    pub icon: &'static str,
    pub pk: &'static str,
    pub tenant_column: Option<&'static str>,
    pub fields: Vec<FieldMeta>,
}

/// Persistence policy for successful Nexus data mutations.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum NexusAuditPolicy {
    /// Do not write Nexus-specific mutation records.
    #[default]
    Disabled,
    /// Commit each mutation only when its minimized audit row is written in
    /// the same database transaction.
    Required,
}

/// Shared state passed into all Nexus route handlers.
#[derive(Clone)]
pub struct NexusState {
    pub registry: Arc<Vec<RegistryEntry>>,
    pub brand: Arc<String>,
    pub audit_policy: NexusAuditPolicy,
}
