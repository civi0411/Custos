//! macOS sandbox-exec / Seatbelt profile containment

pub struct SeatbeltSandbox;
impl SeatbeltSandbox {
    pub fn new() -> Self {
        Self
    }
}
impl Default for SeatbeltSandbox {
    fn default() -> Self {
        Self::new()
    }
}
