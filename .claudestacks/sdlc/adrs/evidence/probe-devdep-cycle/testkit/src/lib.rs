//! Fake implementing core's port.
pub struct FakeStore(pub u32);
impl probe_core::Store for FakeStore {
    fn get(&self) -> u32 {
        self.0
    }
}
