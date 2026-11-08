#![cfg(unix)]
use plicowarden_sdk::ssh_agent::{
    self, Broker, Gate, KeyIdentity, RunningAgent, VaultKeys,
    ssh_key::{Algorithm, HashAlg, PrivateKey, Signature},
};
use signature::Verifier;
use ssh_agent_lib::{agent::Session, client::Client, proto::SignRequest};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

struct SyntheticVault {
    key: PrivateKey,
    signs: AtomicUsize,
}
impl SyntheticVault {
    fn new() -> Self {
        Self {
            key: PrivateKey::random(&mut rand::rngs::OsRng, Algorithm::Ed25519).unwrap(),
            signs: AtomicUsize::new(0),
        }
    }
    fn request(&self) -> SignRequest {
        SignRequest {
            credential: self.key.public_key().key_data().clone().into(),
            data: b"synthetic signing payload".to_vec(),
            flags: 0,
        }
    }
}
#[async_trait::async_trait]
impl VaultKeys for SyntheticVault {
    async fn identities(&self) -> Result<Vec<KeyIdentity>, String> {
        Ok(vec![KeyIdentity {
            id: "synthetic-id".into(),
            name: "Synthetic SSH key".into(),
            public: self.key.public_key().clone(),
        }])
    }
    async fn sign(&self, id: &str, request: &SignRequest) -> Result<Signature, String> {
        assert_eq!(id, "synthetic-id");
        self.signs.fetch_add(1, Ordering::SeqCst);
        let pem = self
            .key
            .to_openssh(ssh_agent::ssh_key::LineEnding::LF)
            .map_err(|_| "Synthetic PEM export failed")?;
        let pem = plicowarden_sdk::protected::LockedSecret::from_string(pem)
            .map_err(|_| "Synthetic PEM protection failed")?;
        let key =
            PrivateKey::from_openssh(pem.as_bytes()).map_err(|_| "Synthetic PEM parse failed")?;
        drop(pem);
        ssh_agent::sign_private(&key, request)
    }
}
fn broker() -> (Gate, Broker) {
    let gate = Gate::new(1);
    gate.set(1, true);
    (gate.clone(), Broker::new(gate))
}
async fn client(agent: &RunningAgent) -> Client<tokio::net::UnixStream> {
    Client::new(tokio::net::UnixStream::connect(&agent.path).await.unwrap())
}
#[tokio::test]
async fn protocol_list_and_sign_require_explicit_approval() {
    let vault = SyntheticVault::new();
    let (_gate, mut broker) = broker();
    let directory = tempfile::tempdir().unwrap();
    let agent = RunningAgent::bind(
        &directory.path().join("private").join("agent.sock"),
        broker.endpoint.clone(),
    )
    .unwrap();
    let mut client = client(&agent).await;
    let listing = tokio::spawn(async move {
        let result = client.request_identities().await;
        (client, result)
    });
    let call = broker.incoming.recv().await.unwrap();
    assert!(broker.receive(call, &vault).await.is_none());
    let (mut client, list) = listing.await.unwrap();
    assert_eq!(
        list.unwrap()[0].credential.key_data(),
        vault.key.public_key().key_data()
    );
    let request = vault.request();
    let signing = tokio::spawn(async move { client.sign(request).await });
    let call = broker.incoming.recv().await.unwrap();
    let approval = broker.receive(call, &vault).await.unwrap();
    assert_eq!(vault.signs.load(Ordering::SeqCst), 0);
    assert_eq!(
        approval.fingerprint,
        vault.key.fingerprint(HashAlg::Sha256).to_string()
    );
    broker.decide(approval.id, true, &vault).await;
    let signature = signing.await.unwrap().unwrap();
    vault
        .key
        .public_key()
        .key_data()
        .verify(&vault.request().data, &signature)
        .unwrap();
    assert_eq!(vault.signs.load(Ordering::SeqCst), 1);
}
#[tokio::test]
async fn denial_and_lock_reject_signing_and_stale_approvals() {
    let vault = SyntheticVault::new();
    let (gate, mut broker) = broker();
    let directory = tempfile::tempdir().unwrap();
    let agent = RunningAgent::bind(
        &directory.path().join("private").join("agent.sock"),
        broker.endpoint.clone(),
    )
    .unwrap();
    for lock in [false, true] {
        let mut client = client(&agent).await;
        let request = vault.request();
        let signing = tokio::spawn(async move { client.sign(request).await });
        let call = broker.incoming.recv().await.unwrap();
        let approval = broker.receive(call, &vault).await.unwrap();
        if lock {
            gate.set(2, false);
            gate.set(1, true);
            assert!(!gate.access().unlocked);
        }
        broker.decide(approval.id, lock, &vault).await;
        assert!(signing.await.unwrap().is_err());
    }
    assert_eq!(vault.signs.load(Ordering::SeqCst), 0);
    assert!(broker.expire().is_empty());
}
#[tokio::test]
async fn locked_agent_does_not_query_vault_and_socket_is_private() {
    use std::os::unix::fs::PermissionsExt;
    let gate = Gate::new(1);
    let broker = Broker::new(gate);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("private").join("agent.sock");
    let agent = RunningAgent::bind(&path, broker.endpoint.clone()).unwrap();
    assert_eq!(
        std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
        0o600
    );
    let mut client = client(&agent).await;
    assert!(client.request_identities().await.is_err());
    assert!(RunningAgent::bind(&path, broker.endpoint.clone()).is_err());
    drop(agent);
    assert!(!path.exists());
}
#[tokio::test]
async fn oversized_packet_is_closed_and_unsupported_import_fails() {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (_, broker) = broker();
    let directory = tempfile::tempdir().unwrap();
    let agent = RunningAgent::bind(
        &directory.path().join("private").join("agent.sock"),
        broker.endpoint.clone(),
    )
    .unwrap();
    let mut socket = tokio::net::UnixStream::connect(&agent.path).await.unwrap();
    socket.write_u32(u32::MAX).await.unwrap();
    assert!(socket.read_u8().await.is_err());
    let mut socket = tokio::net::UnixStream::connect(&agent.path).await.unwrap();
    socket.write_u32(1).await.unwrap();
    socket.write_u8(17).await.unwrap();
    assert_eq!(socket.read_u32().await.unwrap(), 1);
    assert_eq!(socket.read_u8().await.unwrap(), 5); // SSH_AGENT_FAILURE
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openssh_ssh_add_lists_synthetic_vault_key() {
    let vault = SyntheticVault::new();
    let (_, mut broker) = broker();
    let directory = tempfile::tempdir().unwrap();
    let agent = RunningAgent::bind(
        &directory.path().join("private").join("agent.sock"),
        broker.endpoint.clone(),
    )
    .unwrap();
    let path = agent.path.clone();
    let command = tokio::task::spawn_blocking(move || {
        std::process::Command::new("ssh-add")
            .arg("-L")
            .env("SSH_AUTH_SOCK", path)
            .output()
    });
    let call = tokio::time::timeout(Duration::from_secs(3), broker.incoming.recv())
        .await
        .unwrap()
        .unwrap();
    broker.receive(call, &vault).await;
    let output = command
        .await
        .unwrap()
        .expect("OpenSSH ssh-add must be installed for this integration test");
    assert!(output.status.success());
    assert!(
        String::from_utf8(output.stdout)
            .unwrap()
            .contains("Synthetic SSH key")
    );
}
#[test]
fn rsa_flags_select_requested_sha2_and_reject_sha1() {
    let pair =
        ssh_agent::ssh_key::private::RsaKeypair::random(&mut rand::rngs::OsRng, 2048).unwrap();
    let key = PrivateKey::new(pair.into(), "synthetic RSA test").unwrap();
    for (flags, hash) in [(2, HashAlg::Sha256), (4, HashAlg::Sha512)] {
        let request = SignRequest {
            credential: key.public_key().key_data().clone().into(),
            data: b"synthetic RSA data".to_vec(),
            flags,
        };
        let signature = ssh_agent::sign_private(&key, &request).unwrap();
        assert_eq!(signature.algorithm(), Algorithm::Rsa { hash: Some(hash) });
        key.public_key()
            .key_data()
            .verify(&request.data, &signature)
            .unwrap();
    }
    let request = SignRequest {
        credential: key.public_key().key_data().clone().into(),
        data: vec![],
        flags: 0,
    };
    assert!(ssh_agent::sign_private(&key, &request).is_err());
}

#[tokio::test(start_paused = true)]
async fn expired_approval_cannot_sign_even_if_allow_arrives_later() {
    let vault = SyntheticVault::new();
    let (_, mut broker) = broker();
    let (reply, received) = tokio::sync::oneshot::channel();
    let call = ssh_agent::Call {
        epoch: 1,
        process_id: None,
        operation: ssh_agent::Operation::Sign(vault.request()),
        reply,
    };
    let approval = broker.receive(call, &vault).await.unwrap();
    tokio::time::advance(Duration::from_secs(31)).await;
    assert_eq!(broker.expire(), [approval.id]);
    broker.decide(approval.id, true, &vault).await;
    assert!(received.await.is_err());
    assert_eq!(vault.signs.load(Ordering::SeqCst), 0);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn openssh_keygen_signs_and_verifies_through_agent() {
    let vault = SyntheticVault::new();
    let (_, mut broker) = broker();
    let directory = tempfile::tempdir().unwrap();
    let agent = RunningAgent::bind(
        &directory.path().join("private").join("agent.sock"),
        broker.endpoint.clone(),
    )
    .unwrap();
    let public = directory.path().join("synthetic.pub");
    let input = directory.path().join("payload");
    std::fs::write(&public, vault.key.public_key().to_openssh().unwrap()).unwrap();
    std::fs::write(&input, b"synthetic OpenSSH interoperability message").unwrap();
    let path = agent.path.clone();
    let input_clone = input.clone();
    let mut command = tokio::task::spawn_blocking(move || {
        std::process::Command::new("ssh-keygen")
            .args(["-Y", "sign", "-f"])
            .arg(public)
            .args(["-n", "warden-test"])
            .arg(input_clone)
            .env("SSH_AUTH_SOCK", path)
            .output()
    });
    let output = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            tokio::select! {
                output = &mut command => break output.unwrap().unwrap(),
                call = broker.incoming.recv() => {
                    if let Some(approval) = broker.receive(call.unwrap(), &vault).await {
                        broker.decide(approval.id, true, &vault).await;
                    }
                }
            }
        }
    })
    .await
    .unwrap();
    assert!(output.status.success(), "OpenSSH signing failed");
    assert_eq!(vault.signs.load(Ordering::SeqCst), 1);
    let allowed = directory.path().join("allowed_signers");
    std::fs::write(
        &allowed,
        format!(
            "synthetic {}\n",
            vault.key.public_key().to_openssh().unwrap()
        ),
    )
    .unwrap();
    let verification = std::process::Command::new("ssh-keygen")
        .args(["-Y", "verify", "-f"])
        .arg(allowed)
        .args(["-I", "synthetic", "-n", "warden-test", "-s"])
        .arg(directory.path().join("payload.sig"))
        .stdin(std::fs::File::open(input).unwrap())
        .output()
        .unwrap();
    assert!(verification.status.success(), "OpenSSH verification failed");
}

