//! Honeypot trap paths with bounded, expiring peer bans.
//!
//! **Risk reduced:** automated scanners probing well-known sensitive paths
//! (`/.env`, `/.git/config`, `/wp-login.php`…) and then continuing to probe the
//! application.
//!
//! **How:** [`HoneypotLayer`] matches complete request paths against an exact
//! trap list and refuses every hit with `403`. A direct hit bans the socket
//! peer's exact IP address in this process (15 minutes by default); a load a
//! page initiated is refused but not banned, so a lure cannot ban visitors.
//!
//! **Known limits:** scanners that skip the trap paths are not detected, and
//! rotating addresses (an IPv6 prefix, a botnet) avoids the ban. Bans are not
//! shared between instances and are lost on restart. A shared NAT address can
//! be banned by one direct scanner.
//!
//! **Operator duties:** take the peer address from the socket or a trusted
//! proxy, choose trap paths your application never serves, and use network
//! controls for sustained abuse.

mod bans;
pub mod middleware;

pub use middleware::{
    DEFAULT_HONEYPOT_BAN_TTL, DEFAULT_MAX_HONEYPOT_BANS, HoneypotLayer, HoneypotService,
    HoneypotState, MAX_HONEYPOT_TRAP_PATHS,
};
