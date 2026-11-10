//! The part of a Content-Security-Policy that decides whether inline code
//! runs: the directive that governs it and its nonce, hash and
//! `'unsafe-inline'` sources.

/// Inline code a policy can refuse.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Inline {
    /// A `<script>` element without `src`.
    ScriptElement,
    /// A `<style>` element.
    StyleElement,
    /// An `on*` event-handler attribute.
    ScriptAttribute,
    /// A `style` attribute.
    StyleAttribute,
}

impl Inline {
    /// The directives that govern it, most specific first (CSP Level 3).
    fn directives(self) -> [&'static str; 3] {
        match self {
            Self::ScriptElement => ["script-src-elem", "script-src", "default-src"],
            Self::StyleElement => ["style-src-elem", "style-src", "default-src"],
            Self::ScriptAttribute => ["script-src-attr", "script-src", "default-src"],
            Self::StyleAttribute => ["style-src-attr", "style-src", "default-src"],
        }
    }

    /// Nonces never apply to attributes; hashes do only with `'unsafe-hashes'`.
    fn is_attribute(self) -> bool {
        matches!(self, Self::ScriptAttribute | Self::StyleAttribute)
    }

    /// `'strict-dynamic'` concerns scripts only.
    fn is_script(self) -> bool {
        matches!(self, Self::ScriptElement | Self::ScriptAttribute)
    }
}

/// Every policy of a `Content-Security-Policy` value. Inline code runs only
/// when each of them allows it.
#[derive(Debug)]
pub(super) struct Policies(Vec<Policy>);

/// One policy: each directive's name (lowercase) and sources.
#[derive(Debug)]
struct Policy(Vec<(String, Vec<String>)>);

impl Policies {
    /// The policies of a header value; a comma separates two policies, as
    /// when the header is sent twice.
    pub(super) fn parse(header: &str) -> Self {
        Self(header.split(',').map(Policy::parse).collect())
    }

    /// Whether every policy lets `inline` run, given the element's `nonce`
    /// attribute.
    pub(super) fn allows(&self, inline: Inline, nonce: Option<&str>) -> bool {
        self.0.iter().all(|policy| policy.allows(inline, nonce))
    }
}

impl Policy {
    fn parse(policy: &str) -> Self {
        let mut directives: Vec<(String, Vec<String>)> = Vec::new();
        for directive in policy.split(';') {
            let mut tokens = directive.split_ascii_whitespace();
            let Some(name) = tokens.next() else {
                continue;
            };
            let name = name.to_ascii_lowercase();
            // Browsers ignore a repeated directive: the first one counts.
            if directives.iter().all(|(existing, _)| *existing != name) {
                directives.push((name, tokens.map(str::to_owned).collect()));
            }
        }
        Self(directives)
    }

    /// The sources of the most specific directive present for `inline`.
    fn sources(&self, inline: Inline) -> Option<&[String]> {
        inline.directives().into_iter().find_map(|wanted| {
            self.0
                .iter()
                .find(|(name, _)| name == wanted)
                .map(|(_, sources)| sources.as_slice())
        })
    }

    fn allows(&self, inline: Inline, nonce: Option<&str>) -> bool {
        let Some(sources) = self.sources(inline) else {
            // No directive governs it, so nothing restricts it.
            return true;
        };
        let keyword = |wanted: &str| {
            sources
                .iter()
                .any(|source| source.eq_ignore_ascii_case(wanted))
        };
        let prefixed = |prefix: &str| {
            sources.iter().any(|source| {
                source
                    .get(..prefix.len())
                    .is_some_and(|start| start.eq_ignore_ascii_case(prefix))
            })
        };
        let hashed = ["'sha256-", "'sha384-", "'sha512-"]
            .into_iter()
            .any(prefixed);
        if inline.is_attribute() {
            // A hash may allow this exact attribute; it is not computed here.
            if hashed && keyword("'unsafe-hashes'") {
                return true;
            }
        } else {
            let expected = nonce
                .filter(|nonce| !nonce.is_empty())
                .map(|nonce| format!("'nonce-{nonce}'"));
            if expected.is_some_and(|expected| sources.contains(&expected)) {
                return true;
            }
            // A hash may allow this exact content; it is not computed here.
            if hashed {
                return true;
            }
        }
        // Browsers ignore 'unsafe-inline' next to a nonce or a hash, and for
        // scripts next to 'strict-dynamic'.
        keyword("'unsafe-inline'")
            && !prefixed("'nonce-")
            && !hashed
            && !(inline.is_script() && keyword("'strict-dynamic'"))
    }
}

#[cfg(test)]
#[path = "csp_policy_tests.rs"]
mod tests;
