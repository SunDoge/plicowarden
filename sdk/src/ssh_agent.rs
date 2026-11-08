//! Native SSH agent boundary. Only public metadata reaches UI; secrets stay in
//! the vault actor. No private-key cache, serde or debug logging in this module.
use signature::{RandomizedSigner, SignatureEncoding, Signer};
pub use ssh_agent_lib::ssh_key;
use ssh_agent_lib::{
    proto::{
        Identity, PublicCredential, SignRequest,
        message::{Request, Response},
    },
    ssh_encoding::{Decode, Encode},
};
use ssh_key::{Algorithm, HashAlg, PrivateKey, PublicKey, Signature};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{mpsc, oneshot, watch},
    task::JoinHandle,
};
use zeroize::Zeroizing;

pub const APPROVAL_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_PACKET: usize = 256 * 1024;
const MAX_PENDING: usize = 16;
#[derive(Clone)]
pub struct KeyIdentity {
    pub id: String,
    pub name: String,
    pub public: PublicKey,
}
impl KeyIdentity {
    pub fn wire(&self) -> Identity {
        Identity {
            credential: self.public.key_data().clone().into(),
            comment: self.name.clone(),
        }
    }
}
/// Public request metadata only. Not serialized into application reload state.
#[derive(Clone)]
pub struct Approval {
    pub id: u64,
    pub name: String,
    pub fingerprint: String,
    pub process_id: Option<u32>,
}
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Access {
    pub epoch: u64,
    pub unlocked: bool,
}
#[derive(Clone)]
pub struct Gate(watch::Sender<Access>);
impl Gate {
    pub fn new(epoch: u64) -> Self {
        Self(
            watch::channel(Access {
                epoch,
                unlocked: false,
            })
            .0,
        )
    }
    pub fn set(&self, epoch: u64, unlocked: bool) {
        self.0.send_if_modified(|state| {
            if epoch < state.epoch {
                return false;
            }
            let next = Access { epoch, unlocked };
            if *state == next {
                false
            } else {
                *state = next;
                true
            }
        });
    }
    /// Revoke in-flight calls synchronously from the host thread.
    pub fn revoke(&self) {
        self.0.send_modify(|state| {
            state.epoch = state.epoch.saturating_add(1);
            state.unlocked = false;
        });
    }
    pub fn access(&self) -> Access {
        *self.0.borrow()
    }
}
pub enum Operation {
    List,
    Sign(SignRequest),
}
pub enum Answer {
    Identities(Vec<Identity>),
    Signature(Signature),
}
pub struct Call {
    pub epoch: u64,
    pub process_id: Option<u32>,
    pub operation: Operation,
    pub reply: oneshot::Sender<Result<Answer, ()>>,
}
#[derive(Clone)]
pub struct Endpoint {
    tx: mpsc::Sender<Call>,
    gate: Gate,
}
impl Endpoint {
    pub fn channel(gate: Gate) -> (Self, mpsc::Receiver<Call>) {
        let (tx, rx) = mpsc::channel(MAX_PENDING);
        (Self { tx, gate }, rx)
    }
    async fn request(&self, operation: Operation, process_id: Option<u32>) -> Result<Answer, ()> {
        let mut access = self.gate.0.subscribe();
        let state = *access.borrow_and_update();
        if !state.unlocked {
            return Err(());
        }
        let (reply, result) = oneshot::channel();
        let call = Call {
            epoch: state.epoch,
            process_id,
            operation,
            reply,
        };
        tokio::time::timeout(APPROVAL_TIMEOUT, async {
            tokio::select! {
                biased;
                _ = access.changed() => Err(()),
                answer = async { self.tx.try_send(call).map_err(|_| ())?; result.await.map_err(|_| ())? } => {
                    if self.gate.access() == state { answer } else { Err(()) }
                }
            }
        }).await.map_err(|_| ())?
    }
    #[cfg(unix)]
    async fn serve(&self, mut stream: tokio::net::UnixStream, process_id: Option<u32>) {
        loop {
            let packet = tokio::time::timeout(Duration::from_secs(60), async {
                let len = stream.read_u32().await.ok()? as usize;
                if !(1..=MAX_PACKET).contains(&len) {
                    return None;
                }
                let mut bytes = Zeroizing::new(vec![0; len]);
                stream.read_exact(&mut bytes).await.ok()?;
                Some(bytes)
            })
            .await;
            let Ok(Some(bytes)) = packet else {
                break;
            };
            // Never decode unsupported key-import packets or log raw protocol data.
            let mut reader = &bytes[..];
            let request = if matches!(bytes.first(), Some(11 | 13)) {
                Request::decode(&mut reader).ok()
            } else {
                None
            };
            let operation = match request {
                Some(Request::RequestIdentities) if reader.is_empty() => Some(Operation::List),
                Some(Request::SignRequest(request)) if reader.is_empty() => {
                    Some(Operation::Sign(request))
                }
                _ => None,
            };
            let response = match operation {
                Some(operation) => match self.request(operation, process_id).await {
                    Ok(Answer::Identities(ids)) => Response::IdentitiesAnswer(ids),
                    Ok(Answer::Signature(signature)) => Response::SignResponse(signature),
                    Err(()) => Response::Failure,
                },
                None => Response::Failure,
            };
            let mut data = Vec::new();
            if response.encode(&mut data).is_err() {
                break;
            }
            if stream.write_u32(data.len() as u32).await.is_err()
                || stream.write_all(&data).await.is_err()
            {
                break;
            }
        }
    }
}
/// Validate algorithm flags before requesting user approval. SHA-1 is unsupported.
pub fn validate_sign(public: &PublicKey, request: &SignRequest) -> Result<(), String> {
    if request.credential != PublicCredential::from(public.key_data().clone()) {
        return Err("SSH public key no longer matches the vault item.".into());
    }
    match (public.algorithm(), request.flags) {
        (Algorithm::Rsa { .. }, 2 | 4) => Ok(()),
        (Algorithm::Ed25519 | Algorithm::Ecdsa { .. }, 0) => Ok(()),
        _ => Err("Unsupported SSH signature algorithm or flags.".into()),
    }
}
pub fn sign_private(key: &PrivateKey, request: &SignRequest) -> Result<Signature, String> {
    validate_sign(key.public_key(), request)?;
    if let ssh_key::private::KeypairData::Rsa(rsa) = key.key_data() {
        // ssh-key 0.6.7's RsaKeypair conversion passes p twice. Construct
        // validated material explicitly with p and q, keeping rsa's validation.
        let material = rsa::RsaPrivateKey::from_components(
            rsa::BigUint::try_from(&rsa.public.n).map_err(|_| "Invalid RSA key.")?,
            rsa::BigUint::try_from(&rsa.public.e).map_err(|_| "Invalid RSA key.")?,
            rsa::BigUint::try_from(&rsa.private.d).map_err(|_| "Invalid RSA key.")?,
            vec![
                rsa::BigUint::try_from(&rsa.private.p).map_err(|_| "Invalid RSA key.")?,
                rsa::BigUint::try_from(&rsa.private.q).map_err(|_| "Invalid RSA key.")?,
            ],
        )
        .map_err(|_| "Invalid RSA key.")?;
        if rsa::traits::PublicKeyParts::n(&material).bits() < 2048 {
            return Err("RSA keys must be at least 2048 bits.".into());
        }
        let (hash, data) = if request.flags == 2 {
            let signer = rsa::pkcs1v15::SigningKey::<sha2::Sha256>::new(material);
            (
                HashAlg::Sha256,
                signer
                    .try_sign_with_rng(&mut rand::rngs::OsRng, &request.data)
                    .map_err(|_| "SSH signing failed.")?
                    .to_vec(),
            )
        } else {
            let signer = rsa::pkcs1v15::SigningKey::<sha2::Sha512>::new(material);
            (
                HashAlg::Sha512,
                signer
                    .try_sign_with_rng(&mut rand::rngs::OsRng, &request.data)
                    .map_err(|_| "SSH signing failed.")?
                    .to_vec(),
            )
        };
        Signature::new(Algorithm::Rsa { hash: Some(hash) }, data)
            .map_err(|_| "SSH signing failed.".into())
    } else {
        key.try_sign(&request.data)
            .map_err(|_| "SSH signing failed.".into())
    }
}

