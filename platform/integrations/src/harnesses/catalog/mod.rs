//! Static harness catalog defining all standard native and discovered CLI agents.

use once_cell::sync::Lazy;

use super::spec::HarnessSpec;

pub static SPECS: Lazy<Vec<HarnessSpec>> = Lazy::new(|| {
    serde_json::from_str(include_str!("../../../data/harnesses.json"))
        .expect("bundled harnesses.json catalog must be valid")
});

pub fn find_spec(id: &str) -> Option<&'static HarnessSpec> {
    SPECS.iter().find(|s| s.id == id)
}

#[cfg(test)]
mod tests;
