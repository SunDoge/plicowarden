use plico_desktop::NativeApp;
use plico_renderer::{
    FrameSource, Modifiers, Point, PointerButton, PointerEvent, PointerPhase, TextContext, UiEvent,
};
use plico_runtime::{Source, ZoomSource};
use plicowarden::{Warden, appearance::Appearance, display};

fn source(factor: f64) -> ZoomSource {
    let app = Warden::default();
    app.zoom.set(factor);
    let zoom = app.zoom.clone();
    let mut source = ZoomSource::with_controller(Box::new(Source::new(NativeApp(app))), zoom);
    source.set_semantics_enabled(true);
    source
}
fn bounds(source: &ZoomSource, label: &str) -> [f32; 4] {
    let snapshot = source.semantic_snapshot().unwrap();
    snapshot
        .nodes
        .iter()
        .find(|n| n.label.as_deref() == Some(label))
        .unwrap_or_else(|| {
            panic!(
                "missing semantic node {label:?}; available labels: {:?}",
                snapshot
                    .nodes
                    .iter()
                    .filter_map(|node| node.label.as_deref())
                    .collect::<Vec<_>>()
            )
        })
        .bounds
}
fn click(source: &mut ZoomSource, text: &mut TextContext, label: &str) {
    let b = bounds(source, label);
    for phase in [PointerPhase::Down, PointerPhase::Up] {
        source.prepare_for_event(text).unwrap();
        source.handle_event(UiEvent::Pointer(PointerEvent {
            phase,
            position: Point {
                x: f64::from((b[0] + b[2]) / 2.),
                y: f64::from((b[1] + b[3]) / 2.),
            },
            button: Some(PointerButton::Primary),
            buttons: if phase == PointerPhase::Down { 1 } else { 0 },
            modifiers: Modifiers::empty(),
            properties: Default::default(),
        }));
    }
    source.build_frame(text, 1224, 850);
}

#[test]
fn compact_controls_keep_logical_size_and_hit_targets_at_retina_density() {
    let mut baseline = source(1.);
    let mut text = TextContext::new();
    baseline.set_device_scale(1.);
    baseline.build_frame(&mut text, 1224, 850);
    let full = bounds(&baseline, "Select GitHub");
    let mut previous = None;
    for backing_scale in [1., 2.] {
        let mut compact = source(0.85);
        let mut text = TextContext::new();
        compact.set_device_scale(backing_scale);
        compact.build_frame(&mut text, 1224, 850);
        let row = bounds(&compact, "Select GitHub");
        assert!(((row[3] - row[1]) / (full[3] - full[1]) - 0.85).abs() < 0.02);
        if let Some(previous) = previous {
            assert_eq!(row, previous);
        }
        previous = Some(row);
        click(&mut compact, &mut text, "Search all vaults");
        compact.handle_event(UiEvent::TextInput("github".into()));
        compact.build_frame(&mut text, 1224, 850);
        let tree = compact.semantic_snapshot().unwrap();
        assert!(
            tree.nodes
                .iter()
                .any(|n| n.label.as_deref() == Some("Select GitHub"))
        );
        assert!(
            !tree
                .nodes
                .iter()
                .any(|n| n.label.as_deref() == Some("Select Linear"))
        );
        click(&mut compact, &mut text, "Settings");
        assert!(
            compact
                .semantic_snapshot()
                .unwrap()
                .nodes
                .iter()
                .any(|n| n.label.as_deref() == Some("Close settings"))
        );
    }
}

