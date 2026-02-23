/// Tenant context extracted from HTTP request headers.
/// Injected into every handler via Axum `Extension<TenantContext>`.
///
/// Mirrors the Python `core/tenants/` contextvars approach but uses
/// Axum's type-safe extension system instead.
#[derive(Debug, Clone, Default)]
pub struct TenantContext {
    /// X-Organization-Id header value
    pub org_id: String,
    /// X-Space-Id header value (used as DB namespace suffix for multi-tenancy)
    pub space_id: String,
}

impl TenantContext {
    pub fn new(org_id: impl Into<String>, space_id: impl Into<String>) -> Self {
        Self {
            org_id: org_id.into(),
            space_id: space_id.into(),
        }
    }

    /// Returns `true` if this context represents a group (space_id set)
    pub fn is_group(&self) -> bool {
        !self.space_id.is_empty()
    }

    /// Produce a SurrealDB table suffix for tenant isolation.
    /// e.g. `episodic_memory_org1_space2`
    pub fn table_suffix(&self) -> String {
        if self.space_id.is_empty() {
            self.org_id.replace('-', "_")
        } else {
            format!(
                "{}_{}",
                self.org_id.replace('-', "_"),
                self.space_id.replace('-', "_")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_context_is_empty() {
        let ctx = TenantContext::default();
        assert_eq!(ctx.org_id, "");
        assert_eq!(ctx.space_id, "");
        assert!(!ctx.is_group());
    }

    #[test]
    fn new_sets_fields() {
        let ctx = TenantContext::new("acme", "team-a");
        assert_eq!(ctx.org_id, "acme");
        assert_eq!(ctx.space_id, "team-a");
        assert!(ctx.is_group());
    }

    #[test]
    fn table_suffix_org_only() {
        let ctx = TenantContext::new("my-org", "");
        assert_eq!(ctx.table_suffix(), "my_org");
    }

    #[test]
    fn table_suffix_org_and_space() {
        let ctx = TenantContext::new("my-org", "my-space");
        assert_eq!(ctx.table_suffix(), "my_org_my_space");
    }

    #[test]
    fn is_group_false_when_no_space() {
        let ctx = TenantContext::new("org1", "");
        assert!(!ctx.is_group());
    }
}
