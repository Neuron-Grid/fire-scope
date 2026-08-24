use super::DebugOutput;

#[test]
fn stores_debug_state_without_global_mutation() {
    assert_eq!(DebugOutput::new(true), DebugOutput(true));
    assert_eq!(DebugOutput::default(), DebugOutput(false));
}
