// Integration test (separate crate) using the fake from testkit.
#[test]
fn integration_uses_testkit_fake() {
    let fake = probe_testkit::FakeStore(41);
    assert_eq!(probe_core::use_case(&fake), 42);
}