pub struct RunningAgent {
    pub path: PathBuf,
    task: JoinHandle<()>,
    #[cfg(unix)]
    inode: u64,
}
impl RunningAgent {
    pub fn default_path() -> Result<PathBuf, String> {
        if let Some(path) = std::env::var_os("PLICOWARDEN_SSH_AUTH_SOCK") {
            let path = PathBuf::from(path);
            if !path.is_absolute() {
                return Err("PLICOWARDEN_SSH_AUTH_SOCK must be an absolute path.".into());
            }
            return Ok(path);
        }
        let dirs = directories::ProjectDirs::from("dev", "Plico", "Plicowarden")
            .ok_or("Cannot find an agent directory.")?;
        Ok(dirs
            .runtime_dir()
            .unwrap_or(dirs.data_local_dir())
            .join("ssh-agent")
            .join("agent.sock"))
    }
    #[cfg(unix)]
    pub fn bind(path: &Path, endpoint: Endpoint) -> Result<Self, String> {
        use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
        if !path.is_absolute() {
            return Err("SSH socket path must be absolute.".into());
        }
        let directory = path.parent().ok_or("Invalid SSH socket path.")?;
        std::fs::DirBuilder::new()
            .recursive(true)
            .mode(0o700)
            .create(directory)
            .map_err(|_| "Could not create the SSH agent directory.")?;
        let metadata = std::fs::symlink_metadata(directory)
            .map_err(|_| "Cannot inspect the agent directory.")?;
        if !metadata.is_dir()
            || metadata.uid() != rustix::process::geteuid().as_raw()
            || metadata.mode() & 0o077 != 0
        {
            return Err("SSH agent directory must be owned by you with permissions 0700.".into());
        }
        // bind fails rather than replacing another agent's socket.
        let listener = std::os::unix::net::UnixListener::bind(path)
            .map_err(|_| "Could not bind SSH socket. The path may already be in use.")?;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
            .map_err(|_| "Cannot protect the SSH socket.")?;
        let inode = std::fs::symlink_metadata(path)
            .map_err(|_| "Cannot inspect the SSH socket.")?
            .ino();
        listener
            .set_nonblocking(true)
            .map_err(|_| "Cannot initialize SSH socket.")?;
        let listener = tokio::net::UnixListener::from_std(listener)
            .map_err(|_| "Cannot initialize SSH socket.")?;
        let task = tokio::spawn(async move {
            let mut sessions = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    connection = listener.accept() => {
                        let Ok((stream, _)) = connection else { break; };
                        let Ok(credentials) = stream.peer_cred() else { continue; };
                        if credentials.uid() != rustix::process::geteuid().as_raw() || sessions.len() >= MAX_PENDING { continue; }
                        let endpoint = endpoint.clone();
                        let pid = credentials.pid().and_then(|pid| u32::try_from(pid).ok());
                        sessions.spawn(async move { endpoint.serve(stream, pid).await; });
                    }
                    _ = sessions.join_next(), if !sessions.is_empty() => {}
                }
            }
        });
        Ok(Self {
            path: path.into(),
            task,
            inode,
        })
    }
    #[cfg(not(unix))]
    pub fn bind(_path: &Path, _endpoint: Endpoint) -> Result<Self, String> {
        Err("SSH agent is currently supported on Unix platforms only.".into())
    }
}
impl Drop for RunningAgent {
    fn drop(&mut self) {
        self.task.abort();
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            if std::fs::symlink_metadata(&self.path)
                .is_ok_and(|metadata| metadata.ino() == self.inode)
            {
                let _ = std::fs::remove_file(&self.path);
            }
        }
    }
}

