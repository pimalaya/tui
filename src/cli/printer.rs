use std::{
    fmt,
    io::{stderr, stdout, Stderr, Stdout, Write},
    str::FromStr,
};

use clap::ValueEnum;
use color_eyre::{
    eyre::{bail, Context, Error},
    Result,
};
use serde::Serialize;

/// Represents the available output formats.
#[derive(Clone, Debug, Default, Eq, PartialEq, Ord, PartialOrd, ValueEnum)]
pub enum OutputFmt {
    #[default]
    Plain,
    Json,
}

impl FromStr for OutputFmt {
    type Err = Error;

    fn from_str(fmt: &str) -> Result<Self, Self::Err> {
        match fmt {
            fmt if fmt.eq_ignore_ascii_case("json") => Ok(Self::Json),
            fmt if fmt.eq_ignore_ascii_case("plain") => Ok(Self::Plain),
            unknown => bail!("cannot parse output format {unknown}"),
        }
    }
}

impl fmt::Display for OutputFmt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fmt = match *self {
            OutputFmt::Json => "JSON",
            OutputFmt::Plain => "Plain",
        };

        write!(f, "{}", fmt)
    }
}

/// Defines a struct-wrapper to provide a JSON output.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct OutputJson<T: Serialize> {
    response: T,
}

impl<T: Serialize> OutputJson<T> {
    pub fn new(response: T) -> Self {
        Self { response }
    }
}

pub trait PrintTable {
    fn print(&self, writer: &mut dyn Write, table_max_width: Option<u16>) -> Result<()>;
}

pub trait Printer {
    fn out<T: fmt::Display + serde::Serialize>(&mut self, data: T) -> Result<()>;

    fn log<T: fmt::Display + serde::Serialize>(&mut self, data: T) -> Result<()> {
        self.out(data)
    }

    fn is_json(&self) -> bool {
        false
    }
}

pub struct StdoutPrinter {
    stdout: Stdout,
    stderr: Stderr,
    output: OutputFmt,
}

impl StdoutPrinter {
    pub fn new(output: OutputFmt) -> Self {
        Self {
            stdout: stdout(),
            stderr: stderr(),
            output,
        }
    }
}

impl Default for StdoutPrinter {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Printer for StdoutPrinter {
    fn out<T: fmt::Display + serde::Serialize>(&mut self, data: T) -> Result<()> {
        match self.output {
            OutputFmt::Plain => {
                write!(self.stdout, "{data}")?;
            }
            OutputFmt::Json => {
                serde_json::to_writer(&mut self.stdout, &data)
                    .context("cannot write json to writer")?;
            }
        };

        Ok(())
    }

    fn log<T: fmt::Display + serde::Serialize>(&mut self, data: T) -> Result<()> {
        if let OutputFmt::Plain = self.output {
            write!(&mut self.stderr, "{data}")?;
        }

        Ok(())
    }

    fn is_json(&self) -> bool {
        self.output == OutputFmt::Json
    }
}
