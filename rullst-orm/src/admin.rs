pub fn dashboard_html() -> &'static str {
    include_str!("dashboard.html")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_html() {
        let html = dashboard_html();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("Rullst ORM - Admin Panel"));
        assert!(html.contains("Database Overview"));
    }
}
