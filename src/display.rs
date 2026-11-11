//! Application density is independent of the operating system's backing scale.
use plico_desktop::WindowOptions;

pub const fn default_scale(macos: bool) -> f64 {
    if macos { 0.85 } else { 1. }
}

pub fn window_options(macos: bool) -> WindowOptions {
    // Match the official desktop client's first-launch footprint. Recipes use
    // their own explicit viewport, so this only affects the real application.
    let (width, height, min_width, min_height) = (950, 720, 600, 500);
    let native_frame = !macos && cfg!(target_os = "linux");
    WindowOptions::new()
        .title("Plicowarden")
        .decorations(native_frame)
        .corner_radius(if native_frame { 0 } else { 16 })
        .initial_inner_size(width, height)
        .min_inner_size(min_width, min_height)
}
