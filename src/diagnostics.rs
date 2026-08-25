use std::fmt::Display;
use std::io::{self, Write};

pub(crate) fn write_stderr(message: impl Display) {
    let _ = writeln!(io::stderr().lock(), "{message}");
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DebugOutput(bool);

impl DebugOutput {
    pub(crate) const fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    pub(crate) fn log(self, message: impl AsRef<str>) {
        if self.0 {
            write_stderr(format_args!("[debug] {}", message.as_ref()));
        }
    }
}

#[cfg(test)]
#[path = "../tests/unit/diagnostics.rs"]
mod tests;
