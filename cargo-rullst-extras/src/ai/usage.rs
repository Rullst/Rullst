//! Token counts per answer and per session, shown only when the provider
//! reports them, with a cost estimate only at prices the user configured.

use super::credentials::Prices;
use rullst_ai::TokenUsage;

/// `1234567` as `1,234,567`.
pub(super) fn thousands(value: u64) -> String {
    let digits = value.to_string();
    let mut output = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            output.push(',');
        }
        output.push(digit);
    }
    output
}

fn estimate_text(cost: f64) -> String {
    format!("≈ {cost:.4} est. at your prices")
}

/// Running totals for one session. Each answer adds exactly what its line
/// showed: input and output counts, or a total when the provider reported
/// no split.
#[derive(Debug, Default)]
pub(super) struct UsageTotals {
    answers: usize,
    reported: usize,
    input: u64,
    output: u64,
    /// Whether any answer reported an input or output count.
    split: bool,
    /// Totals of answers that reported no input/output split.
    unsplit: Option<u64>,
    cost: Option<f64>,
}

impl UsageTotals {
    /// Records one answer and describes its usage for the status line.
    pub(super) fn record(&mut self, usage: Option<TokenUsage>, prices: Option<Prices>) -> String {
        self.answers += 1;
        let Some(usage) = usage else {
            return "usage not reported".to_string();
        };
        let (input, output) = (usage.input_tokens(), usage.output_tokens());
        let mut parts = Vec::new();
        match (input, output, usage.total_tokens()) {
            (Some(input), Some(output), _) => {
                self.add_split(input, output);
                parts.push(format!(
                    "{} in · {} out tokens",
                    thousands(input),
                    thousands(output)
                ));
            }
            (_, _, Some(total)) => {
                self.unsplit = Some(self.unsplit.unwrap_or(0).saturating_add(total));
                parts.push(format!("{} tokens", thousands(total)));
            }
            (Some(input), None, None) => {
                self.add_split(input, 0);
                parts.push(format!("{} in tokens", thousands(input)));
            }
            (None, Some(output), None) => {
                self.add_split(0, output);
                parts.push(format!("{} out tokens", thousands(output)));
            }
            (None, None, None) => {}
        }
        if !parts.is_empty() {
            self.reported += 1;
        }
        if let Some(cached) = usage.cached_input_tokens().filter(|cached| *cached > 0) {
            parts.push(format!("{} cached", thousands(cached)));
        }
        if let (Some(prices), Some(input), Some(output)) = (prices, input, output) {
            let cost = prices.estimate(input, output);
            self.cost = Some(self.cost.unwrap_or(0.0) + cost);
            parts.push(estimate_text(cost));
        }
        if parts.is_empty() {
            "usage not reported".to_string()
        } else {
            parts.join(" · ")
        }
    }

    fn add_split(&mut self, input: u64, output: u64) {
        self.split = true;
        self.input = self.input.saturating_add(input);
        self.output = self.output.saturating_add(output);
    }

    /// The end-of-session line, when any answer reported usage.
    pub(super) fn summary(&self) -> Option<String> {
        if self.reported == 0 {
            return None;
        }
        let split = format!(
            "{} in · {} out tokens",
            thousands(self.input),
            thousands(self.output)
        );
        let counts = match (self.split, self.unsplit) {
            (true, Some(total)) => {
                format!("{split} + {} tokens without a split", thousands(total))
            }
            (false, Some(total)) => format!("{} tokens", thousands(total)),
            _ => split,
        };
        let mut line = format!("Session usage: {counts} over {} answer(s)", self.answers);
        if self.reported < self.answers {
            line.push_str(&format!(" (reported for {})", self.reported));
        }
        if let Some(cost) = self.cost {
            line.push_str(&format!(" · {}", estimate_text(cost)));
        }
        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_use_thousands_separators() {
        assert_eq!(thousands(0), "0");
        assert_eq!(thousands(999), "999");
        assert_eq!(thousands(1_000), "1,000");
        assert_eq!(thousands(1_234_567), "1,234,567");
    }

    #[test]
    fn answers_and_sessions_show_only_reported_counts() {
        let mut totals = UsageTotals::default();
        assert_eq!(totals.record(None, None), "usage not reported");
        assert_eq!(totals.summary(), None, "nothing is invented");

        let usage = TokenUsage::new(Some(1_200), Some(300)).with_cached_input_tokens(200);
        let line = totals.record(Some(usage), None);
        assert_eq!(line, "1,200 in · 300 out tokens · 200 cached");
        assert!(!line.contains('≈'), "no estimate without configured prices");

        let prices = Prices::new(2.0, 8.0);
        let line = totals.record(
            Some(TokenUsage::new(Some(1_000_000), Some(500_000))),
            prices,
        );
        assert!(line.ends_with("≈ 6.0000 est. at your prices"), "{line}");
        let summary = totals.summary().unwrap();
        assert!(summary.starts_with(
            "Session usage: 1,001,200 in · 500,300 out tokens over 3 answer(s) (reported for 2)"
        ));
        assert!(summary.ends_with("≈ 6.0000 est. at your prices"));

        let mut totals = UsageTotals::default();
        assert_eq!(
            totals.record(
                Some(TokenUsage::new(None, None).with_total_tokens(42)),
                prices
            ),
            "42 tokens"
        );
        // The session line keeps what the answer line showed.
        assert_eq!(
            totals.summary().as_deref(),
            Some("Session usage: 42 tokens over 1 answer(s)")
        );
        assert_eq!(
            totals.record(Some(TokenUsage::new(Some(10), Some(5))), None),
            "10 in · 5 out tokens"
        );
        assert_eq!(
            totals.record(Some(TokenUsage::new(Some(7), None)), None),
            "7 in tokens"
        );
        assert_eq!(
            totals.record(Some(TokenUsage::new(None, None)), None),
            "usage not reported"
        );
        assert_eq!(
            totals.summary().as_deref(),
            Some(
                "Session usage: 17 in · 5 out tokens + 42 tokens without a split over 4 answer(s) (reported for 3)"
            )
        );
    }
}
