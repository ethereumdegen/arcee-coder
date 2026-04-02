use std::path::PathBuf;

/// Returns the arcee config directory (~/.arcee/).
pub fn config_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".arcee")
}

/// Returns the sessions directory (~/.arcee/sessions/).
pub fn sessions_dir() -> PathBuf {
    config_dir().join("sessions")
}

/// Ensure all required directories exist.
pub fn ensure_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(config_dir())?;
    std::fs::create_dir_all(sessions_dir())?;
    Ok(())
}
