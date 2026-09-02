use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-env-changed=BUILD_VERSION");
    println!("cargo:rerun-if-changed=../../scripts/version.sh");

    let version = env::var("BUILD_VERSION")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(version_from_git)
        .unwrap_or_else(|| "development".to_owned());

    println!("cargo:rustc-env=BUILD_VERSION={version}");
}

fn version_from_git() -> Option<String> {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR")?);
    let script = manifest_dir.join("../../scripts/version.sh");
    let output = Command::new(script).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
