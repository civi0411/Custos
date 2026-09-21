//! Standard System Tools Adapter

pub struct StandardTools;
impl StandardTools {
    pub fn new() -> Self {
        Self
    }
}
impl Default for StandardTools {
    fn default() -> Self {
        Self::new()
    }
}
