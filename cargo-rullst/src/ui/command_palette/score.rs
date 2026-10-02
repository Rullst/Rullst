//! Deterministic fuzzy scoring for the command palette. Every query token
//! must match the command name, an alias or the description; names score
//! far above descriptions, and ties go to the shorter name, then the
//! alphabetical one.

/// Longest query considered.
pub(crate) const QUERY_LIMIT: usize = 64;

fn segments(name: &str) -> impl Iterator<Item = &str> {
    name.split([':', '-', '_', ' '])
        .filter(|segment| !segment.is_empty())
}

/// Characters of `token` appear in `name` in order: a base score plus
/// bonuses for consecutive characters and segment starts, minus gaps.
fn subsequence(token: &str, name: &str) -> Option<u32> {
    let name: Vec<char> = name.chars().collect();
    let mut position = 0usize;
    let mut previous: Option<usize> = None;
    let mut bonus = 0u32;
    let mut gaps = 0u32;
    for wanted in token.chars() {
        let found = name
            .iter()
            .skip(position)
            .position(|character| *character == wanted)?
            + position;
        let segment_start = found == 0
            || name
                .get(found - 1)
                .is_some_and(|before| matches!(before, ':' | '-' | '_' | ' '));
        if segment_start {
            bonus += 20;
        }
        match previous {
            Some(previous) if found == previous + 1 => bonus += 10,
            Some(previous) => gaps += u32::try_from(found - previous - 1).unwrap_or(u32::MAX),
            None => {}
        }
        previous = Some(found);
        position = found + 1;
    }
    Some((300 + bonus).saturating_sub(gaps.min(200)))
}

/// The score of one lowercase `token` against one lowercase `name`.
fn name_score(token: &str, name: &str) -> Option<u32> {
    if token == name {
        return Some(1000);
    }
    let length_penalty = |extra: usize| u32::try_from(extra.min(99)).unwrap_or(99);
    if name.starts_with(token) {
        return Some(900 - length_penalty(name.len() - token.len()));
    }
    if let Some(index) = segments(name).position(|segment| segment.starts_with(token)) {
        return Some(800 - u32::try_from(index.min(20) * 10).unwrap_or(200));
    }
    if let Some(index) = name.find(token) {
        return Some(600 - length_penalty(index));
    }
    subsequence(token, name)
}

fn description_score(token: &str, description: &str) -> Option<u32> {
    if description
        .split(|character: char| !character.is_alphanumeric())
        .any(|word| word.starts_with(token))
    {
        Some(150)
    } else if description.contains(token) {
        Some(100)
    } else {
        None
    }
}

/// The score of `query` for a command, or `None` when it does not match.
/// An empty query matches everything with score 0.
pub(crate) fn score(query: &str, name: &str, aliases: &[String], description: &str) -> Option<u32> {
    let query: String = query
        .chars()
        .take(QUERY_LIMIT)
        .collect::<String>()
        .to_lowercase();
    let name = name.to_lowercase();
    let aliases: Vec<String> = aliases.iter().map(|alias| alias.to_lowercase()).collect();
    let description = description.to_lowercase();
    query.split_whitespace().try_fold(0u32, |total, token| {
        let best_name = std::iter::once(name.as_str())
            .chain(aliases.iter().map(String::as_str))
            .filter_map(|candidate| name_score(token, candidate))
            .max();
        let token_score = best_name.or_else(|| description_score(token, &description))?;
        Some(total.saturating_add(token_score))
    })
}

/// Indices of the entries matching `query`, best first. `entries` yields
/// (name, aliases, description); the order is total and deterministic.
pub(crate) fn rank<'a>(
    query: &str,
    entries: impl Iterator<Item = (&'a str, &'a [String], &'a str)>,
) -> Vec<usize> {
    let mut scored: Vec<(u32, &str, usize)> = entries
        .enumerate()
        .filter_map(|(index, (name, aliases, description))| {
            score(query, name, aliases, description).map(|score| (score, name, index))
        })
        .collect();
    if query.trim().is_empty() {
        return scored.into_iter().map(|(_, _, index)| index).collect();
    }
    scored.sort_by(|(left, left_name, _), (right, right_name, _)| {
        right
            .cmp(left)
            .then(left_name.len().cmp(&right_name.len()))
            .then(left_name.cmp(right_name))
    });
    scored.into_iter().map(|(_, _, index)| index).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const COMMANDS: [(&str, &str); 12] = [
        ("db:migrate", "Runs pending database migrations"),
        (
            "db:status",
            "Displays the current status of project migrations",
        ),
        ("deploy", "Deploys application to PaaS cloud providers"),
        ("deploy:doctor", "Inspect deployment configuration"),
        ("dev", "Starts the Rullst development server"),
        (
            "doctor",
            "Runs full system diagnostics and toolchain health checks",
        ),
        (
            "generate:models",
            "Connects to an existing database and generates models",
        ),
        ("make:middleware", "Creates a new Middleware"),
        ("make:migration", "Creates a new empty migration"),
        (
            "make:migration:auto",
            "Generates a migration by diffing structs",
        ),
        (
            "make:model",
            "Creates a new Model in the src/models/ folder",
        ),
        ("update check", "Show an exact CLI release"),
    ];

    fn ranked(query: &str) -> Vec<&'static str> {
        let none: &[String] = &[];
        rank(
            query,
            COMMANDS.iter().map(|(name, about)| (*name, none, *about)),
        )
        .into_iter()
        .map(|index| COMMANDS[index].0)
        .collect()
    }

    #[test]
    fn an_empty_query_keeps_the_original_order() {
        assert_eq!(ranked("").len(), COMMANDS.len());
        assert_eq!(ranked("   ")[0], "db:migrate");
    }

    #[test]
    fn exact_prefix_and_segment_matches_rank_in_that_order() {
        assert_eq!(ranked("dev"), ["dev"]);
        assert_eq!(ranked("doc"), ["doctor", "deploy:doctor"]);
        assert_eq!(
            ranked("mig")[..3],
            ["db:migrate", "make:migration", "make:migration:auto"]
        );
        assert_eq!(ranked("model")[..2], ["make:model", "generate:models"]);
        assert_eq!(ranked("check"), ["update check", "doctor"]);
    }

    #[test]
    fn fuzzy_subsequences_and_descriptions_match_and_rank_below_names() {
        assert_eq!(ranked("mkmdl")[0], "make:model");
        assert_eq!(ranked("dbm")[0], "db:migrate");
        let health = ranked("health");
        assert_eq!(health, ["doctor"]);
        assert!(ranked("qqq").is_empty());
    }

    #[test]
    fn every_token_must_match() {
        assert_eq!(
            ranked("make migration")[..2],
            ["make:migration", "make:migration:auto"]
        );
        assert!(ranked("make zzz").is_empty());
    }

    #[test]
    fn aliases_score_like_names_and_scoring_is_deterministic() {
        let aliases = vec!["make:models-from-db".to_string()];
        assert_eq!(
            score("models-from", "generate:models", &aliases, ""),
            Some(595)
        );
        assert_eq!(
            score("make:models", "generate:models", &aliases, ""),
            Some(892)
        );
        assert_eq!(score("x", "y", &[], ""), None);
        for query in ["m", "mig", "doc", "mkmdl"] {
            assert_eq!(ranked(query), ranked(query), "{query}");
        }
        let long = "m".repeat(10_000);
        assert!(score(&long, "make:model", &[], "").is_none());
    }
}
