//! Experimental recipe runner: one build, independent cases, optional shared GPU.
use plico_desktop::{NativeApp, PreviewScenario, PreviewSession, RenderBackend};
use plico_runtime::testing::Driver;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
use plicowarden::PreviewConfig;

#[derive(usage_rs::Cli)]
#[usage(about = "Run synthetic Warden recipes without SDK or preference access")]
struct Options {
    #[usage(long)]
    manifest: PathBuf,
    /// semantic, fresh, or shared
    #[usage(long, default = "shared")]
    mode: String,
    #[usage(long)]
    output: PathBuf,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    name: String,
    scenario: PathBuf,
    #[serde(default)]
    present: Vec<String>,
    #[serde(default)]
    absent: Vec<String>,
}
#[derive(Serialize)]
struct Outcome {
    name: String,
    passed: bool,
    milliseconds: f64,
    semantic_ms: f64,
    render_ms: f64,
    png_ms: f64,
    error: Option<String>,
}
#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("recipe I/O: {source}"))]
    Io { source: std::io::Error },
    #[snafu(display("recipe JSON: {source}"))]
    Json { source: serde_json::Error },
    #[snafu(display("scenario: {source}"))]
    Preview { source: plico_desktop::PreviewError },
    #[snafu(display("input: {source}"))]
    Input {
        source: plico_runtime::testing::Error,
    },
    #[snafu(display("step {index}: {source}"))]
    Step {
        index: usize,
        source: plico_runtime::testing::Error,
    },
    #[snafu(display("PNG: {source}"))]
    Capture { source: plico_desktop::CaptureError },
    #[snafu(display("{message}"))]
    Assertion { message: String },
}
fn check(case: &Case) -> Result<(), Error> {
    let recipe = PreviewScenario::<PreviewConfig>::load(&case.scenario).context(PreviewSnafu)?;
    if recipe.version != 1 || recipe.viewport.scale != 1.0 {
        return Err(Error::Assertion {
            message: "semantic probe supports version 1 / scale 1 only".into(),
        });
    }
    let mut driver = Driver::new(
        NativeApp(recipe.state.app()),
        recipe.viewport.width,
        recipe.viewport.height,
    )
    .context(InputSnafu)?;
    for (index, step) in recipe.steps.into_iter().enumerate() {
        driver
            .advance(Duration::from_millis(step.after_ms))
            .context(StepSnafu { index: index + 1 })?;
        driver
            .execute(step.command)
            .context(StepSnafu { index: index + 1 })?;
    }
    driver
        .advance(Duration::from_millis(recipe.time_ms))
        .context(InputSnafu)?;
    let snapshot = driver.snapshot();
    let nodes = snapshot.exposed_nodes();
    for (expected, labels) in [(true, &case.present), (false, &case.absent)] {
        for label in labels {
            let found = nodes.iter().any(|node| node.label.as_ref() == Some(label));
            if found != expected {
                return Err(Error::Assertion {
                    message: format!("label {label:?}: expected present={expected}, got {found}"),
                });
            }
        }
    }
    if driver
        .source
        .adapter
        .placed
        .iter()
        .any(|node| !node.rect.iter().all(|n| n.is_finite()))
    {
        return Err(Error::Assertion {
            message: "non-finite layout bounds".into(),
        });
    }
    Ok(())
}
fn run(options: &Options, case: &Case, session: &mut PreviewSession) -> Outcome {
    let start = Instant::now();
    let mut outcome = Outcome {
        name: case.name.clone(),
        passed: false,
        milliseconds: 0.,
        semantic_ms: 0.,
        render_ms: 0.,
        png_ms: 0.,
        error: None,
    };
    let result = (|| {
        let png = options.output.join(format!("{}.png", case.name));
        if options.mode != "semantic" && png.exists() {
            std::fs::remove_file(&png).context(IoSnafu)?;
        }
        let start = Instant::now();
        check(case)?;
        outcome.semantic_ms = start.elapsed().as_secs_f64() * 1000.;
        if options.mode != "semantic" {
            let recipe =
                PreviewScenario::<PreviewConfig>::load(&case.scenario).context(PreviewSnafu)?;
            let start = Instant::now();
            let frame = if options.mode == "shared" {
                recipe.render_in(session, |config| NativeApp(config.app()))
            } else {
                recipe.render(|config| NativeApp(config.app()))
            }
            .context(PreviewSnafu)?;
            outcome.render_ms = start.elapsed().as_secs_f64() * 1000.;
            let start = Instant::now();
            frame.write_png(png).context(CaptureSnafu)?;
            outcome.png_ms = start.elapsed().as_secs_f64() * 1000.;
        }
        Ok::<_, Error>(())
    })();
    outcome.milliseconds = start.elapsed().as_secs_f64() * 1000.;
    outcome.passed = result.is_ok();
    outcome.error = result.err().map(|e| e.to_string());
    outcome
}
fn main() -> Result<(), Error> {
    let options = Options::parse();
    if !["semantic", "fresh", "shared"].contains(&options.mode.as_str()) {
        return Err(Error::Assertion {
            message: "mode must be semantic, fresh or shared".into(),
        });
    }
    let cases: Vec<Case> =
        serde_json::from_slice(&std::fs::read(&options.manifest).context(IoSnafu)?)
            .context(JsonSnafu)?;
    let mut names = std::collections::HashSet::new();
    for case in &cases {
        if case.name.is_empty()
            || !case
                .name
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            || !names.insert(&case.name)
        {
            return Err(Error::Assertion {
                message: format!("unsafe or duplicate case name: {:?}", case.name),
            });
        }
    }
    std::fs::create_dir_all(&options.output).context(IoSnafu)?;
    let mut session = PreviewSession::new(RenderBackend::Wgpu);
    let outcomes: Vec<_> = cases
        .iter()
        .map(|case| run(&options, case, &mut session))
        .collect();
    println!("{}", serde_json::to_string(&outcomes).context(JsonSnafu)?);
    if outcomes.iter().any(|outcome| !outcome.passed) {
        std::process::exit(1);
    }
    Ok(())
}
