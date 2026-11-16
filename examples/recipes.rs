//! Typed TS-generated plans; only synthetic app state and service events.
use plico_desktop::{
    NativeApp, PreviewSession, RenderBackend,
    recipe::{RecipeError, RecipeMode, RecipePlan},
};
use plicowarden::{PreviewConfig, Warden, model::HostCommand};
use serde::Deserialize;
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

#[derive(usage_rs::Cli)]
#[usage(about = "Execute TS-generated Warden recipes (synthetic fixtures only)")]
struct Options {
    #[usage(long)]
    plan: PathBuf,
    #[usage(long)]
    output: PathBuf,
    #[usage(long)]
    semantic: bool,
}
#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("recipe I/O: {source}"))]
    Io { source: std::io::Error },
    #[snafu(display("recipe JSON: {source}"))]
    Json { source: serde_json::Error },
    #[snafu(display("recipe plan: {source}"))]
    Recipe { source: RecipeError },
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum FixtureEvent {
    AccountError { message: String },
    TwoFactorRequired,
    Locked,
}
fn inject(view: &mut NativeApp<Warden>, value: &serde_json::Value) -> Result<(), String> {
    let event: FixtureEvent =
        serde_json::from_value(value.clone()).map_err(|error| error.to_string())?;
    match event {
        FixtureEvent::AccountError { message } => {
            let index = view
                .0
                .commands
                .iter()
                .position(|command| matches!(command, HostCommand::Account { ticket, .. } if Some(*ticket) == view.0.pending))
                .ok_or("account_error requires a pending account request")?;
            if let HostCommand::Account { ticket, .. } = view.0.commands.remove(index) {
                view.0.apply(plicowarden_sdk::Event {
                    ticket,
                    result: Err(message),
                });
            }
        }
        FixtureEvent::TwoFactorRequired => {
            use plicowarden_sdk::{
                Reply, Request,
                service::{LoginOutcome, TwoFactorOption, TwoFactorProviderId},
            };
            let index = view.0.commands.iter().position(|command| matches!(command,
                HostCommand::Account { ticket, request } if Some(*ticket) == view.0.pending && matches!(request.as_ref(), Request::Login { .. })))
                .ok_or("two_factor_required requires a pending login request")?;
            if let HostCommand::Account { ticket, .. } = view.0.commands.remove(index) {
                view.0.apply(plicowarden_sdk::Event {
                    ticket,
                    result: Ok(Reply::Login(LoginOutcome::TwoFactorRequired {
                        providers: vec![
                            TwoFactorOption {
                                id: TwoFactorProviderId::Authenticator,
                                label: "Authenticator app",
                                hint: Some("Enter the code from your authenticator app.".into()),
                                supported: true,
                            },
                            TwoFactorOption {
                                id: TwoFactorProviderId::Email,
                                label: "Email",
                                hint: Some("A code will be sent to r***@example.invalid.".into()),
                                supported: true,
                            },
                            TwoFactorOption {
                                id: TwoFactorProviderId::Webauthn,
                                label: "Security key",
                                hint: None,
                                supported: false,
                            },
                        ],
                    })),
                });
            }
        }
        FixtureEvent::Locked => view.0.apply(plicowarden_sdk::Event {
            ticket: 0,
            result: Ok(plicowarden_sdk::Reply::Locked),
        }),
    }
    Ok(())
}
fn main() -> Result<(), Error> {
    let options = Options::parse();
    let bytes = std::fs::read(&options.plan).context(IoSnafu)?;
    let plan: RecipePlan<PreviewConfig> = serde_json::from_slice(&bytes).context(JsonSnafu)?;
    // Validate before touching output; never construct the real account/persistence host.
    plan.validate().context(RecipeSnafu)?;
    let mut session = PreviewSession::new(RenderBackend::Wgpu);
    let results = plan
        .run(
            &mut session,
            if options.semantic {
                RecipeMode::Semantic
            } else {
                RecipeMode::Visual
            },
            &options.output,
            |config| NativeApp(config.app()),
            inject,
        )
        .context(RecipeSnafu)?;
    std::fs::write(options.output.join("plan.json"), bytes).context(IoSnafu)?;
    let report = serde_json::json!({"version": 1, "mode": if options.semantic {"semantic"} else {"visual"}, "cases": results});
    let serialized = serde_json::to_string_pretty(&report).context(JsonSnafu)?;
    std::fs::write(options.output.join("results.json"), &serialized).context(IoSnafu)?;
    println!("{serialized}");
    if !results.iter().all(|result| result.passed) {
        std::process::exit(1);
    }
    Ok(())
}
