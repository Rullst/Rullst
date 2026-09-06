// src/builder/chunking.rs — Offset and stable primary-key chunk traversal.

use crate::parser::ParsedModel;
use proc_macro2::TokenStream;
use quote::quote;

#[cfg_attr(test, mutants::skip)]
pub fn generate_chunk_methods(parsed: &ParsedModel) -> Vec<TokenStream> {
    let name = &parsed.name;
    let table_name = &parsed.table_name;

    vec![quote! {
        /// Processes rows in offset-based pages.
        ///
        /// Prefer [`Self::chunk_by_id`] when the handler mutates the same table;
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
            let mut offset = 0usize;
            let mut builder = self.clone();
            builder.limit = Some(size);
            loop {
                builder.offset = Some(offset);
                let results = builder.get().await?;
                let count = results.len();
                if count == 0 { break; }
                handler(results).await;
                if count < size { break; }
                offset = offset.checked_add(size).ok_or_else(|| {
                    rullst_orm::Error::Validation(
                        "chunk() offset exceeds the supported range".to_string()
                    )
                })?;
            }
            Ok(())
        }

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
            let mut offset = 0usize;
            let mut builder = self.clone();
            builder.limit = Some(size);
            loop {
                builder.offset = Some(offset);
                let results = builder.get_with_tx(tx).await?;
                let count = results.len();
                if count == 0 { break; }
                handler(results).await;
                if count < size { break; }
                offset = offset.checked_add(size).ok_or_else(|| {
                    rullst_orm::Error::Validation(
                        "chunk_with_tx() offset exceeds the supported range".to_string()
                    )
                })?;
            }
            Ok(())
        }

        /// Processes rows in ascending primary-key order without offset drift.
        ///
        /// The generated SQL uses `id > last_seen_id`, so deleting already
        /// processed rows cannot make later records move behind an offset. The
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
            let mut cursor: Option<i32> = None;
            loop {
                let mut builder = self.clone().order_by("id");
                builder.freeze_scope();
                builder.limit = Some(size);
                builder.offset = None;
                if let Some(last_seen_id) = cursor {
                    builder = builder.where_gt("id", last_seen_id);
                }
                let results = builder.get().await?;
                let count = results.len();
                if count == 0 { break; }
                cursor = results.last().map(|model| model.id);
                handler(results).await?;
                if count < size { break; }
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
            let mut cursor: Option<i32> = None;
            loop {
                let mut builder = self.clone().order_by("id");
                builder.freeze_scope();
                builder.limit = Some(size);
                builder.offset = None;
                if let Some(last_seen_id) = cursor {
                    builder = builder.where_gt("id", last_seen_id);
                }
                let results = builder.get_with_tx(tx).await?;
                let count = results.len();
                if count == 0 { break; }
                cursor = results.last().map(|model| model.id);
                handler(results).await?;
                if count < size { break; }
            }
            Ok(())
        }
    }]
}
