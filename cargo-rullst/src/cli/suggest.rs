//! "Did you mean" for unknown commands: separator-insensitive equality, edit
//! distance (with transpositions), a matching `group:name` segment and
//! prefixes, ranked deterministically.

use clap::Command;

/// Longest input compared; longer typos are not worth an O(n·m) table.
const INPUT_LIMIT: usize = 64;
/// At most this many suggestions are shown.
pub(crate) const SUGGESTION_LIMIT: usize = 5;

/// Visible subcommand names and their aliases, without `help`.
pub(crate) fn command_names(command: &Command) -> Vec<String> {
    let mut names = Vec::new();
    for subcommand in command.get_subcommands() {
        if subcommand.is_hide_set() || subcommand.get_name() == "help" {
            continue;
        }
        names.push(subcommand.get_name().to_string());
        names.extend(subcommand.get_all_aliases().map(str::to_string));
    }
    names.sort();
    names.dedup();
    names
}

fn normalized(value: &str) -> String {
    value
        .chars()
        .filter(char::is_ascii_alphanumeric)
        .map(|character| character.to_ascii_lowercase())
        .collect()
}

/// Optimal string alignment distance: insertions, deletions, substitutions
/// and adjacent transpositions each cost one.
pub(crate) fn edit_distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let width = right.len() + 1;
    let mut table = vec![0usize; (left.len() + 1) * width];
    for (row, cell) in table.iter_mut().step_by(width).enumerate() {
        *cell = row;
    }
    for (column, cell) in table.iter_mut().take(width).enumerate() {
        *cell = column;
    }
    for row in 1..=left.len() {
        for column in 1..=right.len() {
            let cost = usize::from(left[row - 1] != right[column - 1]);
            let mut best = (table[(row - 1) * width + column] + 1)
                .min(table[row * width + column - 1] + 1)
                .min(table[(row - 1) * width + column - 1] + cost);
            if row > 1
                && column > 1
                && left[row - 1] == right[column - 2]
                && left[row - 2] == right[column - 1]
            {
                best = best.min(table[(row - 2) * width + column - 2] + 1);
            }
            table[row * width + column] = best;
        }
    }
    table[left.len() * width + right.len()]
}

/// How close a candidate is; smaller sorts first.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Closeness {
    /// Equal once case and separators are ignored (`make-model`).
    Same,
    /// Within the typo budget, or equal to one `:` segment (`migrate`).
    Typo(usize),
    /// The input starts the name or one of its segments (`make`, `migr`).
    Prefix,
}

impl Closeness {
    /// Typo distance; `None` for a prefix match.
    fn distance(self) -> Option<usize> {
        match self {
            Self::Same => Some(0),
            Self::Typo(distance) => Some(distance),
            Self::Prefix => None,
        }
    }
}

fn closeness(input: &str, name: &str) -> Option<Closeness> {
    let input_lower = input.to_ascii_lowercase();
    let name_lower = name.to_ascii_lowercase();
    let (input_plain, name_plain) = (normalized(input), normalized(name));
    if input_plain.is_empty() {
        return None;
    }
    if input_plain == name_plain {
        return Some(Closeness::Same);
    }
    let segments: Vec<&str> = name_lower.split(':').collect();
    if segments.len() > 1 && segments.iter().any(|segment| *segment == input_lower) {
        return Some(Closeness::Typo(0));
    }
    let budget = (input_plain.len() / 3).clamp(1, 3);
    let distance =
        edit_distance(&input_lower, &name_lower).min(edit_distance(&input_plain, &name_plain));
    if distance <= budget {
        return Some(Closeness::Typo(distance));
    }
    let starts = name_lower.starts_with(&input_lower)
        || segments
            .iter()
            .any(|segment| segment.starts_with(&input_lower));
    (input_plain.len() >= 2 && starts).then_some(Closeness::Prefix)
}

