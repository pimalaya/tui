use std::env;

use color_eyre::Result;

use crate::tracing::Tracing;

pub fn install() -> Result<Tracing> {
    if env::var("RUST_LOG").is_err() {
        if std::env::args().any(|arg| arg == "--debug") {
            env::set_var("RUST_LOG", "debug");
        }
        if std::env::args().any(|arg| arg == "--trace") {
            env::set_var("RUST_LOG", "trace");
        }
    }

    Tracing::install()
}
