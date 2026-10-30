use super::*;

// pid → (parent, started)
fn tree(t: &[(u32, u32, Option<u64>)]) -> (HashMap<u32, u32>, HashMap<u32, Option<u64>>) {
    (t.iter().map(|&(p, q, _)| (p, q)).collect(), t.iter().map(|&(p, _, s)| (p, s)).collect())
}

#[test]
fn a_child_of_claude_is_inside() {
    let (parent, at) = tree(&[(10, 1, Some(100)), (20, 10, Some(200)), (30, 20, Some(300))]);
    assert!(descends_from(&parent, |p| at[&p], 30, &[10]));
}

#[test]
fn a_parent_id_reused_after_the_child_started_is_no_parent() {
    // Seen 2026-10-30: the dashboard was started through a short-lived explorer.exe
    // (gone), whose id Windows then gave to a conhost.exe of Claude's, 8 minutes later.
    let (parent, at) = tree(&[(10, 1, Some(100)), (36, 10, Some(900)), (40, 36, Some(500))]);
    assert!(!descends_from(&parent, |p| at[&p], 40, &[10]));
}

#[test]
fn unknown_start_times_follow_the_ids_as_before() {
    let (parent, at) = tree(&[(10, 1, None), (20, 10, None), (30, 20, Some(300))]);
    assert!(descends_from(&parent, |p| at[&p], 30, &[10]));
}

#[test]
fn a_parent_that_is_gone_ends_the_line() {
    let (parent, at) = tree(&[(10, 1, Some(100)), (30, 99, Some(300))]);
    assert!(!descends_from(&parent, |p| at.get(&p).copied().flatten(), 30, &[10]));
}
