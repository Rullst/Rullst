//! Generated terminal queries.

fn generated(input: syn::DeriveInput) -> String {
    let parsed = crate::parser::parse(&input).expect("test model should parse");
    let methods = super::generate_execution_methods(
        &parsed,
        &quote::format_ident!("UserQueryBuilder"),
        &proc_macro2::TokenStream::new(),
    );
    methods
        .iter()
        .map(ToString::to_string)
        .collect::<String>()
        .split_whitespace()
        .collect()
}

#[test]
fn count_and_paginate_run_the_bounded_count_query() {
    let code = generated(syn::parse_quote! { struct User { id: i32, name: String } });
    let count_query = |row: &str| {
        format!(
            "letquery_str=self.to_count_sql();letcount_bindings=self.__rullst_count_query_bindings();\
             ifrullst_orm::schema::is_query_log_enabled(){{println!(\"[SQLDebug]{{:?}}|Bindings:[{{}}parameter(s)redactedforsecurity]\",query_str,count_bindings.len());}}\
             let{row}:(i64,)=rullst_orm::dispatch_executor!(read_pool,|pool|{{\
             letmutquery=rullst_orm::_sqlx::query_as::<_,(i64,)>(rullst_orm::_sqlx::AssertSqlSafe(query_str.as_str()));\
             forbindingin&count_bindings{{query=rullst_orm::value::BindRullstValue::bind_rullst_value(query,binding);}}\
             lettimeout=rullst_orm::schema::get_query_timeout();\
             ifletSome(t)=timeout{{rullst_orm::_tokio::time::timeout(t,query.fetch_one(pool)).await\
             .map_err(|_|rullst_orm::Error::DatabaseError(\"Queryexecutiontimedout\".to_string()))??}}\
             else{{query.fetch_one(pool).await?}}}});"
        )
    };
    // `paginate` totals the rows, `count` returns them.
    assert!(code.contains(&count_query("total_row")), "{code}");
    assert!(code.contains("lettotal=total_row.0;"));
    assert!(code.contains(&count_query("row")), "{code}");
    assert!(code.contains("Ok(row.0)"));
}
