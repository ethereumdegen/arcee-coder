//! LLM-in-the-loop integration tests.
//!
//! These tests are **non-deterministic** by nature: they spin up a full
//! arcee-coder agentic loop against the real LLM API and assert on the
//! *observable side-effects* (file edits, etc.) rather than exact output.
//!
//! Requirements:
//!   - `ARCEE_API_KEY` must be set in the environment.
//!   - The `arcee` binary must be buildable (`cargo build`).
//!   - The `test-repo/` fixture directory must exist at the repo root.
//!
//! Run with:
//!   cargo test --test llm_loop -- --nocapture
//!
//! Because these hit a live API and can take 30-120 s each, they are gated
//! behind `#[ignore]` so they don't run on every `cargo test`.  Use:
//!   cargo test --test llm_loop -- --ignored --nocapture

use std::path::{Path, PathBuf};
use std::process::Command;

/// Root of the arcee-coder repository.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Path to the compiled `arcee` binary (debug build).
fn arcee_bin() -> PathBuf {
    repo_root().join("target").join("debug").join("arcee")
}

/// Copy an entire directory tree from `src` to `dst`.
fn copy_dir_recursive(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            // Skip node_modules and target dirs to speed up copies
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == "node_modules" || name_str == "target" || name_str == ".git" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}

/// Ensure the arcee binary is built.
fn ensure_binary_built() {
    if !arcee_bin().exists() {
        let status = Command::new("cargo")
            .args(["build"])
            .current_dir(repo_root())
            .status()
            .expect("failed to run cargo build");
        assert!(status.success(), "cargo build failed");
    }
}

