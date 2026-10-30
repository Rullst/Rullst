//! Generated chunked traversals.

fn generated() -> String {
    let input: syn::DeriveInput = syn::parse_quote! {
        #[orm(table = "users")]
        struct User { id: i32, name: String }
    };
    let parsed = crate::parser::parse(&input).expect("test model should parse");
    super::generate_chunk_methods(&parsed)
        .iter()
        .map(ToString::to_string)
        .collect::<String>()
        .split_whitespace()
        .collect()
}

/// The body of an offset traversal named `method` that fetches with `fetch`.
fn offset_body(method: &str, fetch: &str) -> String {
    format!(
        "ifsize==0{{returnErr(rullst_orm::Error::Validation(\"{method}()requiresasizegreaterthanzero\".to_string()));}}\
         letmutremaining=self.__rullst_explicit_limit();letmutoffset=self.offset.unwrap_or(0);letmutbuilder=self.clone();\
         ifbuilder.order_by.is_none(){{builder.order_bindings.clear();builder.order_by=Some(\"users.idASC\".to_string());}}\
         whileletSome(page)=Self::__rullst_chunk_page(size,remaining){{builder.limit=Some(page);builder.offset=Some(offset);\
         letresults={fetch};letcount=results.len();ifcount==0{{break;}}handler(results).await;ifcount<page{{break;}}\
         remaining=remaining.map(|left|left.saturating_sub(count));\
         offset=offset.checked_add(count).ok_or_else(||{{rullst_orm::Error::Validation(\"{method}()offsetexceedsthesupportedrange\".to_string())}})?;}}Ok(())"
    )
}

/// The body of a keyset traversal named `method` that fetches with `fetch`.
fn keyset_body(method: &str, fetch: &str) -> String {
    format!(
        "ifsize==0{{returnErr(rullst_orm::Error::Validation(\"{method}()requiresasizegreaterthanzero\".to_string()));}}\
         letmutremaining=self.__rullst_explicit_limit();letmutcursor:Option<i32>=None;\
         whileletSome(page)=Self::__rullst_chunk_page(size,remaining){{letmutbuilder=self.clone();builder.order_bindings.clear();\
         builder.order_by=Some(\"users.idASC\".to_string());builder.freeze_scope();builder.limit=Some(page);\
         builder.offset=ifcursor.is_none(){{self.offset}}else{{None}};\
         ifletSome(last_seen_id)=cursor{{builder=builder.__rullst_where_own(\"id\",\">\",last_seen_id);}}\
         letresults={fetch};letcount=results.len();ifcount==0{{break;}}cursor=results.last().map(|model|model.id);\
         handler(results).await?;ifcount<page{{break;}}remaining=remaining.map(|left|left.saturating_sub(count));}}Ok(())"
    )
}

#[test]
fn offset_chunks_page_in_primary_key_order_by_default() {
    let code = generated();
    assert!(
        code.contains(&offset_body("chunk", "builder.get().await?")),
        "{code}"
    );
    assert!(
        code.contains(&offset_body(
            "chunk_with_tx",
            "builder.get_with_tx(tx).await?"
        )),
        "{code}"
    );
}

#[test]
fn keyset_chunks_continue_after_the_last_seen_key() {
    let code = generated();
    assert!(
        code.contains(&keyset_body("chunk_by_id", "builder.get().await?")),
        "{code}"
    );
    assert!(
        code.contains(&keyset_body(
            "chunk_by_id_with_tx",
            "builder.get_with_tx(tx).await?"
        )),
        "{code}"
    );
}
