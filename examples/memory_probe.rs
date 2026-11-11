//! Synthetic resize/page stress. RSS includes the driver; counters exclude driver overhead.
use plico_renderer::{
    FrameSource, GesturePhase, Modifiers, Point, Scene, TextContext, UiEvent, WheelDeltaMode,
    WheelEvent,
};
use plico_runtime::{NativeApp, Source};
use snafu::{ResultExt, Snafu};
use std::time::Duration;

#[derive(usage_rs::Cli)]
struct Options {
    #[usage(long, default = "60")]
    frames: u32,
    #[usage(long)]
    scroll: bool,
}
#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("GPU: {source}"))]
    Gpu { source: plico_wgpu::Error },
}
fn report(renderer: &plico_wgpu::Renderer, frame: &str) {
    let rss = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &std::process::id().to_string()])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default();
    let layers = renderer.layer_pool_stats();
    let effects = renderer.effect_cache_stats();
    let images = renderer.image_cache_stats();
    let rasters = renderer.raster_cache_stats();
    let targets = renderer.target_storage_bytes();
    println!(
        "{}",
        serde_json::json!({"frame":frame,"rss_kib":rss.trim().parse::<u64>().ok(),
        "target_bytes":targets.0,"readback_bytes":targets.1,"layer_bytes":layers.retained_bytes,
        "effect_bytes":effects.1,"effect_targets":effects.0,"image_bytes":images.1,"image_sources":images.0,
        "cpu_raster_bytes":rasters.iter().map(|r|r.1).sum::<usize>()})
    );
}
fn main() -> Result<(), Error> {
    let options = Options::parse();
    let mut renderer = pollster::block_on(plico_wgpu::Renderer::new()).context(GpuSnafu)?;
    let clock = plico::VirtualClock::default();
    let mut app = plicowarden::PreviewConfig::default().app();
    if options.scroll {
        let sample = app.items.clone();
        for batch in 0..8 {
            app.items.extend(sample.iter().cloned().map(|mut item| {
                item.id = format!("{}-{batch}", item.id);
                item
            }));
        }
    }
    let mut source = Source::with_clock(NativeApp(app), clock.clone());
    let mut text = TextContext::new();
    renderer.configure_text(&mut text, 2.).context(GpuSnafu)?;
    let mut scene = Scene::new();
    report(&renderer, "start");
    for index in 0..options.frames {
        let size = if options.scroll {
            (1440, 1000)
        } else {
            (1440 + index % 7 * 12, 1000 + index % 5 * 8)
        };
        if options.scroll {
            source.handle_event(UiEvent::Wheel(WheelEvent {
                position: Point { x: 400., y: 400. },
                delta_x: 0.,
                delta_y: if index % 20 < 10 { -60. } else { 60. },
                delta_mode: WheelDeltaMode::Pixel,
                phase: GesturePhase::Changed,
                modifiers: Modifiers::empty(),
            }));
        } else {
            source.view.0.settings_open = index % 3 == 1;
            let page = if index % 3 == 2 {
                plicowarden::model::Page::Generator
            } else {
                plicowarden::model::Page::Vault
            };
            if !source.view.0.router.show(page) {
                source.view.0.router.replace(page);
            }
            source.request_rebuild();
        }
        clock.advance(Duration::from_millis(16)).unwrap();
        source.set_device_scale(2.);
        if source
            .render_retained_frame(&mut scene, &mut text, size, 2.)
            .is_none()
        {
            source.build_and_paint(&mut scene, &mut text, size.0, size.1, 2.);
        }
        drop(
            renderer
                .render_scene(&scene, [size.0 * 2, size.1 * 2], [1.; 4], false)
                .context(GpuSnafu)?,
        );
        if index % 5 == 0 || index + 1 == options.frames {
            report(&renderer, &index.to_string());
        }
    }
    renderer.trim_raster_cache();
    renderer.trim_image_cache();
    renderer.trim_layer_pool();
    renderer.trim_effect_cache();
    renderer.trim_positioned_glyph_cache();
    renderer.trim_readback_buffer();
    renderer.trim_text_cache();
    report(&renderer, "trimmed");
    Ok(())
}
