use std::env;

use color_eyre::Result;

use crate::terminal::tracing::Tracing;

pub fn install() -> Result<Tracing> {
    if env::var("RUST_LOG").is_err() {
        if std::env::args().any(|arg| arg == "--quiet") {
            env::set_var("RUST_LOG", "off");
        } else if std::env::args().any(|arg| arg == "--debug") {
            env::set_var("RUST_LOG", "debug");
        } else if std::env::args().any(|arg| arg == "--trace") {
            env::set_var("RUST_LOG", "trace");
            env::set_var("RUST_BACKTRACE", "1");
        }
    }

    Tracing::install()
}
