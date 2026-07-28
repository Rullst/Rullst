#[cfg(test)]
mod tests {
    use super::*;
    use proc_macro2::Span;

    #[test]
    fn test_split_top_level() {
        // Test basic splitting
        let parts = split_top_level("a, b, c");
        assert_eq!(parts, vec!["a", " b", " c"]);

        // Test parentheses protection
        let parts = split_top_level("a(1, 2), b, c(3, 4)");
        assert_eq!(parts, vec!["a(1, 2)", " b", " c(3, 4)"]);

        // Test unmatched parentheses (depth check behavior)
        let parts = split_top_level("a), b");
        assert_eq!(parts, vec!["a)", " b"]);
    }

    #[test]
    fn test_strip_outer_call() {
        // Test valid call
        assert_eq!(
            strip_outer_call("soft_delete(field = \"a\")", "soft_delete"),
            Some("field = \"a\"".to_string())
        );

        // Test with whitespace
        assert_eq!(
            strip_outer_call("  soft_delete  (  field = \"a\"  )  ", "soft_delete"),
            Some("field = \"a\"".to_string())
        );

        // Test not a call
        assert_eq!(strip_outer_call("soft_delete", "soft_delete"), None);
        assert_eq!(strip_outer_call("soft_delete(a", "soft_delete"), None);
        assert_eq!(strip_outer_call("soft_delete a)", "soft_delete"), None);
    }

    #[test]
    fn test_validate_relation_attribute() {
        // Test valid relations
        assert!(validate_relation_attribute("has_many", "User", Span::call_site()).is_ok());
        assert!(validate_relation_attribute("has_one", "Profile", Span::call_site()).is_ok());
        assert!(validate_relation_attribute("belongs_to", "Team", Span::call_site()).is_ok());
        assert!(validate_relation_attribute("belongs_to_many", "Role", Span::call_site()).is_ok());
        assert!(validate_relation_attribute("morph_many", "Comment", Span::call_site()).is_ok());
        assert!(validate_relation_attribute("morph_one", "Image", Span::call_site()).is_ok());

        // Test empty model
        assert!(validate_relation_attribute("has_many", "", Span::call_site()).is_err());
        assert!(validate_relation_attribute("belongs_to", "", Span::call_site()).is_err());

        // Test empty modifier values
        assert!(validate_relation_attribute("foreign_key", "", Span::call_site()).is_err());
        assert!(validate_relation_attribute("related_key", "", Span::call_site()).is_err());
        assert!(validate_relation_attribute("pivot_table", "", Span::call_site()).is_err());
        assert!(validate_relation_attribute("local_key", "", Span::call_site()).is_err());
        assert!(validate_relation_attribute("name", "", Span::call_site()).is_err());
    }

