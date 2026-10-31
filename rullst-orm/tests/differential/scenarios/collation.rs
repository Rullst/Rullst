//! Text comparison and ordering under each database's default collation:
//! case, accents, trailing spaces and case-variant unique values.

use rullst_orm::Orm;

use super::{create_table, in_order, sorted};
use crate::transcript::Scenario;

#[derive(Debug, Clone, Orm)]
#[orm(table = "diff_people")]
struct Person {
    id: i32,
    /// 1: names that only differ in case or accents; 2: names that differ
    /// at the first collation level, so every collation orders them fully.
    grp: i32,
    name: String,
    email: String,
}

impl Person {
    fn name(&self) -> &str {
        &self.name
    }
}

const PEOPLE: [(i32, &str); 11] = [
    (1, "alice"),
    (1, "Alice"),
    (1, "ALBERT"),
    (1, "Émile"),
    (1, "émile"),
    (2, "bob"),
    (2, "Carl"),
    (2, "Zoe"),
    (2, "dora"),
    (2, "Édith"),
    (2, "_under"),
];

async fn add(grp: i32, name: &str, email: &str) -> Result<(), rullst_orm::Error> {
    let mut person = Person {
        id: 0,
        grp,
        name: name.to_string(),
        email: email.to_string(),
    };
    person.save().await
}

pub async fn run(s: &mut Scenario<'_>) {
    let created = create_table(s, "diff_people", |table| {
        table.id();
        table.integer("grp").not_null();
        table.string("name").not_null().col_type = "VARCHAR(191)".to_string();
        table.string("email").not_null().col_type = "VARCHAR(191)".to_string();
        table.unique(&["email"]).name("diff_people_email_key");
    })
    .await;
    if !created {
        return;
    }
    let mut seeded = Ok(());
    for (index, (grp, name)) in PEOPLE.into_iter().enumerate() {
        seeded = seeded.and(add(grp, name, &format!("person{index}@example.test")).await);
    }
    s.outcome("seed", seeded);

    let query = Person::query;
    let matches = [
        ("eq_same_case", query().where_eq("name", "alice")),
        ("eq_other_case", query().where_eq("name", "ALICE")),
        ("eq_without_accent", query().where_eq("name", "emile")),
        ("eq_trailing_space", query().where_eq("name", "bob ")),
        ("like_ascii_prefix", query().where_like("name", "al%")),
        ("like_accented_prefix", query().where_like("name", "é%")),
        (
            "in_other_case",
            query().where_in("name", vec!["BOB", "carl"]),
        ),
    ];
    for (step, builder) in matches {
        s.outcome(step, sorted(builder.get().await, Person::name));
    }
    s.outcome(
        "order_by_name",
        in_order(
            query().where_eq("grp", 2).order_by("name").get().await,
            Person::name,
        ),
    );
    s.outcome(
        "order_by_name_desc",
        in_order(
            query().where_eq("grp", 2).order_by_desc("name").get().await,
            Person::name,
        ),
    );

    s.outcome(
        "unique_case_variant",
        add(3, "variant", "PERSON0@example.test").await,
    );
    s.outcome(
        "unique_exact_duplicate",
        add(3, "duplicate", "person1@example.test").await,
    );
}
