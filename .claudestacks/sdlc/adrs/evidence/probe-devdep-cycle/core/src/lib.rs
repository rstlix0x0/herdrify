//! A port and a use case generic over it.
pub trait Store {
    fn get(&self) -> u32;
}
pub fn use_case<S: Store>(s: &S) -> u32 {
    s.get() + 1
}

#[cfg(test)]
mod tests {
    // Unit test inside core using the fake from testkit.
    #[test]
    fn unit_uses_testkit_fake() {
        let fake = probe_testkit::FakeStore(41);
        assert_eq!(super::use_case(&fake), 42);
    }
}
