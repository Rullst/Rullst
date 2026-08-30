/// Persistence backends exposed by the polyglot boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Backend {
    /// Deterministic, in-process document backend for tests and local work.
    Mock,
    /// MongoDB document database.
    MongoDb,
    /// DuckDB in-process analytics database.
    DuckDb,
    /// Turso/libSQL edge database through its remote protocol.
    Turso,
    /// SurrealDB multi-model database through its HTTP protocol.
    SurrealDb,
}

/// Portable capabilities intentionally supported by an adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Capability {
    /// Typed document CRUD and bounded listing.
    Documents,
    /// Parameterized analytical statements and bounded result sets.
    Analytics,
    /// Explicit, bounded graph queries.
    Graph,
    /// Parameterized SQL over an edge-native remote protocol.
    EdgeSql,
    /// Typed relational model CRUD and bounded queries.
    RelationalModels,
}

/// A backend's honest capability declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendCapabilities {
    backend: Backend,
    capabilities: &'static [Capability],
}

impl BackendCapabilities {
    /// Creates an immutable capability declaration.
    pub const fn new(backend: Backend, capabilities: &'static [Capability]) -> Self {
        Self {
            backend,
            capabilities,
        }
    }

    /// Returns the backend described by this declaration.
    pub const fn backend(self) -> Backend {
        self.backend
    }

    /// Returns the explicitly supported capabilities.
    pub const fn capabilities(self) -> &'static [Capability] {
        self.capabilities
    }

    /// Tests whether a capability is declared.
    pub fn supports(self, expected: Capability) -> bool {
        self.capabilities.contains(&expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_declarations_are_explicit() {
        let declared = BackendCapabilities::new(Backend::Mock, &[Capability::Documents]);
        assert_eq!(declared.backend(), Backend::Mock);
        assert!(declared.supports(Capability::Documents));
        assert!(!declared.supports(Capability::Graph));
    }
}
