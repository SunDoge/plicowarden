# Login diagnostics

Launch the executable from a terminal with the opt-in diagnostics enabled:

```sh
umask 077
PLICOWARDEN_AUTH_TRACE=1 ./target/debug/plicowarden 2>plicowarden-auth.log
```

Use `./target/release/plicowarden` for a release build, or the executable inside
`Plicowarden.app/Contents/MacOS/` for an application bundle. Reproduce the login
failure, then quit the application and inspect `plicowarden-auth.log`.

Events use `tracing` with structured fields. With this switch, the application
installs a stderr subscriber restricted to its authentication diagnostics target;
it deliberately does not use a broad `RUST_LOG=trace` filter.

Diagnostics record fixed endpoint labels (`prelogin`, `token`, `sync`, `other`),
request IDs, status codes and elapsed milliseconds. They exclude URLs, account
identifiers, headers and request/response bodies. Transport failures report
reqwest timeout/connect/body/decode flags and underlying I/O kinds/OS codes when
available. Some TLS/DNS failures may have no I/O code; these categories alone do
not establish their root cause.

HTTP `event=headers` measures time until response headers, **not** complete body
consumption. For GET sync requests, `event=body_complete` records the total
bytes and time after the entire encrypted response body has arrived. The pinned
SDK's internal login requests may bypass this middleware; `password_login`
still measures their combined duration. `stage=password_login` includes prelogin, key derivation, token
exchange and response processing. `initial_sync` and `vault_decrypt` measure the
subsequent service stages. Stage end means the scope ended and can also indicate
failure or cancellation; it does not assert success. Read-only GET sync downloads wait up to 30 seconds for headers and 15 seconds
without a body chunk. A timeout/connection/body transport failure is retried
once, before any SDK sync handlers run. Response bodies are buffered as the SDK
already requires for JSON parsing. Mutations, authentication and HTTP error
statuses are not automatically replayed. The outer 90-second worker deadline
still applies. No transport protocol changes are introduced.

A slow/failed `prelogin` request precedes the main password derivation. A long
interval between prelogin headers and the next request could include body
reading and key derivation; it is not proof of CPU cost. A release build can help
CPU-bound key derivation but cannot repair a connection reset, TLS failure or
network timeout. The development profile already optimizes dependencies at
level 2.

Do not enable broad Bitwarden SDK TRACE logging on real accounts: the pinned
SDK's built-in request tracing can include request bodies. Plicowarden installs
its own restricted middleware instead.

## Official desktop client comparison

Read-only reference: local `bitwarden/clients`, revision `abccf1e342`.

- `libs/common/src/services/api.service.ts`, `getSync`: clients without autofill
  request `/sync?excludeDomains=true`. Desktop's
  `electron-platform-utils.service.ts` returns false from `supportsAutofill`.
  Plicowarden now uses the same parameter for initial and subsequent syncs;
  it does not consume equivalent-domain data.
- `default-sync.service.ts`, `fullSync`: separate `getSync` and
  `processResponse` measurements, deduplicate in-flight network syncs, and run
  the crypto handler before profile/folders/ciphers. Plicowarden retains the
  SDK's serialized sync and crypto-first handlers and logs complete download
  timing before those handlers begin.
- The inspected official `ApiService.send/fetch` path uses native fetch and
  awaits JSON body consumption. Its explicit request replay handles HTTP 401
  after rebuilding authentication. The 30-second header/15-second body-idle
  limits and single GET download retry here are Plicowarden-specific guards,
  not a claim about official timeout or general network-retry policy.

A local reported failure spent 2.55s authenticating and received sync HTTP 200
headers in 2.58s, then remained inside sync until the 90s operation deadline.
That log alone does not distinguish body reading from SDK response processing.
An authorized read-only sandbox probe subsequently downloaded 17,001 bytes in
9.10s, with 3.40s authentication and under 1ms decryption. These are individual
observations on an intermittent connection, not a benchmark or proof that all
stalls have the same root cause. The complete-body event provides the missing
boundary on future reproductions.

After matching the official desktop's `excludeDomains=true`, a second read-only
probe received 6,754 bytes (about 60% fewer than the earlier response) and
completed sync in 4.06s; authentication took 5.42s and decryption about 1ms.
Network variability prevents attributing the timing difference entirely to the
parameter change. Both probes ended with zero items/folders/decryption failures.

## Explicit proxy comparison

The local HTTP proxy at `127.0.0.1:7890` was compared with direct access using
the same executable and endpoint. Two public config requests over the proxy
completed in 0.82s/0.66s; direct requests took 2.52s and then hit a 20s
connection timeout. A proxy-enabled read-only sandbox login took 1.83s for
authentication and 2.24s for sync; a subsequent direct login took 4.85s and
10.12s respectively, with the same 6,754-byte response. These small sequential
samples favor a route/connectivity explanation but do not exclude deployment
placement or establish a representative latency distribution.

To opt into an existing local HTTP proxy for one launch:

```sh
HTTPS_PROXY=http://127.0.0.1:7890 HTTP_PROXY=http://127.0.0.1:7890 NO_PROXY= \
  PLICOWARDEN_AUTH_TRACE=1 ./target/debug/plicowarden 2>plicowarden-auth.log
```

The port is an example from this machine; use the actual local proxy address.
No system proxy setting is changed by this command.
