// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use serde::Serialize;
use std::io;
use std::str::FromStr;

/// A workload boundary; comparisons are meaningful only within the same workload and parameters.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Workload {
    /// `FieldSection::interpret`, discarding undeclared fields.
    Interpret,
    /// `FieldSection::partition`, collecting undeclared fields into a borrowed `Vec<FieldRef>`.
    Partition,
}

/// How the section's fields arrive.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Variant {
    /// Names as text in mixed case, as an HTTP/1.1 parser passes them.
    Text,
    /// Lowercase names, those in the static table classified by declaration id, as an HTTP/2 or HTTP/3 decoder passes them.
    Indexed,
}

macro_rules! parse_enum {
    ($ty:ty, $($text:literal => $variant:path),+ $(,)?) => {
        impl FromStr for $ty {
            type Err = io::Error;
            fn from_str(value: &str) -> io::Result<Self> {
                match value { $($text => Ok($variant),)+ _ => Err(invalid(format!("invalid {}: {value}", stringify!($ty)))) }
            }
        }
    };
}

parse_enum!(Workload, "interpret" => Workload::Interpret, "partition" => Workload::Partition);
parse_enum!(Variant, "text" => Variant::Text, "indexed" => Variant::Indexed);

/// All workload parameters, repeated in the output record for reproducibility.
#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkConfig {
    pub workload: Workload,
    pub variant: Variant,
    /// Measured sections; each is one interpretation of the whole request.
    pub iterations: u64,
    /// Sections interpreted before measurement.
    pub warmup: u64,
}

impl Default for BenchmarkConfig {
    fn default() -> Self {
        Self {
            workload: Workload::Interpret,
            variant: Variant::Text,
            iterations: 5_000_000,
            warmup: 100_000,
        }
    }
}

impl BenchmarkConfig {
    /// Parse explicit `--name value` options. Unknown, duplicate, and incomplete options are errors.
    pub fn parse(arguments: impl IntoIterator<Item = String>) -> io::Result<Self> {
        let mut config = Self::default();
        let mut arguments = arguments.into_iter();
        let mut seen = std::collections::HashSet::new();
        while let Some(name) = arguments.next() {
            if !seen.insert(name.clone()) {
                return Err(invalid(format!("duplicate option: {name}")));
            }
            let value = arguments
                .next()
                .ok_or_else(|| invalid(format!("missing value for {name}")))?;
            match name.as_str() {
                "--workload" => config.workload = value.parse()?,
                "--variant" => config.variant = value.parse()?,
                "--iterations" => config.iterations = number(&name, &value)?,
                "--warmup" => config.warmup = number(&name, &value)?,
                _ => return Err(invalid(format!("unknown option: {name}"))),
            }
        }
        config.validate()?;
        Ok(config)
    }

    /// Reject configurations that cannot be measured, before measuring.
    // SECURITY: Counts have no upper limit, so huge values can run for a very long time. Arguments come only from the operator and the repository's scenario metadata, who can already run arbitrary code here, and every failure exits without a record.
    pub fn validate(&self) -> io::Result<()> {
        if self.iterations == 0 {
            return Err(invalid("iterations must be nonzero"));
        }
        self.operations()
            .ok_or_else(|| invalid("iterations * fields per section overflows u64"))?;
        Ok(())
    }

    /// The measured operations: one per field, so that per-operation figures are per field.
    pub fn operations(&self) -> Option<u64> {
        self.iterations
            .checked_mul(crate::FIELDS_PER_SECTION as u64)
    }
}

fn number<T: FromStr>(name: &str, value: &str) -> io::Result<T> {
    value
        .parse()
        .map_err(|_| invalid(format!("invalid nonnegative integer for {name}: {value}")))
}

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}
