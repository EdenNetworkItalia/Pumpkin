//! A server for the event dispatch tests and bench. It binds no port and runs no ticker, and its
//! world lives in a new directory under `CARGO_TARGET_TMPDIR`.
#![allow(clippy::unwrap_used)]

use std::sync::{Arc, LazyLock};

use pumpkin_config::{AdvancedConfiguration, BasicConfiguration, TelemetryConfig};
use pumpkin_core::{data::VanillaData, server::Server};
use tokio::runtime::Runtime;

pub static RUNTIME: LazyLock<Runtime> = LazyLock::new(|| {
    tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap()
});

static SERVER: LazyLock<Arc<Server>> = LazyLock::new(|| {
    let dir = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR"))
        .unwrap()
        .keep();
    let basic = BasicConfiguration {
        default_level_name: dir.join("world").to_string_lossy().into_owned(),
        allow_nether: false,
        allow_end: false,
        ..BasicConfiguration::default()
    };
    RUNTIME
        .block_on(Server::new(
            basic,
            AdvancedConfiguration::default(),
            TelemetryConfig::default(),
            VanillaData::default(),
            Vec::new(),
        ))
        .unwrap()
});

pub fn server() -> &'static Arc<Server> {
    &SERVER
}