#[tokio::test]
async fn revocation_cancels_an_approved_request_before_the_worker_can_sign() {
    let vault = SyntheticVault::new();
    let (gate, mut broker) = broker();
    let directory = tempfile::tempdir().unwrap();
    let agent = RunningAgent::bind(
        &directory.path().join("private/agent.sock"),
        broker.endpoint.clone(),
    )
    .unwrap();
    let mut client = client(&agent).await;
    let request = vault.request();
    let signing = tokio::spawn(async move { client.sign(request).await });
    let call = broker.incoming.recv().await.unwrap();
    let approval = broker.receive(call, &vault).await.unwrap();
    gate.revoke();
    broker.decide(approval.id, true, &vault).await;
    assert!(signing.await.unwrap().is_err());
    assert_eq!(vault.signs.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn mismatched_key_and_unknown_flags_never_reach_signing() {
    let vault = SyntheticVault::new();
    let (_, mut broker) = broker();
    for other_key in [false, true] {
        let mut request = vault.request();
        if other_key {
            request.credential = SyntheticVault::new().request().credential;
        } else {
            request.flags = 4;
        }
        let (reply, result) = tokio::sync::oneshot::channel();
        let call = ssh_agent::Call {
            epoch: 1,
            process_id: None,
            operation: ssh_agent::Operation::Sign(request),
            reply,
        };
        assert!(broker.receive(call, &vault).await.is_none());
        assert!(result.await.unwrap().is_err());
    }
    assert_eq!(vault.signs.load(Ordering::SeqCst), 0);
}
