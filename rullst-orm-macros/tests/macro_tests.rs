use syn::{DeriveInput, parse_quote};

#[test]
fn test_basic_model() {
    let input: DeriveInput = parse_quote! {
        #[derive(Orm)]
        #[orm(table = "users")]
        pub struct User {
            pub id: i32,
            pub name: String,
            pub email: String,
        }
    };
    let _ = input;
}