#[test]
fn platform_defaults_and_legacy_preferences_choose_independent_ui_scale() {
    assert_eq!(display::default_scale(true), 0.85);
    assert_eq!(display::default_scale(false), 1.);
    assert!(!display::window_options(true).decorations);
    assert!(display::window_options(false).decorations);
    assert_eq!(
        display::window_options(true).linux_csd,
        cfg!(target_os = "linux")
    );
    assert_eq!(
        display::window_options(false).linux_csd,
        cfg!(target_os = "linux")
    );
    assert_eq!(display::window_options(true).corner_radius, 8);
    assert_eq!(display::window_options(false).corner_radius, 8);
    assert!(display::window_options(true).has_shadow);
    assert!(display::window_options(false).has_shadow);
    assert_eq!(display::window_options(true).initial_inner_size, (950, 720));
    assert_eq!(
        display::window_options(false).initial_inner_size,
        (950, 720)
    );
    assert_eq!(
        display::window_options(true).min_inner_size,
        Some((600, 500))
    );
    let legacy: Appearance = serde_json::from_str(r#"{"liquidGlass":false,"frost":0.42}"#).unwrap();
    assert_eq!(legacy.ui_scale, Appearance::default().ui_scale);
    let invalid = Appearance {
        ui_scale: f64::NAN,
        ..Default::default()
    }
    .validated();
    assert_eq!(invalid.ui_scale, Appearance::default().ui_scale);
}

#[test]
fn custom_titlebar_controls_keep_point_size_and_clear_content_at_different_zoom_levels() {
    use plico_renderer::SemanticRole;
    for (width, height, factor, sidebar) in [
        (1224, 850, 0.85, true),
        (820, 640, 1., false),
        (1224, 850, 0.5, false),
        (1224, 850, 1.5, false),
    ] {
        let mut app = Warden::default();
        app.custom_titlebar = true;
        app.sidebar = sidebar;
        app.zoom.set(factor);
        let zoom = app.zoom.clone();
        let mut source = ZoomSource::with_controller(Box::new(Source::new(NativeApp(app))), zoom);
        source.set_semantics_enabled(true);
        source.set_device_scale(2.);
        source.build_frame(&mut TextContext::new(), width, height);
        for node in &source.semantic_snapshot().unwrap().nodes {
            if matches!(node.role, SemanticRole::Button | SemanticRole::TextInput) && !node.disabled
            {
                if matches!(
                    node.label.as_deref(),
                    Some("Close window" | "Minimize window" | "Maximize or restore window")
                ) {
                    assert!(
                        (node.bounds[2] - node.bounds[0] - 20.).abs() <= 1.5,
                        "{:?} at {factor}: {:?}",
                        node.label,
                        node.bounds
                    );
                    assert!((node.bounds[3] - node.bounds[1] - 24.).abs() < 1.);
                    continue;
                }
                let [x0, y0, x1, y1] = node.bounds;
                assert!(
                    !(x0 < 88. && x1 > 0. && y0 < 56. && y1 > 0.),
                    "window controls overlap {:?} at {factor}",
                    node.label
                );
            }
        }
    }
}

#[test]
fn title_toolbar_uses_compact_controls_without_shrinking_content_actions() {
    let mut source = source(1.);
    let mut text = TextContext::new();
    source.build_frame(&mut text, 1224, 850);

    for label in [
        "Toggle sidebar",
        "Search or run a command",
        "Settings",
        "Connect account",
    ] {
        let [_, top, _, bottom] = bounds(&source, label);
        assert!(
            (bottom - top - 28.).abs() < 1.,
            "{label} should use the compact 28px title-toolbar control: {top}..{bottom}"
        );
    }

    let [_, top, _, bottom] = bounds(&source, "Copy username");
    assert!(
        (bottom - top - 32.).abs() < 1.,
        "content actions should retain their 32px size: {top}..{bottom}"
    );
}

#[test]
fn native_accessibility_preferences_adapt_the_ui_without_changing_saved_appearance() {
    let mut source =
        Source::with_clock(NativeApp(Warden::default()), plico::VirtualClock::default());
    let mut text = TextContext::new();
    let mut scene = plico_renderer::Scene::new();
    source
        .render_retained_frame(&mut scene, &mut text, (1440, 1000), 1.)
        .unwrap();
    let subtitle = |source: &Source<NativeApp<Warden>>| {
        source
            .placed_nodes()
            .iter()
            .find(|node| node.paint.text.as_deref() == Some("Demo vault"))
            .unwrap()
            .paint
            .text_color
    };
    let before = subtitle(&source);
    assert!(
        source
            .handle_event(UiEvent::WindowMetrics(plico_renderer::WindowMetrics {
                reduced_motion: true,
                reduced_transparency: true,
                high_contrast: true,
                ..Default::default()
            }))
            .request_redraw
    );
    source
        .render_retained_frame(&mut scene, &mut text, (1440, 1000), 1.)
        .unwrap();
    assert_ne!(
        subtitle(&source),
        before,
        "secondary text follows native contrast"
    );
    fn fullscreen_shaders(scene: &plico_renderer::Scene) -> usize {
        scene
            .commands
            .iter()
            .filter(|command| {
                matches!(command, plico_renderer::PaintCommand::Shader { effect, .. }
                    if effect.uniform_words()[0] >= 1000.)
            })
            .count()
    }
    assert_eq!(
        fullscreen_shaders(&scene),
        0,
        "the flat window canvas requires no fullscreen shader"
    );
    assert!(!source.has_anim());
}

#[test]
fn official_client_surfaces_ignore_removed_legacy_preferences_and_render() {
    fn backdrop_shaders(scene: &plico_renderer::Scene) -> usize {
        scene
            .commands
            .iter()
            .filter(|command| matches!(command, plico_renderer::PaintCommand::Shader { effect, .. } if effect.reads_backdrop()))
            .count()
    }
    for screen in [
        "vault",
        "login",
        "settings",
        "editor",
        "folders",
        "transfer",
        "ssh-approval",
    ] {
        let mut app = plicowarden::PreviewConfig {
            screen: screen.into(),
            ..Default::default()
        }
        .app();
        app.appearance = serde_json::from_str(r#"{"liquidGlass":true,"animateReflections":true,"frost":0.1,"tint":0.2,"uiScale":1.25}"#).unwrap();
        assert_eq!(app.appearance.ui_scale, 1.25);
        let mut source = Source::with_clock(NativeApp(app), plico::VirtualClock::default());
        let mut text = TextContext::new();
        let mut scene = plico_renderer::Scene::new();
        source
            .render_retained_frame(&mut scene, &mut text, (1440, 1000), 1.)
            .unwrap();
        assert_eq!(
            backdrop_shaders(&scene),
            0,
            "{screen}: app surfaces must stay opaque"
        );
    }
}

#[test]
fn normal_linux_windows_use_native_buttons_while_preview_controls_remain_explicit() {
    assert_eq!(
        Warden::default().custom_titlebar,
        !cfg!(target_os = "linux")
    );
    assert!(plicowarden::PreviewConfig::default().app().custom_titlebar);
}
