use std::{env, io::stderr};

use anyhow::Result;
use tracing_error::ErrorLayer;
use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tracing {
    filter: LevelFilter,
}

impl Tracing {
    pub fn install() -> Result<Self> {
        let (filter_layer, current_filter) = match EnvFilter::try_from_default_env() {
            Err(_) => (EnvFilter::try_new("warn").unwrap(), LevelFilter::OFF),
            Ok(layer) => {
                let level = layer.max_level_hint().unwrap_or(LevelFilter::OFF);
                (layer, level)
            }
        };

        tracing_subscriber::registry()
            .with(fmt::layer().with_writer(stderr))
            .with(filter_layer)
            .with(ErrorLayer::default())
            .init();

        if env::var("RUST_BACKTRACE").is_err() && current_filter == LevelFilter::TRACE {
            env::set_var("RUST_BACKTRACE", "1");
        }

        let debug = current_filter >= LevelFilter::DEBUG;

        // anyhow::config::HookBuilder::new()
        //     .capture_span_trace_by_default(debug)
        //     .display_location_section(debug)
        //     .display_env_section(false)
        //     .install()?;

        Ok(Self {
            filter: current_filter,
        })
    }

    pub fn with_debug_and_trace_notes<T>(&self, mut res: Result<T>) -> Result<T> {
        if self.filter < LevelFilter::DEBUG {
            res = res.note("Run with --debug to enable logs with spantrace.");
        };

        if self.filter < LevelFilter::TRACE {
            res = res.note("Run with --trace to enable verbose logs with backtrace.")
        };

        res
    }
}
