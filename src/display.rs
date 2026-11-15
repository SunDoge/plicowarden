//! Application density is independent of the operating system's backing scale.
use plico_desktop::WindowOptions;

pub const fn default_scale(macos: bool) -> f64 {
    if macos { 0.85 } else { 1. }
}

pub fn window_options(custom_titlebar: bool) -> WindowOptions {
    // Match the official desktop client's first-launch footprint. Recipes use
    // their own explicit viewport, so this only affects the real application.
    let (width, height, min_width, min_height) = (950, 720, 600, 500);
    WindowOptions::new()
        .title("Plicowarden")
        .decorations(!custom_titlebar)
        .linux_csd(cfg!(target_os = "linux"))
        .corner_radius(16)
        .initial_inner_size(width, height)
        .min_inner_size(min_width, min_height)
}
