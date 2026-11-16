# Security and privacy boundaries

Plicowarden is under active development. Its architecture limits where secrets
travel, but it is not an independently audited password manager. Consult
[client-parity.md](client-parity.md) for exact implemented and validated paths.

## Secret boundaries

- Credentials enter through native inputs, never preview JSON, command-line
  flags or environment variables.
- SDK sessions, requests and replies are not serialized UI state.
- Private SSH keys remain in the service worker; approval UI receives only
  public metadata.
- Hidden inputs use secret buffers. Locking drops editor buffers, pending
  approvals and active service state.
- Diagnostic logs expose fixed phases and transport categories rather than
  request bodies, tokens or passwords.
- Synthetic recipes and screenshots must never contain real account data.

## Local data

The encrypted SQLite cache is account-isolated and authenticated. It supports
offline unlock and local search without storing plaintext vault contents or
session tokens; see [accounts-and-cache.md](accounts-and-cache.md).

Device-local appearance, desktop and security preferences contain UI policy,
timeouts and server-selection metadata, not master passwords. SSH approval
history and generator history are memory-only.

Exports never overwrite an existing file. Unix export files are created with
mode 0600. Plaintext export requires an explicit user choice.

## Limits

- Clipboard compare-and-clear is best effort and not atomic across desktop
  environments.
- Temporary SDK strings, renderer caches and explicitly revealed text are not
  guaranteed to be fully erased from process memory.
- Locked buffers cover selected application allocations, not every allocation
  made by third-party SDKs, parsers or platform services.
- System notifications, native packaging and platform credential facilities
  remain subject to OS policy and require platform-specific validation.
- Wired server operations are not a claim of complete production parity or a
  security audit.

## Validation

Automated tests cover cache authentication and account isolation, locked-write
rejection, stale replies, timeout boundaries, transfer permissions, SSH signing
approval and deterministic UI behavior. Tests use synthetic fixtures and do not
connect to real accounts.

Run the full local checks with:

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

Security reports should avoid including real vault data, screenshots or logs.
