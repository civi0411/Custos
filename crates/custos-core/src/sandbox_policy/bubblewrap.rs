//! Linux bwrap unprivileged namespace containment

pub struct BubblewrapSandbox;
impl BubblewrapSandbox {
    pub fn new() -> Self {
        Self
    }
}
impl Default for BubblewrapSandbox {
    fn default() -> Self {
        Self::new()
    }
}
