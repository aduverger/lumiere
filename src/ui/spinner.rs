use std::io::{self, IsTerminal, Write};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const FRAME_INTERVAL: Duration = Duration::from_millis(80);

struct SpinnerState {
    text: String,
    running: bool,
}

pub struct Spinner {
    shared: Arc<(Mutex<SpinnerState>, Condvar)>,
    handle: Option<JoinHandle<()>>,
    is_tty: bool,
}

impl Spinner {
    pub fn new(text: impl Into<String>) -> Self {
        let is_tty = io::stdout().is_terminal();
        let shared = Arc::new((
            Mutex::new(SpinnerState {
                text: text.into(),
                running: is_tty,
            }),
            Condvar::new(),
        ));

        let handle = if is_tty {
            let shared_bg = Arc::clone(&shared);
            Some(thread::spawn(move || {
                let (lock, cvar) = &*shared_bg;
                let mut frame_idx = 0usize;
                let mut state = lock.lock().unwrap_or_else(|e| e.into_inner());
                while state.running {
                    let frame = FRAMES[frame_idx % FRAMES.len()];
                    frame_idx = frame_idx.wrapping_add(1);
                    let _ = write!(
                        io::stdout(),
                        "\r\x1b[2K\x1b[36m{frame}\x1b[0m {}",
                        state.text
                    );
                    let _ = io::stdout().flush();
                    let (guard, _) = cvar
                        .wait_timeout(state, FRAME_INTERVAL)
                        .unwrap_or_else(|e| e.into_inner());
                    state = guard;
                }
            }))
        } else {
            None
        };

        Self {
            shared,
            handle,
            is_tty,
        }
    }

    pub fn update_text(&mut self, text: impl Into<String>) {
        let (lock, _) = &*self.shared;
        let mut state = lock.lock().unwrap_or_else(|e| e.into_inner());
        state.text = text.into();
    }

    pub fn success(&mut self, msg: &str) {
        self.stop();
        if self.is_tty {
            let _ = write!(io::stdout(), "\r\x1b[2K\x1b[32m✓\x1b[0m {msg}\n");
        } else {
            let _ = writeln!(io::stdout(), "✓ {msg}");
        }
        let _ = io::stdout().flush();
    }

    pub fn clear(&mut self) {
        self.stop();
        if self.is_tty {
            let _ = write!(io::stdout(), "\r\x1b[2K");
            let _ = io::stdout().flush();
        }
    }

    fn stop(&mut self) {
        let (lock, cvar) = &*self.shared;
        {
            let mut state = lock.lock().unwrap_or_else(|e| e.into_inner());
            state.running = false;
        }
        cvar.notify_all();
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        if self.handle.is_some() {
            self.clear();
        }
    }
}
