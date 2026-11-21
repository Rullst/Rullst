//! The read-only graph grammar accepts the documented read forms and refuses
//! everything else, whichever SurrealDB release would parse it.

use super::*;

const ACCEPTED: &str = "accepted";

/// `ACCEPTED`, or the reason `GraphQuery::read_only` refuses `query`.
fn verdict(query: &str) -> &'static str {
    match GraphQuery::read_only(query, 10) {
        Ok(_) => ACCEPTED,
        Err(PolyglotError::InvalidIdentifier {
            kind: "graph query",
            reason,
        }) => reason,
        Err(other) => panic!("unexpected error for {query:?}: {other:?}"),
    }
}

fn assert_all(queries: &[&str], expected: &str) {
    for query in queries {
        assert_eq!(verdict(query), expected, "{query}");
    }
}

#[test]
fn documented_read_queries_are_accepted() {
    assert_all(
        &[
            "MATCH (n:person) RETURN n",
            "MATCH (person:person)-[knows:knows]->(friend:person) RETURN friend",
            "MATCH (event:events) RETURN event.sequence AS sequence ORDER BY sequence",
            // Nodes and edges with only a variable, only a label, or neither.
            "MATCH (n) RETURN n",
            "MATCH (:person)<-[:knows]-(friend) RETURN friend",
            "MATCH (a)-[]-(b) RETURN a",
            "MATCH (a)-[e]->(b)<-[f:likes]->(c) RETURN e, f",
            "MATCH ()-[:knows]->() RETURN x",
            // Several paths.
            "MATCH (a:person), (b:city)-[lives:lives_in]-(a) RETURN a, b",
            // Tests joined with AND, OR and NOT.
            "MATCH (n:person) WHERE n.age >= 18 AND n.name <> 'Ada' OR NOT n.city = \"Rio\" RETURN n.name",
            "MATCH (n:person) WHERE n.age < 1 OR n.age <= 2 OR n.age > 3 OR n.age = 4 RETURN n",
            "MATCH (n:person) WHERE NOT NOT n.active RETURN n",
            "MATCH (n:person) WHERE n.manager IS NULL RETURN n",
            "MATCH (n:person) WHERE n.email IS NOT NULL AND n.active RETURN n",
            "MATCH (n:person), (m:person) WHERE n.score > 4.5 AND n.team.id = m.team.id RETURN n",
            "MATCH (n:person), (m:person) WHERE n = m RETURN n",
            "MATCH (n:person) WHERE n.verified = true RETURN n",
            // RETURN items, aliases and every ORDER BY form.
            "MATCH (n:person) RETURN n AS person",
            "MATCH (n:person) RETURN n.name AS name, n.age AS age ORDER BY age DESC, name ASC, n.id",
            "MATCH (n:person) RETURN n ORDER BY n.address.city",
            "MATCH (n:person) RETURN n, n.age ORDER BY n DESC",
            "MATCH (n:person) RETURN n ORDER BY n.age ASC, n",
        ],
        ACCEPTED,
    );
}

#[test]
fn keywords_are_case_insensitive_and_whitespace_is_ascii() {
    assert_all(
        &[
            "match (n:Person) where n.Age > 1 and n.x is not null return n order by n.Age desc",
            "MaTcH (n) WhErE n.a = 1 Or n.b = 2 ReTuRn n As m OrDeR bY m AsC",
            "MATCH\n\t(n:person)\r\n  RETURN\tn",
            "\u{a0}MATCH (n) RETURN n\u{2003}",
        ],
        ACCEPTED,
    );
    assert_all(
        &[
            "MATCH\u{a0}(n) RETURN n",
            "MATCH (n) RETURN\u{2028}n",
            "MATCH (\u{f1}) RETURN \u{f1}",
        ],
        UNKNOWN_CHARACTER,
    );
}

