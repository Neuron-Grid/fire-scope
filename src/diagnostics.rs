#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DebugOutput(bool);

impl DebugOutput {
    pub(crate) const fn new(enabled: bool) -> Self {
        Self(enabled)
    }

    pub(crate) fn log(self, message: impl AsRef<str>) {
        if self.0 {
            eprintln!("[debug] {}", message.as_ref());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::DebugOutput;

    #[test]
    fn stores_debug_state_without_global_mutation() {
        assert_eq!(DebugOutput::new(true), DebugOutput(true));
        assert_eq!(DebugOutput::default(), DebugOutput(false));
    }
}
