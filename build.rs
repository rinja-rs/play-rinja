use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};

fn main() {
    println!("cargo::rerun-if-changed=.git/modules/rinja/refs/heads/master");

    let root = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let rinja = root.join("rinja");

    let output = Command::new("git")
        .args(["describe", "--tags", "--long", "HEAD"])
        .current_dir(&rinja)
        .stdout(Stdio::piped())
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("`git describe` returned {}", output.status);
    }
    let descr = String::from_utf8_lossy(output.stdout.trim_ascii());
    println!("cargo::rustc-env=RINJA_REV={descr}");
}
