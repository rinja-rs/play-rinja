use std::env;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

fn main() {
    let mut rinja = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    rinja.push("rinja");

    println!("cargo::rerun-if-changed=.git/modules/rinja/refs/heads/master");
    git_run(
        "RINJA_DESCR",
        &rinja,
        ["describe", "--tags", "--long", "HEAD"],
    );
    git_run("RINJA_REV", &rinja, ["rev-parse", "HEAD"]);

    println!("cargo::rerun-if-changed=.git/modules/rinja/config");
    git_run("RINJA_URL", &rinja, ["remote", "get-url", "origin"]);
}

#[track_caller]
fn git_run(var: &str, cwd: &Path, args: impl IntoIterator<Item: AsRef<OsStr>>) {
    let output = Command::new("git")
        .args(args)
        .current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .output()
        .unwrap();
    if !output.status.success() {
        panic!("`git` returned {}", output.status);
    }

    let mut output = String::from_utf8(output.stdout).unwrap();
    output.truncate(output.trim_ascii_end().len());
    println!("cargo::rustc-env={var}={output}");
}
