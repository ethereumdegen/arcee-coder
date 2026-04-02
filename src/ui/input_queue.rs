use std::io::BufRead;
use std::os::unix::io::AsRawFd;

/// Drain any complete lines that the user typed ahead into stdin while the
/// query loop was running. Uses non-blocking reads so it never waits — it
/// just grabs whatever is already buffered in the terminal's line discipline.
///
/// This works because during query_loop the terminal is in cooked mode: each
/// Enter-pressed line sits in the stdin buffer until someone reads it.
/// Rustyline isn't active during query_loop, so the lines accumulate.
pub fn drain_pending_stdin() -> Vec<String> {
    let stdin = std::io::stdin();
    let fd = stdin.as_raw_fd();

    // Save current flags and set non-blocking
    let flags = unsafe { libc::fcntl(fd, libc::F_GETFL) };
    if flags < 0 {
        return Vec::new();
    }
    unsafe { libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK) };

    let mut lines = Vec::new();
    let locked = stdin.lock();
    let mut reader = std::io::BufReader::new(locked);

    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,    // EOF
            Ok(_) => {
                let trimmed = line.trim().to_string();
                if !trimmed.is_empty() {
                    lines.push(trimmed);
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(_) => break,
        }
    }

    // Restore blocking mode
    unsafe { libc::fcntl(fd, libc::F_SETFL, flags) };

    lines
}
