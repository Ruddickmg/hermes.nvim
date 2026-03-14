//! UI waiting utilities for integration tests
//! Pattern based on e2e/src/utilities/autocommand.rs
use std::time::{Duration, Instant};

/// Poll with sleep until condition is met or timeout
pub fn wait_for<F>(condition: F, timeout: Duration) -> bool
where
    F: Fn() -> bool,
{
    let start = Instant::now();
    loop {
        if condition() {
            return true;
        }
        if start.elapsed() > timeout {
            return false;
        }
        nvim_oxi::api::command("sleep 50m").ok();
    }
}

/// Check if a floating window exists
/// Floating windows have a non-empty 'relative' field in their config
pub fn find_floating_window() -> Option<nvim_oxi::api::Window> {
    let wins = nvim_oxi::api::list_wins();
    eprintln!("[DEBUG] Total windows: {}", wins.len());

    wins.into_iter().find(|win| {
        // Floating windows have a 'relative' option set (to 'editor', 'win', or 'cursor')
        let result = nvim_oxi::api::get_option_value::<String>(
            "relative",
            &nvim_oxi::api::opts::OptionOpts::builder()
                .win(win.clone())
                .build(),
        );

        let is_floating = result
            .map(|rel| {
                let floating = !rel.is_empty();
                eprintln!("[DEBUG] Window relative='{}', floating={}", rel, floating);
                floating
            })
            .unwrap_or(false);

        is_floating
    })
}

/// Wait for floating window to appear
pub fn wait_for_floating_window(timeout: Duration) -> Option<nvim_oxi::api::Window> {
    wait_for(|| find_floating_window().is_some(), timeout)
        .then(find_floating_window)
        .flatten()
}

/// Wait for channel to receive outcome
pub fn wait_for_outcome<T>(
    receiver: &mut tokio::sync::oneshot::Receiver<T>,
    timeout: Duration,
) -> Option<T> {
    let start = Instant::now();
    loop {
        match receiver.try_recv() {
            Ok(outcome) => return Some(outcome),
            Err(_) => {
                if start.elapsed() > timeout {
                    return None;
                }
                nvim_oxi::api::command("sleep 50m").ok();
            }
        }
    }
}
