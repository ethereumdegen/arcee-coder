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

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Root of the arcee-coder repository.
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Directory for test logs.
fn log_dir() -> PathBuf {
    let dir = repo_root().join("test-tmp").join("logs");
    std::fs::create_dir_all(&dir).expect("failed to create log dir");
    dir
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
/// Also writes full stdout/stderr to a log file in test-tmp/logs/.
fn run_arcee(cwd: &Path, prompt: &str, max_turns: usize, test_name: &str) -> (std::process::ExitStatus, String, String) {
    let output = Command::new(arcee_bin())
        .args([
            prompt,
            "--permission-mode", "bypass",
            "--max-turns", &max_turns.to_string(),
            "--output-format", "text",
        ])
        .current_dir(cwd)
        .env("ARCEE_PERMISSION_STRICTNESS", "low")
        .env("RUST_LOG", "arcee_code=debug")
        .output()
        .expect("failed to execute arcee binary");

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Write detailed log file
    let log_path = log_dir().join(format!("{test_name}.log"));
    if let Ok(mut f) = std::fs::File::create(&log_path) {
        let _ = writeln!(f, "=== TEST: {test_name} ===");
        let _ = writeln!(f, "CWD: {}", cwd.display());
        let _ = writeln!(f, "Prompt: {prompt}");
        let _ = writeln!(f, "Max turns: {max_turns}");
        let _ = writeln!(f, "Exit status: {}", output.status);
        let _ = writeln!(f, "");
        let _ = writeln!(f, "==================== STDOUT ====================");
        let _ = writeln!(f, "{stdout}");
        let _ = writeln!(f, "");
        let _ = writeln!(f, "==================== STDERR ====================");
        let _ = writeln!(f, "{stderr}");
    }
    println!("  Log written to: {}", log_path.display());

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
// requires live API + ARCEE_API_KEY
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

    let (status, stdout, stderr) = run_arcee(&work_dir, &prompt, 30, "update_phone_number");

    println!("=== arcee exited with: {} ===", status);

    // The agent should have succeeded (exit 0).
    assert!(
        status.success(),
        "arcee exited with non-zero status: {status}\nSee test-tmp/logs/update_phone_number.log for details"
    );

    // Now verify the phone number was updated in the source files.
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

    // Append file state to the log for debugging
    let log_path = log_dir().join("update_phone_number.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().append(true).open(&log_path) {
        let _ = writeln!(f, "");
        let _ = writeln!(f, "==================== FOUNDER.TSX AFTER RUN ====================");
        let _ = writeln!(f, "{content}");
    }

    // Check that the new phone number appears (in display format or tel: format)
    let has_display = content.contains(new_phone);                  // 734-555-9526
    let has_tel = content.contains("7345559526");                    // tel:7345559526
    let has_tel_dashes = content.contains("734-555-9526");           // tel:734-555-9526

    assert!(
        has_display || has_tel || has_tel_dashes,
        "New phone number '{new_phone}' not found in Founder.tsx.\n\
         See test-tmp/logs/update_phone_number.log for full details.\n\
         Relevant lines:\n{}",
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
        "Old phone number '734-444-9526' still present in Founder.tsx — agent did not replace it.\n\
         See test-tmp/logs/update_phone_number.log for full details."
    );

    println!("=== PASS: Phone number successfully updated to {new_phone} ===");
}

/// Ask the agent to find the phone number on the landing page and verify
/// it correctly reports 734-444-9526.
#[test]
// requires live API + ARCEE_API_KEY
fn test_read_phone_number_from_landing_page() {
    ensure_binary_built();

    let work_dir = prepare_starflask_temp("read_phone_number");
    let prompt = "What is the phone number on the landing page on the website in this repo?";

    println!("=== Running arcee agentic loop ===");
    println!("  CWD:    {}", work_dir.display());
    println!("  Prompt: {prompt}");

    let (status, stdout, _stderr) = run_arcee(&work_dir, prompt, 20, "read_phone_number");

    println!("=== arcee exited with: {} ===", status);

    assert!(
        status.success(),
        "arcee exited with non-zero status: {status}\nSee test-tmp/logs/read_phone_number.log for details"
    );

    // The agent's stdout should contain the phone number 734-444-9526 somewhere
    let expected_phone = "734-444-9526";
    let expected_digits = "7344449526";

    let has_dashed = stdout.contains(expected_phone);
    let has_digits = stdout.contains(expected_digits);

    assert!(
        has_dashed || has_digits,
        "Expected phone number '{expected_phone}' not found in arcee output.\n\
         See test-tmp/logs/read_phone_number.log for full details."
    );

    println!("=== PASS: Agent correctly reported phone number as {expected_phone} ===");
}

/// A lightweight sanity test: ask arcee to list files and verify it exits cleanly.
#[test]
// requires live API + ARCEE_API_KEY
fn test_sanity_file_listing() {
    ensure_binary_built();

    let work_dir = prepare_starflask_temp("sanity_listing");
    let (status, _stdout, _stderr) = run_arcee(
        &work_dir,
        "List all .tsx files in frontend/src/components/sections/ and print their names.",
        10,
        "sanity_file_listing",
    );

    assert!(
        status.success(),
        "arcee exited with non-zero status\nSee test-tmp/logs/sanity_file_listing.log for details"
    );
    println!("=== PASS: sanity file listing completed ===");
}
