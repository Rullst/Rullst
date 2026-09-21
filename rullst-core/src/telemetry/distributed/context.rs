use super::TelemetryError;
use http::{HeaderMap, HeaderValue};
use opentelemetry::{
    Context,
    trace::{SpanContext, SpanId, TraceContextExt, TraceFlags, TraceId, TraceState},
};
use std::fmt;
use tracing_opentelemetry::OpenTelemetrySpanExt;

/// Explicit upstream trust policy. Trace context never authenticates a caller.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParentPolicy {
    /// Discards all incoming context, including attacker-supplied sampling flags.
    StartNew,
    /// The host already trusts the peer and its correlation/sampling scope.
    TrustedPeer,
}

/// Canonical nonzero W3C version-00 context without baggage or vendor state.
#[derive(Clone, PartialEq, Eq)]
pub struct TraceParent(String);

impl TraceParent {
    /// Applies an explicit ingress decision before entering a new span. Missing
    /// or ignored context starts a root instead of inheriting ambient task state.
    pub fn apply_incoming(
        span: &tracing::Span,
        headers: &HeaderMap,
        policy: ParentPolicy,
    ) -> Result<(), TelemetryError> {
        if let Some(parent) = Self::from_headers(headers, policy)? {
            parent.attach_to(span)
        } else {
            span.set_parent(Context::new())
                .map_err(|_| TelemetryError::Unavailable)
        }
    }
    /// Validates an exact lowercase 55-byte version-00 traceparent.
    pub fn try_new(value: impl Into<String>) -> Result<Self, TelemetryError> {
        let value = value.into();
        let bytes = value.as_bytes();
        if bytes.len() != 55
            || bytes.get(..3) != Some(b"00-")
            || bytes.get(35) != Some(&b'-')
            || bytes.get(52) != Some(&b'-')
            || !bytes.iter().enumerate().all(|(i, b)| {
                matches!(i, 2 | 35 | 52) || b.is_ascii_digit() || matches!(b, b'a'..=b'f')
            })
            || bytes[3..35].iter().all(|b| *b == b'0')
            || bytes[36..52].iter().all(|b| *b == b'0')
        {
            return Err(TelemetryError::InvalidContext);
        }
        Ok(Self(value))
    }

    /// Returns the explicitly propagated correlation value. Treat it as metadata,
    /// not an account identifier, secret or authorization capability.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Selects at most one HTTP value. StartNew ignores even malformed external
    /// context; TrustedPeer rejects duplicates and malformed values.
    pub fn from_headers(
        headers: &HeaderMap,
        policy: ParentPolicy,
    ) -> Result<Option<Self>, TelemetryError> {
        if policy == ParentPolicy::StartNew {
            return Ok(None);
        }
        let mut values = headers.get_all("traceparent").iter();
        let Some(value) = values.next() else {
            return Ok(None);
        };
        if values.next().is_some() || value.as_bytes().len() != 55 {
            return Err(TelemetryError::InvalidContext);
        }
        Self::try_new(value.to_str().map_err(|_| TelemetryError::InvalidContext)?).map(Some)
    }

    /// Captures the current span's context when an OpenTelemetry layer is installed.
    pub fn capture(span: &tracing::Span) -> Result<Self, TelemetryError> {
        let context = span.context();
        let binding = context.span();
        let span = binding.span_context();
        if !span.is_valid() {
            return Err(TelemetryError::Unavailable);
        }
        Self::try_new(format!(
            "00-{}-{}-{:02x}",
            span.trace_id(),
            span.span_id(),
            span.trace_flags().to_u8() & 1
        ))
    }

    /// Installs this already-trusted remote parent. Call before recording/entering
    /// the new span; no thread-local context guard crosses an await.
    pub fn attach_to(&self, span: &tracing::Span) -> Result<(), TelemetryError> {
        let trace =
            TraceId::from_hex(&self.0[3..35]).map_err(|_| TelemetryError::InvalidContext)?;
        let id = SpanId::from_hex(&self.0[36..52]).map_err(|_| TelemetryError::InvalidContext)?;
        let flags =
            u8::from_str_radix(&self.0[53..55], 16).map_err(|_| TelemetryError::InvalidContext)?;
        let context = Context::new().with_remote_span_context(SpanContext::new(
            trace,
            id,
            TraceFlags::new(flags & 1),
            true,
            TraceState::default(),
        ));
        span.set_parent(context)
            .map_err(|_| TelemetryError::Unavailable)
    }

    /// Replaces traceparent and removes baggage/tracestate in the outbound carrier.
    /// Other application headers are left intact; authorize the destination first.
    pub fn inject(&self, headers: &mut HeaderMap) -> Result<(), TelemetryError> {
        headers.insert(
            "traceparent",
            HeaderValue::from_str(&self.0).map_err(|_| TelemetryError::InvalidContext)?,
        );
        headers.remove("tracestate");
        headers.remove("baggage");
        Ok(())
    }
}
impl fmt::Debug for TraceParent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("TraceParent([REDACTED])")
    }
}