#[test]
fn keywords_inside_strings_and_names_are_not_clauses() {
    assert_all(
        &[
            "MATCH (n:person) WHERE n.bio = 'DELETE (n) SET n.x = 1 -- // /* # $p {} [x] fn::x() LIMIT 5' RETURN n",
            "MATCH (n:person) WHERE n.bio = \"CREATE (m) CALL x() YIELD y\" RETURN n",
            "MATCH (n:person) WHERE n.quote = \"it's\" OR n.quote = 'say \"hi\"' RETURN n",
            "MATCH (n:person) WHERE n.name = 'A' OR n.name = 'Grace' RETURN n",
            "MATCH (n:person) WHERE n.name = 'Zo\u{eb} \u{2713}' RETURN n",
            "MATCH (n:person) WHERE n.name='Ada'AND n.age=1 RETURN n",
            "MATCH (set_log:remove_log)-[deleted_by:inserted_by]->(_creator) WHERE set_log.limit_value = 0 RETURN set_log.delete_at AS order_id ORDER BY order_id",
            "MATCH (n:create)-[e:merge]->(m:update) RETURN n.set, n.delete, n.insert, n.remove",
            "MATCH (n:person) WHERE n.price = 10.25 RETURN n",
        ],
        ACCEPTED,
    );
}

/// The clauses SurrealDB 3.2 runs (`INSERT`, `SET`, `REMOVE`, `DELETE`) and
/// the ones a later release could add, wherever a clause could start.
#[test]
fn write_and_unknown_clauses_are_refused_in_every_position() {
    let keywords = [
        "INSERT", "SET", "REMOVE", "DELETE", "DETACH", "CREATE", "CALL", "DROP", "USE", "LET",
        "FILTER", "FOR", "YIELD", "UNION", "NEXT", "MERGE", "UPDATE", "OPTIONAL", "WITH",
        "DISTINCT", "OFFSET", "SKIP",
    ];
    for keyword in keywords {
        let mixed = keyword
            .char_indices()
            .map(|(index, character)| {
                if index % 2 == 0 {
                    character.to_ascii_lowercase()
                } else {
                    character
                }
            })
            .collect::<String>();
        for spelling in [keyword.to_owned(), keyword.to_ascii_lowercase(), mixed] {
            assert_eq!(
                verdict(&format!("{spelling} (n:person) RETURN n")),
                NOT_MATCH,
                "{spelling}"
            );
            for query in [
                format!("MATCH (n:person) {spelling} (m:person)"),
                format!("MATCH (n:person) {spelling} n.age = 1 RETURN n"),
                format!("MATCH (n:person)-[e:knows]->(m) {spelling} n"),
                format!("MATCH (n:person) WHERE n.age > 1 {spelling} n RETURN n"),
                format!("MATCH (n:person) WHERE n.ok {spelling} n RETURN n"),
                format!("MATCH (n:person) WHERE n IS NULL {spelling} n RETURN n"),
                format!("MATCH (n:person) RETURN n {spelling} n"),
                format!("MATCH (n:person) RETURN n AS m {spelling} (x)"),
                format!("MATCH (n:person) RETURN n ORDER BY n {spelling} n"),
                format!("MATCH (n:person) RETURN n ORDER BY n DESC {spelling} n"),
            ] {
                assert_eq!(verdict(&query), OUTSIDE_GRAMMAR, "{query}");
            }
        }
    }
}