/// Up to [`SUGGESTION_LIMIT`] names closest to `input`, best first; ties go
/// to the shorter name, then alphabetical order.
pub(crate) fn suggestions<'a>(input: &str, names: &'a [String]) -> Vec<&'a str> {
    let input: String = input.trim().chars().take(INPUT_LIMIT).collect();
    let mut ranked: Vec<(Closeness, &str)> = names
        .iter()
        .filter_map(|name| closeness(&input, name).map(|closeness| (closeness, name.as_str())))
        .collect();
    ranked.sort_by(|(left, left_name), (right, right_name)| {
        left.cmp(right)
            .then(left_name.len().cmp(&right_name.len()))
            .then(left_name.cmp(right_name))
    });
    // A close match hides prefix matches and much worse typos, which would
    // only add noise; prefix-only results list the whole group.
    let limit = ranked
        .first()
        .and_then(|(closeness, _)| closeness.distance());
    ranked
        .into_iter()
        .filter(|(closeness, _)| match (limit, closeness.distance()) {
            (Some(limit), Some(distance)) => distance <= limit + 1,
            (Some(_), None) => false,
            (None, _) => true,
        })
        .take(SUGGESTION_LIMIT)
        .map(|(_, name)| name)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names() -> Vec<String> {
        [
            "auth",
            "audit",
            "db:migrate",
            "db:rollback",
            "db:seed",
            "db:status",
            "dev",
            "doctor",
            "deploy",
            "make:controller",
            "make:migration",
            "make:migration:auto",
            "make:model",
            "make:middleware",
            "make:mail",
            "new",
            "update",
            "upgrade",
        ]
        .map(str::to_string)
        .to_vec()
    }

    #[test]
    fn edit_distance_counts_transpositions_once() {
        assert_eq!(edit_distance("", ""), 0);
        assert_eq!(edit_distance("abc", ""), 3);
        assert_eq!(edit_distance("upgrdae", "upgrade"), 1);
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("ab", "ba"), 1);
    }

    #[test]
    fn typos_and_separators_suggest_the_intended_command() {
        let names = names();
        for (input, expected) in [
            ("make:modek", "make:model"),
            ("make-model", "make:model"),
            ("MAKE_MODEL", "make:model"),
            ("doctr", "doctor"),
            ("db:migrte", "db:migrate"),
            ("migrate", "db:migrate"),
            ("upgrdae", "upgrade"),
        ] {
            assert_eq!(
                suggestions(input, &names).first(),
                Some(&expected),
                "{input}"
            );
        }
    }

    #[test]
    fn prefixes_list_the_whole_group_in_a_stable_order() {
        let names = names();
        assert_eq!(
            suggestions("db", &names),
            ["db:seed", "db:status", "db:migrate", "db:rollback"]
        );
        let make = suggestions("make", &names);
        assert_eq!(make.len(), SUGGESTION_LIMIT);
        assert_eq!(make.first(), Some(&"make:mail"));
        assert_eq!(
            suggestions("migr", &names)[..2],
            ["db:migrate", "make:migration"]
        );
    }

    #[test]
    fn close_matches_hide_prefix_noise_and_nonsense_has_no_suggestion() {
        let names = names();
        assert_eq!(suggestions("dve", &names), ["dev"]);
        assert_eq!(suggestions("make:modek", &names), ["make:model"]);
        assert!(suggestions("zzzzzz", &names).is_empty());
        assert!(suggestions("", &names).is_empty());
        assert!(suggestions("--", &names).is_empty());
        let long = "x".repeat(10_000);
        assert!(suggestions(&long, &names).is_empty());
    }

    #[test]
    fn hidden_commands_and_help_are_not_suggested() {
        let command = Command::new("rullst")
            .subcommand(Command::new("make:model").visible_alias("model"))
            .subcommand(Command::new("secret").hide(true))
            .subcommand(Command::new("help"));
        assert_eq!(command_names(&command), ["make:model", "model"]);
    }
}
