// src/builder/chunking.rs — Offset and stable primary-key chunk traversal.

use crate::parser::ParsedModel;
use crate::query_span::instrument_query;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_chunk_methods(parsed: &ParsedModel) -> Vec<TokenStream> {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    // Qualified, so a join cannot make the default order or the keyset
    // order ambiguous.
    let default_order = format!("{table_name}.id ASC");

    let chunk_traced = instrument_query(
        name,
        table_name,
        "chunk",
        &["size"],
        offset_pages("chunk", &default_order, quote! { builder.get().await? }),
    );
    let chunk_by_id_traced = instrument_query(
        name,
        table_name,
        "chunk_by_id",
        &["size"],
        keyset_pages(
            "chunk_by_id",
            &default_order,
            quote! { builder.get().await? },
        ),
    );
    let chunk_with_tx_body = offset_pages(
        "chunk_with_tx",
        &default_order,
        quote! { builder.get_with_tx(tx).await? },
    );
    let chunk_by_id_with_tx_body = keyset_pages(
        "chunk_by_id_with_tx",
        &default_order,
        quote! { builder.get_with_tx(tx).await? },
    );

    vec![quote! {
        /// Processes rows in offset-based pages.
        ///
        /// Without an `order_by`, pages follow the primary key, because SQL
        /// gives consecutive offset queries no stable order otherwise. An
        /// explicit `limit()` caps the rows handed to the handler in total and
        /// an explicit `offset()` is where the traversal starts. Prefer
        /// [`Self::chunk_by_id`] when the handler mutates the same table;
        /// offset pagination can skip rows after deletes or reordering.
        pub async fn chunk<F, Fut>(&self, size: usize, mut handler: F) -> Result<(), rullst_orm::Error>
        where
            F: FnMut(Vec<#name>) -> Fut + Send,
            Fut: std::future::Future<Output = ()> + Send,
        {
            #chunk_traced
        }

        /// Transaction-aware counterpart of [`Self::chunk`], with the same
        /// default primary-key order.
        pub async fn chunk_with_tx<F, Fut>(&self, size: usize, tx: &mut rullst_orm::db::Transaction<'static>, mut handler: F) -> Result<(), rullst_orm::Error>
        where
            F: FnMut(Vec<#name>) -> Fut + Send,
            Fut: std::future::Future<Output = ()> + Send,
        {
            #chunk_with_tx_body
        }

        /// Processes rows in ascending primary-key order without offset drift.
        ///
        /// The generated SQL uses `<table>.id > last_seen_id` (qualified, so a
        /// join cannot make it ambiguous), so deleting already processed rows
        /// cannot make later records move behind an offset. An explicit
        /// `limit()` caps the rows handed to the handler in total, and an
        /// explicit `offset()` skips that many rows before the first page. The
        /// handler is fallible and stops traversal on its first error.
        pub async fn chunk_by_id<F, Fut>(&self, size: usize, mut handler: F) -> Result<(), rullst_orm::Error>
        where
            F: FnMut(Vec<#name>) -> Fut + Send,
            Fut: std::future::Future<Output = Result<(), rullst_orm::Error>> + Send,
        {
            #chunk_by_id_traced
        }

        /// Transaction-aware counterpart of [`Self::chunk_by_id`].
        pub async fn chunk_by_id_with_tx<F, Fut>(&self, size: usize, tx: &mut rullst_orm::db::Transaction<'static>, mut handler: F) -> Result<(), rullst_orm::Error>
        where
            F: FnMut(Vec<#name>) -> Fut + Send,
            Fut: std::future::Future<Output = Result<(), rullst_orm::Error>> + Send,
        {
            #chunk_by_id_with_tx_body
        }

        /// The size of the next page of a chunked traversal, or `None` once
        /// the rows of an explicit `limit()` were handed to the handler.
        fn __rullst_chunk_page(size: usize, remaining: Option<usize>) -> Option<usize> {
            match remaining {
                Some(0) => None,
                Some(left) => Some(size.min(left)),
                None => Some(size),
            }
        }
    }]
}

/// The body of `chunk`/`chunk_with_tx`: offset pages in the builder's order,
/// the primary key by default. `fetch` reads one page of `builder`; `method`
/// names the method in validation errors.
fn offset_pages(method: &str, default_order: &str, fetch: TokenStream) -> TokenStream {
    let zero_size = format!("{method}() requires a size greater than zero");
    let offset_overflow = format!("{method}() offset exceeds the supported range");
    quote! {
        if size == 0 {
            return Err(rullst_orm::Error::Validation(
                #zero_size.to_string()
            ));
        }
        let mut remaining = self.__rullst_explicit_limit();
        let mut offset = self.offset.unwrap_or(0);
        let mut builder = self.clone();
        if builder.order_by.is_none() {
            builder.order_bindings.clear();
            builder.order_by = Some(#default_order.to_string());
        }
        while let Some(page) = Self::__rullst_chunk_page(size, remaining) {
            builder.limit = Some(page);
            builder.offset = Some(offset);
            let results = #fetch;
            let count = results.len();
            if count == 0 { break; }
            handler(results).await;
            if count < page { break; }
            remaining = remaining.map(|left| left.saturating_sub(count));
            offset = offset.checked_add(count).ok_or_else(|| {
                rullst_orm::Error::Validation(
                    #offset_overflow.to_string()
                )
            })?;
        }
        Ok(())
    }
}

/// The body of `chunk_by_id`/`chunk_by_id_with_tx`: pages in primary-key
/// order that continue after the last seen key. `fetch` reads one page of
/// `builder`; `method` names the method in validation errors.
fn keyset_pages(method: &str, default_order: &str, fetch: TokenStream) -> TokenStream {
    let zero_size = format!("{method}() requires a size greater than zero");
    quote! {
        if size == 0 {
            return Err(rullst_orm::Error::Validation(
                #zero_size.to_string()
            ));
        }
        let mut remaining = self.__rullst_explicit_limit();
        let mut cursor: Option<i32> = None;
        while let Some(page) = Self::__rullst_chunk_page(size, remaining) {
            let mut builder = self.clone();
            builder.order_bindings.clear();
            builder.order_by = Some(#default_order.to_string());
            builder.freeze_scope();
            builder.limit = Some(page);
            // An explicit offset skips rows once, before the first page;
            // later pages continue after the last seen key.
            builder.offset = if cursor.is_none() { self.offset } else { None };
            if let Some(last_seen_id) = cursor {
                builder = builder.__rullst_where_own("id", ">", last_seen_id);
            }
            let results = #fetch;
            let count = results.len();
            if count == 0 { break; }
            cursor = results.last().map(|model| model.id);
            handler(results).await?;
            if count < page { break; }
            remaining = remaining.map(|left| left.saturating_sub(count));
        }
        Ok(())
    }
}
