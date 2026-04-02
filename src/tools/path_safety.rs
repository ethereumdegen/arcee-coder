use std::path::{Path, PathBuf};

/// Resolve a path relative to cwd, then validate it's safe.
/// Returns the canonicalized path or an error message.
pub fn resolve_and_validate(path: &str, cwd: &Path) -> Result<PathBuf, String> {
    let raw = PathBuf::from(path);
    let resolved = if raw.is_absolute() { raw } else { cwd.join(raw) };

    // Block device paths
    let resolved_str = resolved.to_string_lossy();
    if resolved_str.starts_with("/dev/")
        || resolved_str.starts_with("/proc/")
        || resolved_str.starts_with("/sys/")
    {
        return Err(format!(
            "Access denied: {} is a system path",
            resolved.display()
        ));
    }

    // Canonicalize to resolve symlinks and .. components
    // For paths that don't exist yet (write), canonicalize the parent
    let canonical = if resolved.exists() {
        resolved
            .canonicalize()
            .map_err(|e| format!("Cannot resolve path {}: {}", resolved.display(), e))?
    } else {
        // For new files, canonicalize parent and append filename
        let parent = resolved.parent().unwrap_or(Path::new("/"));
        let filename = resolved
            .file_name()
            .ok_or_else(|| format!("Invalid file path: {}", resolved.display()))?;

        if parent.exists() {
            let canonical_parent = parent
                .canonicalize()
                .map_err(|e| format!("Cannot resolve parent {}: {}", parent.display(), e))?;
            canonical_parent.join(filename)
        } else {
            // Parent doesn't exist yet — allow it (write will create dirs)
            resolved
        }
    };

    Ok(canonical)
}

/// Safely truncate a string at a character boundary (never mid-UTF8).
pub fn safe_truncate(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    // Find the last valid character boundary at or before max_bytes
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_truncate_ascii() {
        assert_eq!(safe_truncate("hello world", 5), "hello");
    }

    #[test]
    fn test_safe_truncate_utf8() {
        // '日' is 3 bytes
        let s = "日本語";
        assert_eq!(safe_truncate(s, 3), "日");
        assert_eq!(safe_truncate(s, 4), "日"); // mid-char, backs up
        assert_eq!(safe_truncate(s, 6), "日本");
    }

    #[test]
    fn test_dev_paths_blocked() {
        let cwd = Path::new("/tmp");
        assert!(resolve_and_validate("/dev/zero", cwd).is_err());
        assert!(resolve_and_validate("/proc/self/environ", cwd).is_err());
        assert!(resolve_and_validate("/sys/class/net", cwd).is_err());
    }
}
