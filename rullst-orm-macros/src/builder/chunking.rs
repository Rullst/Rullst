// src/builder/chunking.rs — Offset and stable primary-key chunk traversal.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_chunk_methods(parsed: &ParsedModel) -> Vec<TokenStream> {
    let name = &parsed.name;
    let table_name = &parsed.table_name;
    // Qualified, so a join cannot make the default order or the keyset
    // order ambiguous.
    let default_order = format!("{table_name}.id ASC");

    vec![quote! {
        /// Processes rows in offset-based pages.
        ///
        /// Without an `order_by`, pages follow the primary key, because SQL
        /// gives consecutive offset queries no stable order otherwise. An
        /// explicit `limit()` caps the rows handed to the handler in total and
        /// an explicit `offset()` is where the traversal starts. Prefer
        /// [`Self::chunk_by_id`] when the handler mutates the same table;
        /// offset pagination can skip rows after deletes or reordering.
        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self, handler),
            fields(
                orm.model = stringify!(#name),
                orm.table = #table_name,
                orm.operation = "chunk"
            )
        )]
        pub async fn chunk<F, Fut>(&self, size: usize, mut handler: F) -> Result<(), rullst_orm::Error>
        where
            F: FnMut(Vec<#name>) -> Fut + Send,
            Fut: std::future::Future<Output = ()> + Send,
        {
            if size == 0 {
                return Err(rullst_orm::Error::Validation(
                    "chunk() requires a size greater than zero".to_string()
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
                let results = builder.get().await?;
                let count = results.len();
                if count == 0 { break; }
                handler(results).await;
                if count < page { break; }
                remaining = remaining.map(|left| left.saturating_sub(count));
                offset = offset.checked_add(count).ok_or_else(|| {
                    rullst_orm::Error::Validation(
                        "chunk() offset exceeds the supported range".to_string()
                    )
                })?;
            }
            Ok(())
        }

        /// Transaction-aware counterpart of [`Self::chunk`], with the same
        /// default primary-key order.
        pub async fn chunk_with_tx<F, Fut>(&self, size: usize, tx: &mut rullst_orm::db::Transaction<'static>, mut handler: F) -> Result<(), rullst_orm::Error>
        where
            F: FnMut(Vec<#name>) -> Fut + Send,
            Fut: std::future::Future<Output = ()> + Send,
        {
            if size == 0 {
                return Err(rullst_orm::Error::Validation(
                    "chunk_with_tx() requires a size greater than zero".to_string()
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
                let results = builder.get_with_tx(tx).await?;
                let count = results.len();
                if count == 0 { break; }
                handler(results).await;
                if count < page { break; }
                remaining = remaining.map(|left| left.saturating_sub(count));
                offset = offset.checked_add(count).ok_or_else(|| {
                    rullst_orm::Error::Validation(
                        "chunk_with_tx() offset exceeds the supported range".to_string()
                    )
                })?;
            }
            Ok(())
        }

        /// Processes rows in ascending primary-key order without offset drift.
        ///
        /// The generated SQL uses `<table>.id > last_seen_id` (qualified, so a
        /// join cannot make it ambiguous), so deleting already processed rows
        /// cannot make later records move behind an offset. An explicit
        /// `limit()` caps the rows handed to the handler in total, and an
        /// explicit `offset()` skips that many rows before the first page. The
        /// handler is fallible and stops traversal on its first error.
        #[rullst_orm::_tracing::instrument(
            name = "rullst.orm.query",
            target = "rullst_orm",
            skip(self, handler),
            fields(
                orm.model = stringify!(#name),
                orm.table = #table_name,
                orm.operation = "chunk_by_id"
            )
        )]
        pub async fn chunk_by_id<F, Fut>(&self, size: usize, mut handler: F) -> Result<(), rullst_orm::Error>
        where
            F: FnMut(Vec<#name>) -> Fut + Send,
            Fut: std::future::Future<Output = Result<(), rullst_orm::Error>> + Send,
        {
            if size == 0 {
                return Err(rullst_orm::Error::Validation(
                    "chunk_by_id() requires a size greater than zero".to_string()
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
                let results = builder.get().await?;
                let count = results.len();
                if count == 0 { break; }
                cursor = results.last().map(|model| model.id);
                handler(results).await?;
                if count < page { break; }
                remaining = remaining.map(|left| left.saturating_sub(count));
            }
            Ok(())
        }

        /// Transaction-aware counterpart of [`Self::chunk_by_id`].
        pub async fn chunk_by_id_with_tx<F, Fut>(&self, size: usize, tx: &mut rullst_orm::db::Transaction<'static>, mut handler: F) -> Result<(), rullst_orm::Error>
        where
            F: FnMut(Vec<#name>) -> Fut + Send,
            Fut: std::future::Future<Output = Result<(), rullst_orm::Error>> + Send,
        {
            if size == 0 {
                return Err(rullst_orm::Error::Validation(
                    "chunk_by_id_with_tx() requires a size greater than zero".to_string()
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
                let results = builder.get_with_tx(tx).await?;
                let count = results.len();
                if count == 0 { break; }
                cursor = results.last().map(|model| model.id);
                handler(results).await?;
                if count < page { break; }
                remaining = remaining.map(|left| left.saturating_sub(count));
            }
            Ok(())
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
