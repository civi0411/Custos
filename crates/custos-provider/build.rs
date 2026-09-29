use std::path::PathBuf;

fn main() {
    let out_dir = PathBuf::from(std::env::var_os("OUT_DIR").expect("OUT_DIR must be set"));
    let dest = out_dir.join("canonical_models.json.zst");
    let source = PathBuf::from("src/types/canonical/canonical_models.json.zst");
    println!("cargo:rerun-if-changed={}", source.display());

    if source.exists() {
        std::fs::copy(&source, &dest).expect("failed to copy canonical_models.json.zst");
    } else {
        std::fs::write(&dest, b"").expect("failed to write fallback zst");
    }
}
