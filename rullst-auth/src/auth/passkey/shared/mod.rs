//! Explicit bound passkey ceremonies with a trusted shared store.
mod contracts;
mod manager;
#[cfg(feature = "passkey-postgres")]
mod postgres;

pub use contracts::{
    CeremonyClock, CeremonyDurability, CeremonyIntent, CeremonyKind, CeremonyStoreConfig,
    ConsumedCeremony, PasskeyBinding, PasskeyCeremonyError, PasskeyCeremonyStore,
    SystemCeremonyClock,
};
pub use manager::SharedPasskeyAuth;
#[cfg(feature = "passkey-postgres")]
pub use postgres::PostgresCeremonyStore;

#[cfg(test)]
mod contract_tests;
#[cfg(all(test, feature = "passkey-postgres"))]
mod tests;