#[async_trait::async_trait]
pub trait VaultKeys: Send + Sync {
    async fn identities(&self) -> Result<Vec<KeyIdentity>, String>;
    async fn sign(&self, id: &str, request: &SignRequest) -> Result<Signature, String>;
}
struct Pending {
    call: Call,
    key: String,
    deadline: tokio::time::Instant,
}
pub struct Broker {
    pub endpoint: Endpoint,
    pub incoming: mpsc::Receiver<Call>,
    gate: Gate,
    pending: std::collections::HashMap<u64, Pending>,
    next: u64,
}
impl Broker {
    pub fn new(gate: Gate) -> Self {
        let (endpoint, incoming) = Endpoint::channel(gate.clone());
        Self {
            endpoint,
            incoming,
            gate,
            pending: Default::default(),
            next: 0,
        }
    }
    pub async fn receive(&mut self, call: Call, vault: &impl VaultKeys) -> Option<Approval> {
        let access = self.gate.access();
        if !access.unlocked || access.epoch != call.epoch || call.reply.is_closed() {
            let _ = call.reply.send(Err(()));
            return None;
        }
        let Ok(keys) = vault.identities().await else {
            let _ = call.reply.send(Err(()));
            return None;
        };
        if self.gate.access() != access || call.reply.is_closed() {
            let _ = call.reply.send(Err(()));
            return None;
        }
        match &call.operation {
            Operation::List => {
                let _ = call.reply.send(Ok(Answer::Identities(
                    keys.iter().take(1024).map(KeyIdentity::wire).collect(),
                )));
                None
            }
            Operation::Sign(request) => {
                let Some(key) = keys
                    .into_iter()
                    .find(|key| key.wire().credential == request.credential)
                else {
                    let _ = call.reply.send(Err(()));
                    return None;
                };
                if self.pending.len() >= MAX_PENDING || validate_sign(&key.public, request).is_err()
                {
                    let _ = call.reply.send(Err(()));
                    return None;
                }
                self.next += 1;
                let approval = Approval {
                    id: self.next,
                    name: key.name,
                    fingerprint: key.public.fingerprint(HashAlg::Sha256).to_string(),
                    process_id: call.process_id,
                };
                self.pending.insert(
                    approval.id,
                    Pending {
                        call,
                        key: key.id,
                        deadline: tokio::time::Instant::now() + APPROVAL_TIMEOUT,
                    },
                );
                Some(approval)
            }
        }
    }
    pub async fn decide(&mut self, id: u64, allow: bool, vault: &impl VaultKeys) {
        let Some(pending) = self.pending.remove(&id) else {
            return;
        };
        let state = self.gate.access();
        let answer = if allow
            && state.unlocked
            && state.epoch == pending.call.epoch
            && tokio::time::Instant::now() < pending.deadline
            && !pending.call.reply.is_closed()
        {
            match &pending.call.operation {
                Operation::Sign(request) => vault
                    .sign(&pending.key, request)
                    .await
                    .map(Answer::Signature)
                    .map_err(|_| ()),
                _ => Err(()),
            }
        } else {
            Err(())
        };
        let answer = if self.gate.access() == state {
            answer
        } else {
            Err(())
        };
        let _ = pending.call.reply.send(answer);
    }
    pub fn expire(&mut self) -> Vec<u64> {
        let state = self.gate.access();
        let now = tokio::time::Instant::now();
        let mut closed = Vec::new();
        self.pending.retain(|id, pending| {
            let keep = state.unlocked
                && state.epoch == pending.call.epoch
                && now < pending.deadline
                && !pending.call.reply.is_closed();
            if !keep {
                closed.push(*id);
            }
            keep
        });
        closed
    }
    pub fn clear(&mut self) -> Vec<u64> {
        let ids = self.pending.keys().copied().collect();
        self.pending.clear();
        ids
    }
}