#[test]
fn the_write_forms_surrealdb_implements_and_later_ones_are_refused() {
    assert_all(
        &[
            "MATCH (n:person) INSERT (m:person)",
            "MATCH (n:person) SET n.age = 1 RETURN n",
            "MATCH (n:person) REMOVE n.age RETURN n",
            "MATCH (n:person) DELETE n",
            "MATCH (n:person) DETACH DELETE n",
            "MATCH (n:person) CREATE (m:person)",
            "MATCH (n:person) MERGE (m:person)",
            "MATCH (n:person) UPDATE n SET n.age = 1",
            "MATCH (n:person) CALL drop_all() YIELD x RETURN x",
            "MATCH (n:person) LET x = 1 RETURN x",
            "MATCH (n:person) FILTER n.age > 1 RETURN n",
            "MATCH (n:person) FOR x IN n.list RETURN x",
            "MATCH (n:person) RETURN n UNION MATCH (m) RETURN m",
            "MATCH (n:person) RETURN n NEXT MATCH (m) DELETE m",
            "MATCH (n:person) RETURN fn::delete(n)",
            "MATCH (n:person) WHERE fn::x(n) = 1 RETURN n",
            "MATCH (n:person) RETURN count(n)",
            "MATCH (n:person) MATCH (m) RETURN n",
        ],
        OUTSIDE_GRAMMAR,
    );
    assert_all(
        &[
            "DELETE person",
            "CREATE (n:person)",
            "create (n)",
            "DROP GRAPH g",
            "USE g MATCH (n) RETURN n",
            "LET x = 1",
            "CALL procedure()",
            "OPTIONAL MATCH (n) RETURN n",
            "RETURN 1",
            "(n) RETURN n",
            "'MATCH' (n) RETURN n",
            "LIMIT 1",
            "WHERE n.x = 1",
        ],
        NOT_MATCH,
    );
}

#[test]
fn the_adapter_alone_writes_limit() {
    assert_all(
        &[
            "MATCH (n:person) RETURN n LIMIT 5",
            "MATCH (n:person) RETURN n ORDER BY n LIMIT 5",
            "MATCH (n:person) RETURN n ORDER BY n DESC limit 5",
            "MATCH (n:person) RETURN limit",
            "MATCH (limit) RETURN n",
            "MATCH (n:limit) RETURN n",
            "MATCH (n:person) RETURN n.limit",
            "MATCH (n:person) RETURN n AS Limit",
        ],
        OUTSIDE_GRAMMAR,
    );
    let query = GraphQuery::read_only(" MATCH (n:person) WHERE n.age > 1 RETURN n\n", 7).unwrap();
    assert_eq!(
        query.bounded_query(),
        "MATCH (n:person) WHERE n.age > 1 RETURN n LIMIT 7"
    );
}

#[test]
fn incomplete_or_misplaced_parts_are_refused() {
    assert_all(
        &[
            "MATCH",
            "MATCH (n)",
            "MATCH (n:person)-[e:knows]->(m)",
            "MATCH (n) RETURN",
            "MATCH (n) RETURN n,",
            "MATCH (n) RETURN n AS",
            "MATCH (n) RETURN n ORDER",
            "MATCH (n) RETURN n ORDER BY",
            "MATCH (n) RETURN n ORDER BY n,",
            "MATCH (n) RETURN n.",
            "MATCH (n) RETURN n.1",
            "MATCH (n) RETURN 1",
            "MATCH (n) RETURN 'n'",
            "MATCH (n) RETURN n AS m AS o",
            "MATCH (n) RETURN n ORDER BY n ASC DESC",
            "MATCH (n) RETURN n ORDER n",
            "MATCH (n) WHERE RETURN n",
            "MATCH (n) WHERE n = RETURN n",
            "MATCH (n) WHERE n IS RETURN n",
            "MATCH (n) WHERE n IS NOT RETURN n",
            "MATCH (n) WHERE n IS NOT 1 RETURN n",
            "MATCH (n) WHERE n.x = 1 AND RETURN n",
            "MATCH (n) WHERE 1 = n.x RETURN n",
            "MATCH (n) WHERE n.x = 1 = 2 RETURN n",
            "MATCH (n) WHERE n.x = m.y = 2 RETURN n",
            "MATCH (n) WHERE n.x = 'a' 'b' RETURN n",
            "MATCH (n) WHERE n.x = 'it''s' RETURN n",
            "MATCH (n) WHERE n.x = NULL RETURN n",
            "MATCH (n) WHERE n.x RETURN n ORDER BY n WHERE n.y",
            "MATCH (n:) RETURN n",
            "MATCH (n:a:b) RETURN n",
            "MATCH (n m) RETURN n",
            "MATCH (n)(m) RETURN n",
            "MATCH (n), RETURN n",
            "MATCH (n)-[e]->RETURN n",
            "MATCH (n)-[e) RETURN n",
            "MATCH (n]-> RETURN n",
            "MATCH (n]- RETURN n",
            "MATCH (n)-[e:knows:likes]->(m) RETURN n",
            "MATCH -[e]->(n) RETURN n",
            "MATCH (a)-[e]- >(b) RETURN a",
            "MATCH (n) , (m) WHERE n.x = 1 , RETURN n",
        ],
        OUTSIDE_GRAMMAR,
    );
}

