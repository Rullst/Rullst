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
