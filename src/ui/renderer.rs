/// The renderer runs on a dedicated std::thread with the smol async runtime,
/// hosting the iocraft render_loop.

use crate::ui::bridge::UiHandle;
use crate::ui::components::App;
use iocraft::prelude::*;

/// Install a panic hook that restores the terminal before printing the panic.
fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        // Restore terminal so the panic message is legible
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(
            std::io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::cursor::Show,
        );
        default_hook(info);
    }));
}

/// Spawn the UI thread. Returns a JoinHandle so the main thread can wait for it.
pub fn spawn_ui_thread(handle: UiHandle) -> std::thread::JoinHandle<()> {
    install_panic_hook();

    std::thread::Builder::new()
        .name("iocraft-ui".into())
        .spawn(move || {
            smol::block_on(async {
                let result = element! {
                    App(ui_handle: handle)
                }
                .render_loop()
                .await;

                if let Err(e) = result {
                    // Restore terminal before printing error
                    let _ = crossterm::terminal::disable_raw_mode();
                    eprintln!("\x1b[31m[UI thread error: {e}]\x1b[0m");
                }
            });
        })
        .expect("failed to spawn UI thread")
}
