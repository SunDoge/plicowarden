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
consumption. `stage=password_login` includes prelogin, key derivation, token
exchange and response processing. `initial_sync` and `vault_decrypt` measure the
subsequent service stages. Stage end means the scope ended and can also indicate
failure or cancellation; it does not assert success. No retries or transport
protocol changes are introduced by these diagnostics.

A slow/failed `prelogin` request precedes the main password derivation. A long
interval between prelogin headers and the next request could include body
reading and key derivation; it is not proof of CPU cost. A release build can help
CPU-bound key derivation but cannot repair a connection reset, TLS failure or
network timeout. The development profile already optimizes dependencies at
level 2.

Do not enable broad Bitwarden SDK TRACE logging on real accounts: the pinned
SDK's built-in request tracing can include request bodies. Plicowarden installs
its own restricted middleware instead.
