use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

/// Braille raindrops animation shown while the model is thinking.
///
/// Ported from the `unicode-animations` npm package's "rain" animation
/// used by stark-bot.
pub struct ThinkingIndicator {
    running: Arc<AtomicBool>,
    handle: Option<thread::JoinHandle<()>>,
}

/// Generate the 12 frames of the rain animation as braille strings.
///
/// Uses an 8-column × 4-row grid, rendered as 4 braille characters per frame.
/// Each column has a staggered offset so drops fall at different times.
fn gen_rain_frames() -> Vec<String> {
    const W: usize = 8;
    const H: usize = 4;
    const TOTAL_FRAMES: usize = 12;
    const OFFSETS: [usize; 8] = [0, 3, 1, 5, 2, 7, 4, 6];

    let mut frames = Vec::with_capacity(TOTAL_FRAMES);

    for f in 0..TOTAL_FRAMES {
        let mut grid = [[false; W]; H];
        for c in 0..W {
            let row = (f + OFFSETS[c]) % (H + 2);
            if row < H {
                grid[row][c] = true;
            }
        }
        frames.push(grid_to_braille(&grid));
    }

    frames
}

/// Convert a boolean grid (H rows × W cols) to a braille string.
///
/// Each braille character encodes a 2-wide × 4-tall cell.
/// Unicode braille pattern U+2800 + dot bits:
///   col0 row0 → bit 0    col1 row0 → bit 3
///   col0 row1 → bit 1    col1 row1 → bit 4
///   col0 row2 → bit 2    col1 row2 → bit 5
///   col0 row3 → bit 6    col1 row3 → bit 7
fn grid_to_braille(grid: &[[bool; 8]; 4]) -> String {
    let mut result = String::new();
    // 8 columns / 2 = 4 braille characters
    for cell_col in 0..4 {
        let c = cell_col * 2;
        let mut bits: u8 = 0;
        // Left column
        if grid[0][c] { bits |= 1 << 0; }
        if grid[1][c] { bits |= 1 << 1; }
        if grid[2][c] { bits |= 1 << 2; }
        if grid[3][c] { bits |= 1 << 6; }
        // Right column
        if c + 1 < 8 {
            if grid[0][c + 1] { bits |= 1 << 3; }
            if grid[1][c + 1] { bits |= 1 << 4; }
            if grid[2][c + 1] { bits |= 1 << 5; }
            if grid[3][c + 1] { bits |= 1 << 7; }
        }
        result.push(char::from_u32(0x2800 + bits as u32).unwrap_or(' '));
    }
    result
}

impl ThinkingIndicator {
    /// Start the thinking indicator animation on a background thread.
    pub fn start() -> Self {
        let running = Arc::new(AtomicBool::new(true));
        let r = running.clone();

        let handle = thread::spawn(move || {
            let frames = gen_rain_frames();
            let label = "\x1b[2m\x1b[3mthinking\x1b[0m "; // dimmed italic
            let mut frame_idx = 0;

            while r.load(Ordering::Relaxed) {
                let frame = &frames[frame_idx % frames.len()];
                // \r moves to start of line, write label + animation, clear rest of line
                print!("\r{label}{frame}\x1b[K");
                let _ = std::io::stdout().flush();
                frame_idx += 1;
                thread::sleep(Duration::from_millis(100));
            }

            // Clear the line when done
            print!("\r\x1b[K");
            let _ = std::io::stdout().flush();
        });

        Self {
            running,
            handle: Some(handle),
        }
    }

    /// Stop the animation and clean up.
    pub fn stop(mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

impl Drop for ThinkingIndicator {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}
