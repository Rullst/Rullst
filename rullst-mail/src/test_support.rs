//! Helpers shared by this crate's unit tests.
#![allow(clippy::unwrap_used)]

/// A loopback port nothing listens on, reserved while this value lives.
///
/// It is the local end of a connection to a listener this test owned, so a
/// connection to it is refused and no parallel test can bind it. A port read
/// from a dropped listener can be handed to another test's one-shot fixture
/// server before it is used, which then answers in place of a refusal and
/// loses the connection its own test expects.
pub(crate) struct RefusedPort {
    pub(crate) port: u16,
    _connection: (std::net::TcpStream, std::net::TcpStream),
}

impl RefusedPort {
    pub(crate) fn new() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let client = std::net::TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (server, _) = listener.accept().unwrap();
        Self {
            port: client.local_addr().unwrap().port(),
            _connection: (client, server),
        }
    }
}

/// Records the `event` field of every tracing event on the thread where it
/// is the default subscriber (`tracing::subscriber::set_default`).
#[derive(Clone, Default)]
pub(crate) struct EventNames(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl EventNames {
    /// Takes the names recorded so far.
    pub(crate) fn take(&self) -> Vec<String> {
        std::mem::take(&mut *self.0.lock().unwrap())
    }

    /// How many recorded events carry `name`.
    pub(crate) fn count(names: &[String], name: &str) -> usize {
        names.iter().filter(|recorded| *recorded == name).count()
    }
}

struct EventField<'names>(&'names mut Vec<String>);

impl tracing::field::Visit for EventField<'_> {
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "event" {
            self.0.push(value.to_string());
        }
    }

    fn record_debug(&mut self, _field: &tracing::field::Field, _value: &dyn std::fmt::Debug) {}
}

impl tracing::Subscriber for EventNames {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _attributes: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        event.record(&mut EventField(&mut self.0.lock().unwrap()));
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}
