#![cfg(not(any(feature = "strict-postgres", feature = "strict-mysql")))]

use proptest::prelude::*;
use rullst_orm::Orm;
use std::sync::Once;

#[derive(Debug, Clone, rullst_orm::FromRow, Orm)]
pub struct PropUser {
    pub id: i32,
    pub name: String,
    pub email: String,
}

static INIT: Once = Once::new();

fn init_orm() {
    INIT.call_once(|| {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let _ = rullst_orm::Orm::init("sqlite::memory:").await;
        });
    });
}

proptest! {
    #[test]
    fn test_builder_does_not_panic(
        name in "\\PC*",
        email in "\\PC*",
        limit in 0..1000usize,
        offset in 0..1000usize
    ) {
        init_orm();

        let builder = PropUser::query()
            .where_like("name", name.as_str())
            .where_eq("email", email.as_str())
            .order_by_desc(name.as_str())
            .limit(limit)
            .offset(offset);

        let sql = builder.to_sql();

        prop_assert!(!sql.is_empty());
    }
}
