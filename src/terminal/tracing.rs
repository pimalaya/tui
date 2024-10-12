use std::env;

use color_eyre::{eyre::Result, Section};
use tracing_error::ErrorLayer;
use tracing_subscriber::{filter::LevelFilter, fmt, prelude::*, EnvFilter};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Tracing {
    filter: LevelFilter,
}

impl Tracing {
    pub fn install() -> Result<Self> {
        let fmt_layer = fmt::layer();

        let (filter_layer, current_filter) = match EnvFilter::try_from_default_env() {
            Err(_) => (EnvFilter::try_new("off").unwrap(), LevelFilter::OFF),
            Ok(layer) => {
                let level = layer.max_level_hint().unwrap_or(LevelFilter::OFF);
                (layer, level)
            }
        };

        let registry = tracing_subscriber::registry()
            .with(filter_layer)
            .with(ErrorLayer::default());

        if current_filter == LevelFilter::OFF {
            registry.with(fmt_layer.without_time()).init()
        } else {
            registry.with(fmt_layer).init()
        }

        if env::var("RUST_BACKTRACE").is_err() && current_filter == LevelFilter::TRACE {
            env::set_var("RUST_BACKTRACE", "1");
        }

        let debug = current_filter >= LevelFilter::DEBUG;

        color_eyre::config::HookBuilder::new()
            .capture_span_trace_by_default(debug)
            .display_location_section(debug)
            .display_env_section(false)
            .install()?;

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