    #[test]
    fn test_parse_model_attributes() {
        // To kill mutants on match arms ("morph_many", "belongs_to", etc.) 
        // we could test `parse` directly by passing a Mock `DeriveInput`
        use syn::parse_quote;

        let input: DeriveInput = parse_quote! {
            #[orm(table_name = "test_table", global_scope = "active = 1", tenant_column = "tenant_id", policy = "TestPolicy")]
            #[orm(auditable, searchable)]
            #[orm(before_save = "before_s", after_save = "after_s")]
            #[orm(before_delete = "before_d", after_delete = "after_d")]
            #[orm(after_fetch = "after_f")]
            #[orm(soft_delete(column = "del_at", value = "1", delval = "0"))]
            struct TestModel {
                id: i32,
                #[orm(has_many = "M1")] m1: Vec<M1>,
                #[orm(has_one = "M2")] m2: M2,
                #[orm(belongs_to = "M3")] m3: M3,
                #[orm(belongs_to_many = "M4", pivot_table = "piv_m4")] m4: Vec<M4>,
                #[orm(morph_many = "M5", name = "m5_able", local_key = "custom_id")] m5: Vec<M5>,
                #[orm(morph_one = "M6", name = "m6_able", foreign_key = "f_id", related_key = "r_id")] m6: M6,
                #[orm(skip)] skipped: i32,
                #[orm(hidden)] hidden: String,
                #[orm(masked)] token: String,
            }
        };

        let parsed = parse(&input).unwrap();
        
        assert_eq!(parsed.table_name, "test_table");
        assert_eq!(parsed.global_scope, "active = 1");
        assert_eq!(parsed.tenant_column, "tenant_id");
        assert_eq!(parsed.policy, "TestPolicy");
        assert!(parsed.auditable);
        assert!(parsed.searchable);
        assert_eq!(parsed.before_save, "before_s");
        assert_eq!(parsed.after_save, "after_s");
        assert_eq!(parsed.before_delete, "before_d");
        assert_eq!(parsed.after_delete, "after_d");
        assert_eq!(parsed.after_fetch, "after_f");

        let sd = parsed.soft_delete.unwrap();
        assert_eq!(sd.column, "del_at");
        assert_eq!(sd.value, "1");
        assert_eq!(sd.delval, "0");

        let r = &parsed.relations;
        assert_eq!(r.len(), 6);
        assert_eq!(r[0].rel_type, "has_many");
        assert_eq!(r[1].rel_type, "has_one");
        assert_eq!(r[2].rel_type, "belongs_to");
        assert_eq!(r[3].rel_type, "belongs_to_many");
        assert_eq!(r[3].pivot_table, "piv_m4");
        assert_eq!(r[4].rel_type, "morph_many");
        assert_eq!(r[4].morph_name, "m5_able");
        assert_eq!(r[4].local_key, "custom_id");
        assert_eq!(r[5].rel_type, "morph_one");
        assert_eq!(r[5].morph_name, "m6_able");
        assert_eq!(r[5].foreign_key, "f_id");
        assert_eq!(r[5].related_key, "r_id");

        assert!(parsed.skipped_fields.iter().any(|i| i == "skipped"));
        assert!(parsed.hidden_fields.iter().any(|i| i == "hidden"));
    }

    #[test]
    fn test_parse_sensitive_fields_audit() {
        use syn::parse_quote;

        // Test that password triggers error if auditable and not masked
        let input: DeriveInput = parse_quote! {
            #[orm(table = "t", auditable)]
            struct Model { id: i32, password: String }
        };
        assert!(parse(&input).is_err());

        // Test that token triggers error if auditable and not masked
        let input2: DeriveInput = parse_quote! {
            #[orm(table = "t", auditable)]
            struct Model { id: i32, token_field: String }
        };
        assert!(parse(&input2).is_err());

        // Test that secret triggers error if auditable and not masked
        let input3: DeriveInput = parse_quote! {
            #[orm(table = "t", auditable)]
            struct Model { id: i32, my_secret: String }
        };
        assert!(parse(&input3).is_err());

        // Test that api_key triggers error if auditable and not masked
        let input4: DeriveInput = parse_quote! {
            #[orm(table = "t", auditable)]
            struct Model { id: i32, api_key: String }
        };
        assert!(parse(&input4).is_err());

        // Test that if masked, it passes
        let input5: DeriveInput = parse_quote! {
            #[orm(table = "t", auditable)]
            struct Model { id: i32, #[orm(masked)] password: String }
        };
        assert!(parse(&input5).is_ok());

        // Test that if skipped, it passes
        let input6: DeriveInput = parse_quote! {
            #[orm(table = "t", auditable)]
            struct Model { id: i32, #[orm(skip)] password: String }
        };
        assert!(parse(&input6).is_ok());

        // Test that if relation, it passes
        let input7: DeriveInput = parse_quote! {
            #[orm(table = "t", auditable)]
            struct Model { id: i32, #[orm(has_one="P")] password: String }
        };
        assert!(parse(&input7).is_ok());
    }
}
