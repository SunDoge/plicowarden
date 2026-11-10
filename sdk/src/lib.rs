//! GPL-3.0-only Warden application service. SDK sessions live only in this actor.
mod clipboard;
pub mod protected;
pub mod security;
pub mod server;
pub mod service;
pub mod icons;
pub mod ssh_agent;
pub use bitwarden_vault::{FieldType, FieldView};

use service::{
    EditData, ItemDetails, ItemPatch, LoginOutcome, LoginRequest, Mutation, TotpCode,
    TwoFactorSubmitRequest, VaultService, VaultSnapshot,
};
use snafu::Snafu;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
use tokio::sync::{mpsc as async_mpsc, watch};
use zeroize::Zeroizing;

pub type Wake = Arc<dyn Fn() + Send + Sync>;
/// Deliberately not Debug/Serialize: requests can contain credentials.
pub enum Request {
    ConfigureSecurity(security::SecuritySettings),
    Login {
        config: LoginRequest,
        password: Zeroizing<String>,
    },
    TwoFactor(TwoFactorSubmitRequest),
    SendEmail,
    Sync,
    SshAgent(bool),
    SshDecision {
        id: u64,
        allow: bool,
    },
    Details(String),
    Reveal(String),
    PasswordHistory(String),
    Totp(String),
    CopyField {
        id: String,
        field: String,
    },
    CopyText(Zeroizing<String>),
    Edit(String),
    Save {
        id: Option<String>,
        patch: ItemPatch,
    },
    ExportVault {
        path: std::path::PathBuf,
        format: service::ExportKind,
        password: Zeroizing<String>,
    },
    ImportKdbx {
        path: std::path::PathBuf,
        password: Zeroizing<String>,
        key_file: Option<std::path::PathBuf>,
    },
    MoveToFolder {
        ids: Vec<String>,
        folder_id: Option<String>,
    },
    BatchAction {
        ids: Vec<String>,
        action: String,
    },
    Action {
        id: String,
        action: String,
    },
    Folder {
        id: Option<String>,
        name: String,
        delete: bool,
    },
}
pub enum Reply {
    SecurityConfigured(security::SecuritySettings),
    Login(LoginOutcome),
    LoginProgress(service::LoginStage),
    Snapshot(VaultSnapshot),
    Details(Box<ItemDetails>),
    Revealed(Zeroizing<String>),
    PasswordHistory {
        id: String,
        entries: Vec<(i64, Zeroizing<String>)>,
    },
    Totp(TotpCode),
    Edit(EditData),
    Mutation(Mutation),
    BatchMutation(service::BatchMutation),
    Transfer(service::TransferResult),
    Copied,
    EmailSent,
    Locked,
    SshStatus(Option<String>),
    SshApproval(ssh_agent::Approval),
    SshClosed(u64),
    TotpExpired(i64),
}
pub struct Event {
    pub ticket: u64,
    pub result: Result<Reply, String>,
}
#[derive(Debug, Snafu)]
pub enum SendError {
    #[snafu(display("The account worker is busy or has stopped. Try connecting again."))]
    Unavailable,
}
struct Work {
    ticket: u64,
    request: Request,
}
pub struct Client {
    tx: async_mpsc::Sender<Work>,
    rx: mpsc::Receiver<Event>,
    close: watch::Sender<bool>,
    wake: Arc<Mutex<Option<Wake>>>,
    agent_gate: ssh_agent::Gate,
    activity: Arc<std::sync::atomic::AtomicBool>,
}
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}
impl Client {
    pub fn new() -> Self {
        Self::with_security(Default::default())
    }
    pub fn with_security(settings: security::SecuritySettings) -> Self {
        let activity = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let worker_activity = activity.clone();
        let (tx, mut incoming) = async_mpsc::channel::<Work>(8);
        let (outgoing, rx) = mpsc::channel();
        let (close, mut closed) = watch::channel(false);
        let wake: Arc<Mutex<Option<Wake>>> = Arc::new(Mutex::new(None));
        let worker_wake = Arc::clone(&wake);
        let agent_gate = ssh_agent::Gate::new(1);
        let access = agent_gate.clone();
        std::thread::spawn(move || {
            let notify = |event| {
                let _ = outgoing.send(event);
                let callback = worker_wake.lock().ok().and_then(|w| w.clone());
                if let Some(callback) = callback {
                    callback();
                }
            };
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(_) => {
                    notify(Event {
                        ticket: 0,
                        result: Err("Could not start account worker.".into()),
                    });
                    return;
                }
            };
            runtime.block_on(async {
                let mut service = VaultService::default();
                service.configure_security(settings);
                let mut broker = ssh_agent::Broker::new(access.clone());
                let mut agent: Option<ssh_agent::RunningAgent> = None;
                let mut interval = tokio::time::interval(Duration::from_secs(1));
                let mut code_expiry: Option<i64> = None;
                loop {
                    observe_activity(&mut service, &worker_activity, &access, &mut agent, &mut broker, &mut code_expiry, &notify);
                    let work = tokio::select! { biased;
                        _ = closed.changed() => break,
                        _ = interval.tick() => {
                            observe_activity(&mut service, &worker_activity, &access, &mut agent, &mut broker, &mut code_expiry, &notify);
                            if service.lock_if_idle() {
                                stop_agent(&access, &mut agent, &mut broker, &notify);
                                code_expiry=None; notify(Event { ticket: 0, result: Ok(Reply::Locked) });
                            }
                            for id in broker.expire() { notify(Event { ticket: 0, result: Ok(Reply::SshClosed(id)) }); }
                            let now=std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d|d.as_secs() as i64).unwrap_or(0);
                            if let Some(expiry)=code_expiry.filter(|expiry|*expiry<=now) {
                                code_expiry=None;notify(Event {ticket:0,result:Ok(Reply::TotpExpired(expiry))});
                            }
                            continue;
                        },
                        call = broker.incoming.recv() => {
                            if let Some(call) = call {
                                if service.lock_if_idle() {
                                    stop_agent(&access, &mut agent, &mut broker, &notify);
                                    notify(Event { ticket: 0, result: Ok(Reply::Locked) });
                                }
                                let approval = tokio::select! { biased;
                                    _ = closed.changed() => break,
                                    approval = broker.receive(call, &service) => approval,
                                };
                                if let Some(approval) = approval { notify(Event { ticket: 0, result: Ok(Reply::SshApproval(approval)) }); }
                            }
                            continue;
                        },
                        work = incoming.recv() => match work { Some(work) => work, None => break },
                    };
                    if matches!(&work.request, Request::Login { .. } | Request::TwoFactor(_)) {
                        stop_agent(&access, &mut agent, &mut broker, &notify);
                    }
                    observe_activity(&mut service, &worker_activity, &access, &mut agent, &mut broker, &mut code_expiry, &notify);
                    if service.lock_if_idle() {
                        stop_agent(&access, &mut agent, &mut broker, &notify);
                        notify(Event { ticket: 0, result: Ok(Reply::Locked) });
                    }
                    let operation = async {
                        match work.request {
                            Request::SshAgent(enabled) => {
                                if !enabled {
                                    stop_agent(&access, &mut agent, &mut broker, &notify);
                                    Ok(Reply::SshStatus(None))
                                } else if service.is_locked() {
                                    Err("Unlock your vault before enabling SSH agent.".into())
                                } else {
                                    if agent.is_none() {
                                        let path = ssh_agent::RunningAgent::default_path()?;
                                        agent = Some(ssh_agent::RunningAgent::bind(&path, broker.endpoint.clone())?);
                                    }
                                    let epoch = access.access().epoch.saturating_add(1);
                                    if epoch == u64::MAX { return Err("SSH agent request cancelled.".into()); }
                                    access.set(epoch, true);
                                    Ok(Reply::SshStatus(agent.as_ref().map(|a| a.path.to_string_lossy().into_owned())))
                                }
                            }
                            Request::SshDecision { id, allow } => {
                                broker.decide(id, allow, &service).await;
                                Ok(Reply::SshClosed(id))
                            }
                            Request::Login { config, password } => {
                                if config.email.trim().is_empty() || password.is_empty() {
                                    Err("Enter your email and master password.".into())
                                } else {
                                    service.login_with_progress(config, password, |stage| {
                                        notify(Event { ticket: work.ticket, result: Ok(Reply::LoginProgress(stage)) });
                                    }).await.map(Reply::Login)
                                }
                            }
                            request => execute(&mut service, request).await,
                        }
                    };
                    let result = tokio::select! { biased;
                        _ = closed.changed() => break,
                        result = tokio::time::timeout(Duration::from_secs(90), operation) => result,
                    };
                    match result {
                        Ok(result) => {
                            if let Ok(Reply::Totp(code))=&result {code_expiry=Some(code.expires_at);}
                            notify(Event { ticket: work.ticket, result });
                        },
                        Err(_) => {
                            service.lock();
                            stop_agent(&access, &mut agent, &mut broker, &notify);
                            notify(Event { ticket: work.ticket, result: Err("The request timed out. Connect to your account again.".into()) });
                            notify(Event { ticket: 0, result: Ok(Reply::Locked) });
                        }
                    }
                }
                stop_agent(&access, &mut agent, &mut broker, &notify);
                service.lock();
            });
        });
        Self {
            tx,
            rx,
            close,
            wake,
            agent_gate,
            activity,
        }
    }
    pub fn note_activity(&self) {
        self.activity
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
    pub fn send(&self, ticket: u64, request: Request) -> Result<(), SendError> {
        if matches!(&request, Request::SshAgent(false)) {
            self.agent_gate.revoke();
        }
        self.tx
            .try_send(Work { ticket, request })
            .map_err(|_| SendError::Unavailable)
    }
    pub fn try_recv(&self) -> Option<Event> {
        self.rx.try_recv().ok()
    }
    pub fn set_wake(&self, wake: Wake) {
        if let Ok(mut slot) = self.wake.lock() {
            *slot = Some(wake);
        }
    }
}
impl Drop for Client {
    fn drop(&mut self) {
        self.agent_gate.set(u64::MAX, false);
        let _ = self.close.send(true);
    }
}
async fn execute(service: &mut VaultService, request: Request) -> Result<Reply, String> {
    Ok(match request {
        Request::ConfigureSecurity(settings) => {
            let settings = settings.validated();
            service.configure_security(settings);
            Reply::SecurityConfigured(settings)
        }
        Request::Login { config, password } => {
            if config.email.trim().is_empty() || password.is_empty() {
                return Err("Enter your email and master password.".into());
            }
            Reply::Login(service.login(config, password).await?)
        }
        Request::TwoFactor(input) => Reply::Login(service.submit_two_factor(input).await?),
        Request::SendEmail => {
            service.send_two_factor_email().await?;
            Reply::EmailSent
        }
        Request::SshAgent(_) | Request::SshDecision { .. } => {
            return Err("SSH command must be handled by the account actor.".into());
        }
        Request::Sync => Reply::Snapshot(service.refresh().await?),
        Request::Details(id) => Reply::Details(Box::new(service.details(&id).await?)),
        Request::PasswordHistory(id) => Reply::PasswordHistory {
            entries: service.password_history(&id).await?,
            id,
        },
        Request::Reveal(id) => Reply::Revealed(service.reveal_password(&id).await?),
        Request::Totp(id) => Reply::Totp(service.totp_code(&id).await?),
        Request::CopyField { id, field } => {
            service.copy_field(&id, &field).await?;
            Reply::Copied
        }
        Request::CopyText(text) => {
            service.copy_text(text).await?;
            Reply::Copied
        }
        Request::Edit(id) => Reply::Edit(service.edit_data(&id).await?),
        Request::Save { id, patch } => {
            Reply::Mutation(service.save_item(id.as_deref(), patch).await?)
        }
        Request::ExportVault {
            path,
            format,
            password,
        } => Reply::Transfer(service.export_file(&path, format, password).await?),
        Request::ImportKdbx {
            path,
            password,
            key_file,
        } => Reply::Transfer(
            service
                .import_kdbx_file(&path, password, key_file.as_deref())
                .await?,
        ),
        Request::MoveToFolder { ids, folder_id } => {
            Reply::BatchMutation(service.move_to_folder(ids, folder_id.as_deref()).await?)
        }
        Request::BatchAction { ids, action } => {
            Reply::BatchMutation(service.batch_action(ids, &action).await?)
        }
        Request::Action { id, action } => Reply::Mutation(service.item_action(&id, &action).await?),
        Request::Folder { id, name, delete } => {
            Reply::Mutation(service.folder_action(id.as_deref(), &name, delete).await?)
        }
    })
}

fn observe_activity(
    service: &mut VaultService,
    activity: &std::sync::atomic::AtomicBool,
    gate: &ssh_agent::Gate,
    agent: &mut Option<ssh_agent::RunningAgent>,
    broker: &mut ssh_agent::Broker,
    code_expiry: &mut Option<i64>,
    notify: &impl Fn(Event),
) {
    if activity.swap(false, std::sync::atomic::Ordering::Relaxed) && service.note_activity() {
        stop_agent(gate, agent, broker, notify);
        *code_expiry = None;
        notify(Event {
            ticket: 0,
            result: Ok(Reply::Locked),
        });
    }
}

fn stop_agent(
    gate: &ssh_agent::Gate,
    agent: &mut Option<ssh_agent::RunningAgent>,
    broker: &mut ssh_agent::Broker,
    notify: &impl Fn(Event),
) {
    gate.revoke();
    let active = agent.take().is_some();
    for id in broker.clear() {
        notify(Event {
            ticket: 0,
            result: Ok(Reply::SshClosed(id)),
        });
    }
    if active {
        notify(Event {
            ticket: 0,
            result: Ok(Reply::SshStatus(None)),
        });
    }
}
