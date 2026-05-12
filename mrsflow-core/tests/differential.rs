//! Differential harness integration tests.
//!
//! Wraps the bash scripts under `tools/grammar-fuzz/` so they're reachable
//! from `cargo test --ignored`. Marked `#[ignore]` because the runs spawn
//! scryer-prolog once per corpus case and take tens of seconds; they aren't
//! suitable for the fast inner loop, but they belong in the regression
//! surface so the Prolog companion can't silently rot.
//!
//! Skip cleanly when `bash` or `scryer-prolog` is missing (Windows boxes
//! without git-bash, fresh checkouts that haven't installed scryer) — the
//! point is that a developer with the tools gets the test, and CI failure
//! is the right signal for a developer without them.
//!
//! Run with:
//!   cargo test --test differential -- --ignored

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("CARGO_MANIFEST_DIR has a parent (workspace root)")
        .to_path_buf()
}

fn command_available(cmd: &str) -> bool {
    Command::new(cmd)
        .arg("--version")
        .output()
        .map(|out| out.status.success() || !out.stdout.is_empty() || !out.stderr.is_empty())
        .unwrap_or(false)
}

fn run_diff(script: &str) {
    if !command_available("bash") {
        eprintln!("SKIP: bash not on PATH");
        return;
    }
    if !command_available("scryer-prolog") {
        eprintln!("SKIP: scryer-prolog not on PATH");
        return;
    }

    let path = repo_root().join("tools").join("grammar-fuzz").join(script);
    let output = Command::new("bash")
        .arg(&path)
        .output()
        .expect("failed to spawn bash");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        output.status.success(),
        "{script} reported failures:\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
}

#[test]
#[ignore = "slow: spawns scryer-prolog per case; run with --ignored"]
fn parser_differential() {
    run_diff("diff_parser.sh");
}

#[test]
#[ignore = "slow: spawns scryer-prolog per case; run with --ignored"]
fn eval_differential() {
    run_diff("diff_eval.sh");
}
