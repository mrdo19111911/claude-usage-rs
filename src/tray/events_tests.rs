use super::*;

#[test]
fn exit_at_once_when_no_desktop_switch_runs() {
    assert_eq!(exit_step(true, false), ExitStep::Now);
}

#[test]
fn exit_waits_while_a_desktop_switch_runs() {
    assert_eq!(exit_step(true, true), ExitStep::Wait);
}

#[test]
fn nothing_asked_nothing_done() {
    assert_eq!(exit_step(false, false), ExitStep::Stay);
    assert_eq!(exit_step(false, true), ExitStep::Stay);
}
