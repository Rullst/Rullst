//! The minute, hour, day-of-month and month fields of a cron expression.
//!
//! A field is a comma-separated list of elements. An element is `*`, a number,
//! a range `a-b` (`a` at most `b`) or, in the month field, a name (`jan`,
//! `january`, any case) or a range of names (`jan-mar`). Each of them except a
//! single name may take a step: `*/15`, `1-30/2`, `jan-jun/2`, and `5/15`, which
//! runs from 5 to the field's maximum. A step is 1 to the field's maximum. The
//! day-of-month field also accepts `?` as `*`. This is the grammar the `cron`
//! crate applied to these fields until 13.0.0-alpha.5.

/// The values a field allows: bit `n` is set when value `n` is allowed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Values(u64);

impl Values {
    /// The set whose bits are `bits`.
    pub(super) const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    /// Whether `value` is allowed.
    pub(super) fn contains(self, value: u32) -> bool {
        self.0.checked_shr(value).is_some_and(|bits| bits & 1 == 1)
    }

    /// The smallest allowed value that is at least `value`.
    pub(super) fn first_from(self, value: u32) -> Option<u32> {
        let bits = self.0.checked_shr(value).filter(|bits| *bits != 0)?;
        Some(value + bits.trailing_zeros())
    }
}

/// One field: its name in messages, its bounds and what else it accepts.
pub(super) struct Unit {
    name: &'static str,
    min: u32,
    max: u32,
    /// Accepts `?` as `*`.
    any: bool,
    names: &'static [(&'static str, u32)],
}

pub(super) const MINUTE: Unit = Unit {
    name: "minute",
    min: 0,
    max: 59,
    any: false,
    names: &[],
};

pub(super) const HOUR: Unit = Unit {
    name: "hour",
    min: 0,
    max: 23,
    any: false,
    names: &[],
};

pub(super) const DAY_OF_MONTH: Unit = Unit {
    name: "day-of-month",
    min: 1,
    max: 31,
    any: true,
    names: &[],
};

pub(super) const MONTH: Unit = Unit {
    name: "month",
    min: 1,
    max: 12,
    any: false,
    names: &[
        ("jan", 1),
        ("january", 1),
        ("feb", 2),
        ("february", 2),
        ("mar", 3),
        ("march", 3),
        ("apr", 4),
        ("april", 4),
        ("may", 5),
        ("jun", 6),
        ("june", 6),
        ("jul", 7),
        ("july", 7),
        ("aug", 8),
        ("august", 8),
        ("sep", 9),
        ("september", 9),
        ("oct", 10),
        ("october", 10),
        ("nov", 11),
        ("november", 11),
        ("dec", 12),
        ("december", 12),
    ],
};

impl Unit {
    /// Parses one whole field.
    pub(super) fn parse(&self, field: &str) -> Result<Values, String> {
        let mut bits = 0;
        for element in field.split(',') {
            bits |= self
                .element(element)
                .map_err(|reason| format!("{} `{field}`: {reason}", self.name))?;
        }
        Ok(Values(bits))
    }

    /// The values of one list element.
    fn element(&self, element: &str) -> Result<u64, String> {
        let (base, step) = match element.split_once('/') {
            Some((base, step)) => (base, Some(self.step(step)?)),
            None => (element, None),
        };
        let (start, end) = if base == "*" || (self.any && base == "?") {
            (self.min, self.max)
        } else if let Some((start, end)) = base.split_once('-') {
            self.range(start, end)?
        } else if is_number(base) {
            let start = self.number(base)?;
            // `5/15` runs from 5 to the maximum.
            (start, if step.is_some() { self.max } else { start })
        } else {
            let value = self.name_value(base)?;
            if step.is_some() {
                return Err(format!(
                    "`{element}` steps a single name; use a range such as `jan-dec/2`"
                ));
            }
            (value, value)
        };
        // Distinct powers of two, so the sum is their union.
        Ok((start..=end)
            .step_by(step.unwrap_or(1))
            .map(|value| 1_u64 << value)
            .sum())
    }

    /// `start-end`, two numbers or two names, in order.
    fn range(&self, start: &str, end: &str) -> Result<(u32, u32), String> {
        let (first, last) = match (is_number(start), is_number(end)) {
            (true, true) => (self.number(start)?, self.number(end)?),
            (false, false) => (self.name_value(start)?, self.name_value(end)?),
            _ => return Err(format!("`{start}-{end}` mixes a number and a name")),
        };
        if first > last {
            return Err(format!("range `{start}-{end}` is reversed"));
        }
        Ok((first, last))
    }

    fn number(&self, text: &str) -> Result<u32, String> {
        match text.parse::<u32>() {
            Ok(value) if (self.min..=self.max).contains(&value) => Ok(value),
            _ => Err(format!("`{text}` must be {}-{}", self.min, self.max)),
        }
    }

    fn name_value(&self, text: &str) -> Result<u32, String> {
        let lowercase = text.to_ascii_lowercase();
        self.names
            .iter()
            .find(|(name, _)| *name == lowercase)
            .map(|(_, value)| *value)
            .ok_or_else(|| format!("`{text}` is not a number, range or name"))
    }

    fn step(&self, text: &str) -> Result<usize, String> {
        match text.parse::<usize>() {
            Ok(step) if is_number(text) && (1..=self.max as usize).contains(&step) => Ok(step),
            _ => Err(format!("step `{text}` must be 1-{}", self.max)),
        }
    }
}

/// One or more ASCII digits and nothing else (no sign).
fn is_number(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|byte| byte.is_ascii_digit())
}