/// Run the arcee agentic loop on a working directory with a given prompt.
///
/// Returns `(exit_status, stdout, stderr)`.
fn run_arcee(cwd: &Path, prompt: &str, max_turns: usize) -> (std::process::ExitStatus, String, String) {
    let output = Command::new(arcee_bin())
        .args([
            prompt,
            "--permission-mode", "bypass",
            "--max-turns", &max_turns.to_string(),
            "--output-format", "text",
        ])
        .current_dir(cwd)
        .env("ARCEE_PERMISSION_STRICTNESS", "low")
        .output()
        .expect("failed to execute arcee binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    (output.status, stdout, stderr)
}

// ---------------------------------------------------------------------------
// Test fixtures
// ---------------------------------------------------------------------------

/// Source fixture: test-repo/starflask-digital-web
fn starflask_fixture() -> PathBuf {
    repo_root().join("test-repo").join("starflask-digital-web")
}

/// Create a fresh temp copy of the starflask fixture and return its path.
/// The copy lives under `test-tmp/` which is .gitignored.
fn prepare_starflask_temp(test_name: &str) -> PathBuf {
    let tmp = repo_root().join("test-tmp").join(test_name);
    // Clean any previous run
    if tmp.exists() {
        std::fs::remove_dir_all(&tmp).expect("failed to clean previous temp dir");
    }
    let src = starflask_fixture();
    assert!(
        src.exists(),
        "Fixture not found at {}. Run: cp -r ~/starflask_digital/starflask-digital-web test-repo/",
        src.display()
    );
    copy_dir_recursive(&src, &tmp).expect("failed to copy fixture to temp dir");
    tmp
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
#[ignore] // requires live API — run with: cargo test --test llm_loop -- --ignored --nocapture
fn test_update_phone_number_on_landing_page() {
    ensure_binary_built();

    let work_dir = prepare_starflask_temp("phone_number_update");
    let new_phone = "734-555-9526";
    let prompt = format!(
        "Update the cell phone number on the landing page to {new_phone}. \
         Look through the frontend source files for phone numbers and update them."
    );

    println!("=== Running arcee agentic loop ===");
    println!("  CWD:    {}", work_dir.display());
    println!("  Prompt: {prompt}");

    let (status, stdout, stderr) = run_arcee(&work_dir, &prompt, 30);

    println!("=== arcee exited with: {} ===", status);
    println!("--- stdout (last 2000 chars) ---");
    let stdout_tail: String = stdout.chars().rev().take(2000).collect::<Vec<_>>().into_iter().rev().collect();
    println!("{stdout_tail}");
    println!("--- stderr (last 2000 chars) ---");
    let stderr_tail: String = stderr.chars().rev().take(2000).collect::<Vec<_>>().into_iter().rev().collect();
    println!("{stderr_tail}");

    // The agent should have succeeded (exit 0).
    assert!(
        status.success(),
        "arcee exited with non-zero status: {status}"
    );

    // Now verify the phone number was updated in the source files.
    // We check multiple representations of the phone number.
    let founder_path = work_dir
        .join("frontend")
        .join("src")
        .join("components")
        .join("sections")
        .join("Founder.tsx");

    assert!(
        founder_path.exists(),
        "Founder.tsx not found at {}",
        founder_path.display()
    );

    let content = std::fs::read_to_string(&founder_path)
        .expect("failed to read Founder.tsx");

    // Check that the new phone number appears (in display format or tel: format)
    let has_display = content.contains(new_phone);                  // 734-555-9526
    let has_tel = content.contains("7345559526");                    // tel:7345559526
    let has_tel_dashes = content.contains("734-555-9526");           // tel:734-555-9526

    assert!(
        has_display || has_tel || has_tel_dashes,
        "New phone number '{new_phone}' not found in Founder.tsx.\n\
         File content (relevant lines):\n{}",
        content
            .lines()
            .filter(|l| l.to_lowercase().contains("phone") || l.contains("tel") || l.contains("734") || l.contains("555"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // Also verify the OLD number is gone
    let old_gone = !content.contains("734-444-9526") && !content.contains("7344449526");
    assert!(
        old_gone,
        "Old phone number '734-444-9526' still present in Founder.tsx — agent did not replace it."
    );

    println!("=== PASS: Phone number successfully updated to {new_phone} ===");
}

/// Ask the agent to find the phone number on the landing page and verify
/// it correctly reports 734-444-9526.
#[test]
#[ignore] // requires live API — run with: cargo test --test llm_loop -- --ignored --nocapture
fn test_read_phone_number_from_landing_page() {
    ensure_binary_built();

    let work_dir = prepare_starflask_temp("read_phone_number");
    let prompt = "What is the phone number on the landing page on the website in this repo?";

    println!("=== Running arcee agentic loop ===");
    println!("  CWD:    {}", work_dir.display());
    println!("  Prompt: {prompt}");

    let (status, stdout, stderr) = run_arcee(&work_dir, prompt, 20);

    println!("=== arcee exited with: {} ===", status);
    println!("--- stdout (last 3000 chars) ---");
    let stdout_tail: String = stdout.chars().rev().take(3000).collect::<Vec<_>>().into_iter().rev().collect();
    println!("{stdout_tail}");
    println!("--- stderr (last 1000 chars) ---");
    let stderr_tail: String = stderr.chars().rev().take(1000).collect::<Vec<_>>().into_iter().rev().collect();
    println!("{stderr_tail}");

    assert!(
        status.success(),
        "arcee exited with non-zero status: {status}"
    );

    // The agent's stdout should contain the phone number 734-444-9526 somewhere
    // in its response. Check for various representations.
    let expected_phone = "734-444-9526";
    let expected_digits = "7344449526";

    let has_dashed = stdout.contains(expected_phone);
    let has_digits = stdout.contains(expected_digits);

    assert!(
        has_dashed || has_digits,
        "Expected phone number '{expected_phone}' not found in arcee output.\n\
         stdout (last 3000 chars):\n{stdout_tail}"
    );

    println!("=== PASS: Agent correctly reported phone number as {expected_phone} ===");
}

/// A lightweight sanity test: ask arcee to list files and verify it exits cleanly.
#[test]
#[ignore]
fn test_sanity_file_listing() {
    ensure_binary_built();

    let work_dir = prepare_starflask_temp("sanity_listing");
    let (status, _stdout, _stderr) = run_arcee(
        &work_dir,
        "List all .tsx files in frontend/src/components/sections/ and print their names.",
        10,
    );

    assert!(status.success(), "arcee exited with non-zero status");
    println!("=== PASS: sanity file listing completed ===");
}
