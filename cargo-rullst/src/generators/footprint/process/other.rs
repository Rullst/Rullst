//! Platforms without a process sampler: every process figure is
//! `NOT MEASURED` with this reason.
use super::super::target::Listener;
use super::{Executable, Methods};

const UNSUPPORTED: &str = "process sampling supports Linux, macOS and Windows only";

pub(in super::super) const METHODS: Methods = Methods {
    rss: UNSUPPORTED,
    cpu: UNSUPPORTED,
    executable: UNSUPPORTED,
    lookup: UNSUPPORTED,
    unreadable: UNSUPPORTED,
};

pub(in super::super) fn rss_bytes(_pid: u32) -> Option<u64> {
    None
}

pub(in super::super) fn cpu_seconds(_pid: u32) -> Option<f64> {
    None
}

pub(in super::super) fn executable(_pid: u32) -> Option<Executable> {
    None
}

pub(in super::super) fn cpu_model() -> Option<String> {
    None
}

pub(in super::super) fn listener(_port: u16) -> Listener {
    Listener::Unknown(UNSUPPORTED)
}
