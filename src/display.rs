//! Application density is independent of the operating system's backing scale.
use plico_desktop::WindowOptions;

pub const fn default_scale(macos: bool) -> f64 {
    if macos { 0.85 } else { 1. }
}

pub fn window_options(macos: bool) -> WindowOptions {
    let (width, height, min_width, min_height) = if macos {
        (1224, 850, 697, 544)
    } else {
        (1440, 1000, 820, 640)
    };
    WindowOptions::new()
        .title("Plicowarden")
        .titlebar_overlay(macos)
        .initial_inner_size(width, height)
        .min_inner_size(min_width, min_height)
}
