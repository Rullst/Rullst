//! The field-level failures of a validated payload, read through Rullst types.

use std::any::Any;
use std::collections::BTreeMap;

/// The field-level failures of a payload that failed its [`Validate`]
/// constraints: what [`ValidationError::ValidationError`] carries.
///
/// Since 13.0.0-alpha.6 the failures are read through Rullst types, so that the
/// `validator` crate behind them is not part of this type:
/// [`messages`](Self::messages) lists the messages of each field path as the
/// JSON response shows them, and [`codes`](Self::codes) the validator codes.
/// Code that needs the `validator` crate's own errors, for example their
/// parameters, downcasts [`as_any`](Self::as_any) to the
/// `validator::ValidationErrors` of the version that [`Validate`] comes from.
///
/// Field paths name nested failures as `address.zip` and list items as
/// `items[0].name`. `Debug` lists only the paths and codes: the `validator`
/// derive stores each rejected input as a `value` parameter, which can hold a
/// password or other personal data.
///
/// ```
/// use rullst_core::validation::{FieldErrors, Validate, ValidationError};
///
/// #[derive(Validate)]
/// struct Signup {
///     #[validate(email(message = "Enter an email address"))]
///     email: String,
/// }
///
/// let signup = Signup { email: "taken@example.com".into() };
/// let mut errors = FieldErrors::validate(&signup).err().unwrap_or_default();
/// // A failure only the application can see, in the same response.
/// errors.add("email", "taken", "This email address is already registered");
/// assert_eq!(errors.codes()["email"], ["taken"]);
/// let rejection = ValidationError::ValidationError { errors, is_htmx: false };
/// # let _ = rejection;
/// ```
///
/// [`Validate`]: crate::Validate
/// [`ValidationError::ValidationError`]: crate::ValidationError::ValidationError
#[derive(Clone, Default)]
pub struct FieldErrors {
    /// The failures of the payload's own constraints, as `validator` reports them.
    reported: validator::ValidationErrors,
    /// Failures added by the application: path, code and message.
    added: Vec<(String, String, String)>,
}

impl FieldErrors {
    /// No failure; add them with [`add`](Self::add).
    pub fn new() -> Self {
        Self::default()
    }

    /// Runs the [`Validate`](crate::Validate) constraints of `value`, as
    /// [`ValidatedJson`](crate::ValidatedJson) and
    /// [`ValidatedForm`](crate::ValidatedForm) do; `Err` holds its failures.
    pub fn validate<T: crate::Validate + ?Sized>(value: &T) -> Result<(), Self> {
        value.validate().map_err(|reported| Self {
            reported,
            added: Vec::new(),
        })
    }

    /// Adds a failure of the field path `field` with a validator-style `code`
    /// (for example `taken`) and the `message` the response shows for it.
    pub fn add(
        &mut self,
        field: impl Into<String>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        self.added.push((field.into(), code.into(), message.into()));
    }

    /// `true` when no field failed.
    pub fn is_empty(&self) -> bool {
        self.reported.is_empty() && self.added.is_empty()
    }

    /// The messages of each failed field path, as the JSON response and the
    /// HTMX fragment show them: a constraint without a message reads
    /// `Invalid value for field '<path>'`.
    pub fn messages(&self) -> BTreeMap<String, Vec<String>> {
        let mut messages = BTreeMap::<String, Vec<String>>::new();
        self.visit(&mut |path, _, message| {
            messages
                .entry(path.to_string())
                .or_default()
                .push(message_or_default(path, message));
        });
        messages
    }

    /// The validator codes of each failed field path (`length`, `email`, or
    /// the code of a custom validator or of [`add`](Self::add)).
    pub fn codes(&self) -> BTreeMap<String, Vec<String>> {
        let mut codes = BTreeMap::<String, Vec<String>>::new();
        self.visit(&mut |path, code, _| {
            codes
                .entry(path.to_string())
                .or_default()
                .push(code.to_string());
        });
        codes
    }

    /// The failures of the payload's constraints as the `validator` crate
    /// reported them, without those added with [`add`](Self::add):
    /// `errors.as_any().downcast_ref::<validator::ValidationErrors>()`, with
    /// the `validator` version that [`Validate`](crate::Validate) comes from.
    /// Their parameters can hold the rejected input; never log them.
    pub fn as_any(&self) -> &(dyn Any + Send + Sync) {
        &self.reported
    }

    /// Visits every failure with its field path, code and message: the
    /// payload's own, including those of `#[validate(nested)]` structs and
    /// lists, then the added ones.
    pub(crate) fn visit(&self, visit: &mut impl FnMut(&str, &str, Option<&str>)) {
        visit_reported(&self.reported, "", visit);
        for (path, code, message) in &self.added {
            visit(path.as_str(), code.as_str(), Some(message.as_str()));
        }
    }
}

impl std::fmt::Debug for FieldErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("FieldErrors").field(&self.codes()).finish()
    }
}

/// A failure's message, or the generic one when its constraint has none.
pub(crate) fn message_or_default(path: &str, message: Option<&str>) -> String {
    message.map_or_else(
        || format!("Invalid value for field '{path}'"),
        str::to_string,
    )
}

/// Visits the `validator` failures under `prefix` with their field paths.
fn visit_reported(
    errors: &validator::ValidationErrors,
    prefix: &str,
    visit: &mut impl FnMut(&str, &str, Option<&str>),
) {
    for (field, kind) in errors.errors() {
        let path = if prefix.is_empty() {
            field.to_string()
        } else {
            format!("{prefix}.{field}")
        };
        match kind {
            validator::ValidationErrorsKind::Field(field_errors) => {
                for field_error in field_errors {
                    visit(&path, &*field_error.code, field_error.message.as_deref());
                }
            }
            validator::ValidationErrorsKind::Struct(inner) => {
                visit_reported(inner, &path, visit);
            }
            validator::ValidationErrorsKind::List(items) => {
                for (index, inner) in items {
                    visit_reported(inner, &format!("{path}[{index}]"), visit);
                }
            }
        }
    }
}
