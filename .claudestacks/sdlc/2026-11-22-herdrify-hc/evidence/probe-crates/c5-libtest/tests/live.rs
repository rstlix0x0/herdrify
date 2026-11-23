//! C5 probe: what a passing, early-returning test prints.

#[test]
fn live_test_skips_when_hc_missing() {
    if std::env::var_os("C5_HC_PRESENT").is_none() {
        eprintln!("hc not installed; live test skipped");
        return;
    }
    unreachable!("never reached in this probe");
}

#[test]
fn passing_test_prints_warning_prefix() {
    eprintln!("warning: stderr line from a passing test");
    println!("warning: stdout line from a passing test");
}