#[test]
fn comments_parameters_and_other_syntax_are_refused() {
    assert_all(
        &[
            "MATCH (n) RETURN n // comment",
            "MATCH (n) RETURN n -- comment",
            "MATCH (n) RETURN n /* comment */",
            "MATCH (n) RETURN n # comment",
            "MATCH (n) WHERE n.x = $value RETURN n",
            "MATCH (n {name: 'Ada'}) RETURN n",
            "MATCH (n) RETURN n[0]",
            "MATCH (n) RETURN n.*",
            "MATCH (n) RETURN `n`",
            "MATCH (n:a|b) RETURN n",
            "MATCH (n:a&b) RETURN n",
            "MATCH (n) WHERE n.x = -1 RETURN n",
            "MATCH (n) WHERE n.x + 1 = 2 RETURN n",
            "MATCH (n) WHERE n.x != 1 RETURN n",
            "MATCH (n)->(m) RETURN n",
            "MATCH (n)<-(m) RETURN n",
            "MATCH (a)-->(b) RETURN a",
            "MATCH (a)- [e]->(b) RETURN a",
            "MATCH (a)-[e] ->(b) RETURN a",
            "MATCH (a)-[e]=>(b) RETURN a",
        ],
        UNKNOWN_CHARACTER,
    );
    assert_eq!(
        verdict("MATCH (n) RETURN n; DELETE n"),
        "multiple statements and semicolons are not accepted"
    );
    assert_eq!(
        verdict("MATCH (n) WHERE n.x = 'a;b' RETURN n"),
        "multiple statements and semicolons are not accepted"
    );
}

#[test]
fn strings_end_at_their_own_quote_and_take_no_escapes() {
    assert_all(
        &[
            "MATCH (n) WHERE n.x = 'unclosed RETURN n",
            "MATCH (n) WHERE n.x = 'mixed\" RETURN n",
            "MATCH (n) WHERE n.x = \"mixed' RETURN n",
            "MATCH (n) WHERE n.x = 'a\\'b' RETURN n",
            "MATCH (n) WHERE n.x = \"a\\\" RETURN n //\"",
            "MATCH (n) WHERE n.x = 'a\\\\' RETURN n",
            "MATCH (n) WHERE n.x = 'a\\nb' RETURN n",
        ],
        BAD_STRING,
    );
}

#[test]
fn numbers_are_unsigned_integers_or_decimals() {
    assert_all(
        &[
            "MATCH (n) WHERE n.x = 0 RETURN n",
            "MATCH (n) WHERE n.x = 1234567890 RETURN n",
            "MATCH (n) WHERE n.x = 3.14 RETURN n",
        ],
        ACCEPTED,
    );
    assert_all(
        &[
            "MATCH (n) WHERE n.x = 1and n.y = 2 RETURN n",
            "MATCH (n) WHERE n.x = 1e5 RETURN n",
            "MATCH (n) WHERE n.x = 10ms RETURN n",
            "MATCH (n) WHERE n.x = 0x10 RETURN n",
            "MATCH (n) WHERE n.x = 1_000 RETURN n",
            "MATCH (n) WHERE n.x = 1.5f RETURN n",
            "MATCH (n) WHERE n.x = 1. RETURN n",
            "MATCH (n) WHERE n.x = 1.x RETURN n",
            "MATCH (n) WHERE n.x = 1..2 RETURN n",
            "MATCH (n) WHERE n.x = 1.2.3 RETURN n",
        ],
        BAD_NUMBER,
    );
    assert_eq!(
        verdict("MATCH (n) WHERE n.x = 3.14)"),
        OUTSIDE_GRAMMAR,
        "a number ends at punctuation"
    );
}
