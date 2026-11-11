# Plicowarden

A native Rust frontend aligned with the independent C# Warden application:
three-column vault, the same synthetic fixture, navigation, detail sections,
account forms, editor, password generator and appearance settings. Built with
Plico's immediate widgets and default wgpu desktop host. No .NET runtime, C ABI,
source symlinks or original Warden checkout is required.

```sh
cargo run --locked -p plicowarden
# Synthetic-only offscreen previews, without account workers or saved preferences:
cargo run --locked -p plicowarden -- --capture reports/warden/vault.png
cargo run --locked -p plicowarden -- --scenario scenarios/login.json --capture reports/warden/login.png
cargo run --locked -p plicowarden -- --scenario scenarios/compact.json --capture reports/warden/compact.png
```

Other fixtures: settings, search, generator and editor. `--width`, `--height` and
`--screen` select a simple capture without a scenario. Credentials are entered
only through native inputs, never flags, environment variables or fixture JSON.
Normal launch starts with the same clearly labelled demo vault as C# Warden.


## Repository boundary

This is an independent GPL-3.0-only Cargo workspace. Plico stays Apache-2.0;
it does not depend on Plicowarden or the Bitwarden SDK. The `sdk/` package is
`plicowarden-sdk`; the application package/binary is `plicowarden`.

Plico dependencies are pinned to Git commit `9c3eab38a16c8151b32275135ccc7f82fa9a3e8f`
from `https://github.com/SunDoge/plico.git`. Cargo fetches the framework;
a sibling Plico checkout is not required. `Cargo.lock` records the same source.
No original Warden, Warden Desktop, Wabou or .NET checkout is needed.

## SSH agent (Unix)

After connecting a real account, open Settings and choose **Enable SSH agent**.
Set `SSH_AUTH_SOCK` to the socket path shown there; `ssh-add -L` lists eligible
vault keys. OpenSSH and Git SSH signing can request signatures. Plicowarden asks for
approval **for each signature**, showing key name, SHA-256 fingerprint and peer
PID when the OS provides it. The PID is socket-peer metadata, not verified app
identity, destination or signing purpose. Dismissal, denial and a 30-second
timeout reject the request. Disabling, idle lock, disconnect or exit revokes all
pending approvals. Enable again after signing in; enablement is not persisted.

Settings · SSH agent splits into **Permissions** and **History**. Permissions shows
the current agent state; every signature is approved once, so no app receives
standing access. History keeps the last 200 completed requests in memory: key name,
SHA-256 fingerprint, peer PID when available, how the request ended (allowed, denied,
timed out, vault locked or failed) and how long ago. Key listings and signatures are
not recorded. Locking the vault, signing out or switching accounts clears the history,
as does **Clear history**; it is not persisted across app restarts.

Only non-archived, non-trashed SSH items without master-password reprompt and
with password-view permission are eligible. Only Ed25519, supported ECDSA and
RSA SHA-2 signatures are accepted; RSA SHA-1 and RSA keys below 2048 bits are
rejected. The agent does not accept imported private keys via `ssh-add`.
Private keys are decrypted and parsed in the worker for approved requests,
with no private-key cache in the UI. A checked locked buffer protects the
OpenSSH input; SDK/parser allocations are not claimed to be fully locked.

The socket parent must be owned by you with no group/other permissions; socket
mode is 0600. Existing socket paths are never overwritten. Optional
`PLICOWARDEN_SSH_AUTH_SOCK` must be an absolute path inside such a private directory.
Windows pipes, FIDO/security-key operations, destination-constrained forwarding,
platform packaging and real-account SSH integration are not validated here.

## Interface size

Native macOS windows start at 1224×850 logical points and 85% interface scale;
other platforms keep 1440×1000 and 100%. Retina backing density remains managed
by the OS. Settings → Appearance → Interface scale changes the whole interface,
including pointer, accessibility and IME geometry through Plico's ZoomSource.
Use Cmd +/−/0 on macOS (Ctrl elsewhere); 0 means 100%. Save appearance to persist
the current scale. Old appearance files receive the platform's default scale.
The pinned renderer also crops oversized opacity targets to their actual ink,
so small dimmed controls do not each allocate a full Retina window texture;
the 128 MiB frame budget remains in place.

Reference JSON/TS preview fixtures remain at 100% density on every platform.
Linux normal windows request native decorations: the window system supplies
buttons and the outer contour, without an additional client-area round clip.
The exact corners and shadow follow the desktop's decoration implementation.
macOS and Windows retain the custom close, minimize and maximize/restore buttons.
Synthetic preview fixtures explicitly enable custom controls for repeatable captures. They retain OS-point sizing
independent of content zoom. The same controls are available on the login page;
the title region starts an OS-managed window drag. Maximize/restore queries the
current native state rather than maintaining an application-side guess.

For synthetic 85% captures at both 1× and Retina 2× backing density:

```sh
cargo run --locked --example display_probe -- --output reports/display
```

## Login flow

Login follows the local official Bitwarden client's email → master-password flow.
The server selector sits below the form; self-hosted endpoints expand only when
requested. Returning to email preserves the address and clears secrets. The form
supports Enter, focus recovery after errors, password visibility, cancellation
and the existing two-factor providers. Escape returns to email or dismisses server
settings. Invalid self-hosted endpoints are reported before submitting credentials;
changing verification methods clears the previous code. It does not advertise unsupported SSO/passkey login.

Reference: `bitwarden/clients/libs/auth/src/angular/login/login.component.html`,
`libs/angular/src/auth/environment-selector/environment-selector.component.html`
and `libs/components/src/anon-layout/anon-layout.component.html` (read-only design
references; no Angular source copied).

```sh
python3 scripts/run-recipes.py --suite recipes/login-review.ts
```

## Memory regression probe

The pinned renderer fixes a backdrop-output retention bug that allocated more
textures on every scrolling frame. Preflight reservation is now separate from
actual draw ownership. Old scene/image resources are released on removal;
coverage masks use one-channel GPU textures without changing coverage pixels.

Run synthetic stress without opening accounts or a window:

```sh
cargo run --locked --example memory_probe -- --frames 600 --scroll
cargo run --locked --example memory_probe -- --frames 60
```

JSON lines report RSS (from `ps`), output targets, effect/image cache bytes and
CPU raster cache bytes. This offscreen test includes CPU readback storage and
platform driver overhead; RSS is not interchangeable with macOS Activity Monitor
footprint. Cache trimming need not immediately return allocator/driver pages.

## Development and architecture

Root dev settings keep the framework at opt1 and third-party dependencies at
opt2, use line tables for workspace debugging, disable dependency debug info,
and set this application's optimization to opt0. Release settings are unchanged.
The `plico new` template follows the same fast-iteration policy for new apps.

- `src/model.rs`: ordinary Rust UI state and commands; no serialized application
  snapshots, SDK session objects or IO in widget declarations.
- `src/ui.rs`, `detail.rs`, `forms.rs`: Context-first native controls with
  `#[component]` and `layout + |cx| { ... }`. No application renderer Widget trait.
- `src/host.rs`: owns account and preference workers and forwards their wakeups.
  Workers receive values and return replies; they never retain UI state references.
- `sdk/`: copied GPL application service using the same pinned official Bitwarden
  SDK as C# Warden. Rust messages replace C ABI/JSON request buffers. The framework
  does not depend on this service; UI-only edits do not recompile its implementation.
- `src/appearance.rs`: appearance-only JSON, saved by a host worker under the OS
  local data directory for **Plicowarden**, independently of C# preferences.
- The interface follows Triwarden's floating-panel layout (see
  [the layout notes](docs/triwarden-layout.md)) and uses ordinary Plico
  widgets over a soft pastel window wash. Panels, cards, fields and the
  floating footer are solid white with hairline borders and soft shadows;
  reduced transparency drops the wash. Colors and control proportions follow
  MyGo's standard light theme.

## Official client parity

See [the capability inventory](docs/client-parity.md) for implemented paths,
verification limits and the remaining official desktop-client work. The normal vault
keeps navigation/list/details visible at the supported minimum window size; screenshots
with a hidden sidebar are explicitly toggled states, not the default.

Single and bulk folder moves preserve the complete item payload and revision
preconditions, validate destination membership and report partial failures. No folder
filters out archived/trashed items. Bulk selection supports favorite/archive/trash/restore/purge with destructive
confirmation and partial-failure reporting. Import / Export provides personal-vault
JSON/password-protected JSON exports and official-SDK KeePass KDBX import. Native
file dialogs run in the host; credentials and decrypted export data remain in workers.
Exports never overwrite an existing file; Unix export files are created with mode 0600.

```sh
python3 scripts/run-recipes.py --suite recipes/folder-review.ts
python3 scripts/run-recipes.py --suite recipes/bulk-review.ts
python3 scripts/run-recipes.py --suite recipes/transfer-review.ts
```

## Implemented paths

Search, reverse sort, type/favorite/folder/archive/trash filters, item selection,
password reveal/hide, one-time codes, copy actions, cancel/lock and demo recovery.
Account login supports US/EU/self-hosted and custom service URLs, plus the same
supported two-factor providers as the copied SDK service. Invalid HTTP endpoints
are rejected before a network request.

The five-type editor sends changed field patches with revisions; untouched URI
matching and optional SDK data are preserved. Personal item create/edit,
favorites, clone, archive, trash, restore/purge and folder mutations are wired to
the copied service. Shared items follow its read-only/ownership guards. Hidden
inputs use secret buffers; multiline key editing follows the C# editor's visible
text-area behavior. SSH agent support uses a separate, opt-in signing boundary; see below.

Password history is fetched on explicit reveal, respects password-view permissions, and can be copied using the existing clipboard expiry worker. Hiding history, changing selection/filter or locking clears its zeroizing display buffers.

The generator supports passwords, 6–12-word EFF short-list passphrases and readable random usernames (availability is not checked). All modes use OS cryptographic randomness with unbiased selection, required
character groups and a shuffled result. History is memory-only and cleared on
lock. SDK sessions, requests and replies are not Debug/Serialize UI state.
Settings has Appearance, Security and SSH agent sections. Device-local `security.json`
stores only auto-lock (1/5/15/30/60 minutes), clipboard expiry (10/30/60/120 seconds)
and minimize-after-copy preferences. Defaults remain five minutes and 30 seconds.
Successful saves update an existing worker as well as future connections.
Clicks, typing, dragging and scrolling refresh live sessions; hover and rendering do not.
Expired sessions cannot be revived by new input. Clipboard expiry clears only app-owned,
unchanged content. These services stay in the worker;
lock disconnects the old actor and ignores obsolete replies. Expired TOTP values
are removed via worker wakeups, without reading system time inside components.

## Tray and system notifications

Plico provides the native tray/menu/notification lifecycle through its optional
`plico-desktop/desktop-integration` feature. This app supplies branding, menu IDs,
SSH notification policy and preference persistence; it does not depend directly on
tray-icon, notify-rust, zbus or plico-shell.

Normal launches create a native tray icon (a template image in the macOS menu bar).
Its menu opens the window, locks the vault, or quits the process. Closing the app
window keeps its SDK worker alive when a tray host is available; automatic locking
continues while hidden. Quit revokes the SSH signing gate and drops the worker.
The menu has persistent “Keep running when closed” and “SSH approval notifications”
options in device-local `desktop.json`, both enabled by default.

New accepted SSH approval requests show a generic system notification. Clicking it
opens the app; it never approves signing. Names, fingerprints, processes and secrets
are excluded from notification text. At most one notification observer runs at a time.
The platform controls notification permissions and delivery; signing approval remains
usable inside the app if delivery is unavailable. Windows notification app identity and
macOS notification delivery still need packaged-app validation.

Linux uses StatusNotifierItem over D-Bus without GTK. Without a registered tray host,
closing exits normally. If the host disappears while the window is hidden, it is shown
again. Native tray initialization failures also preserve normal close/exit behavior.
This is separate from start-at-login, a single-instance service, OS screen-lock handling,
and application menus, which remain pending.

The Linux native smoke test uses isolated D-Bus/X11 services and only synthetic data:

```sh
cargo build --locked --example desktop_probe
/usr/bin/python3 scripts/check-native-tray.py --binary target/debug/examples/desktop_probe
```

It needs Xvfb, xdotool, dbus-run-session, and system Python's dbus/GLib bindings.
It checks native menus, hiding/restoration, notification activation, saved preferences,
tray-host loss, quit and the no-tray fallback. It does not substitute for macOS testing.

## Validation and limits

```sh
cargo test --locked -p plicowarden -p plicowarden-sdk
cargo clippy --locked -p plicowarden -p plicowarden-sdk --all-targets -- -D warnings
plico fmt --check src tests sdk/src
```

To install the matching formatter CLI when needed:

```sh
cargo install --git https://github.com/SunDoge/plico.git --rev 9c3eab38a16c8151b32275135ccc7f82fa9a3e8f --locked plico-cli
```

Tests drive real declarations, layout and native input through the deterministic
runtime driver. They cover C# demo interactions, empty recovery, all preview forms
at 1440/1000/820 px, centered login controls and reference column boundaries,
SDK HTTP rejection delivered through a wake, password-input clearing, stale-reply
rejection, field-patch preservation, generation and preference round trips.
The copied service retains its organization-key, cancellation, strict patch and
locked-write tests. Tests never access a real vault.

This is a development client. Server-side authenticated login/sync/CRUD and every
provider have not been end-to-end revalidated for this frontend in this task;
those calls are wired, not a claim of audited or production parity. Demo passkey,
Watchtower and attachment rows retain the C# sample display and perform no real
operations. No persistent/offline vault, biometric/PIN unlock, SSO/WebAuthn login,
AutoFill, passkey provider, attachment transfer, Send or import/export.

Clipboard compare-and-clear is best effort, not atomic across desktops. Temporary
SDK strings, render caches and explicitly revealed text are not guaranteed to be
fully erased from process memory. Only appearance is intentionally persisted.
Captured scenes contain synthetic data only. Platform behavior beyond the local
Linux wgpu validation is not claimed.

## Licensing

This application and its copied SDK integration are **GPL-3.0-only**. Plico's
framework remains Apache-2.0. See [SOURCES.md](SOURCES.md), the application LICENSE,
SDK LICENSE and included asset/third-party notices for provenance.

## Recipe batching experiment

Build once, then run the original seven synthetic recipes with assertions and screenshots:

```sh
cargo build --locked -p plicowarden --example recipe_probe
target/debug/examples/recipe_probe --manifest scenarios/recipes.json \
  --mode shared --output reports/recipes
```

Run from the repository root. Modes: `semantic` (no GPU/PNG), `fresh` (a renderer
per case), `shared` (reuse backend resources, create fresh app/input/clock per case).
The manifest adds final `present` / `absent` semantic-label assertions to existing
JSON scenarios. Each failed case is reported; later cases continue, and any
failure makes the process exit nonzero. Scenario paths are working-directory relative.
This is an experimental example, not a general CLI contract. See
[the experiment](docs/experiments/recipe-batching.md) for results and limits.

## TypeScript recipes (Bun)

Bun >= 1.4.4 manages TS dependencies and runs the authoring tools and tests.
The root `bun.lock` is the only JavaScript lockfile. `bun run check` typechecks
the suite; `bun run test` runs helper tests.

The current authoring path supports shared fixtures/flows, a viewport matrix,
in-flow assertions, multiple captures and synthetic account result injection:

```sh
python3 scripts/run-recipes.py
# After editing TS only, reuse the Rust executable:
python3 scripts/run-recipes.py --no-build
python3 scripts/run-recipes.py --no-build --semantic
```

See [the actual suite](recipes/suite.ts) and [recipe documentation](docs/recipes.md).
The Rust `recipes` example receives the generated plan; it never opens a real SDK
client or user preferences. Failures retain source location, flow, semantic evidence,
last successful capture and a failure PNG. This supersedes `recipe_probe` for
composable test authoring; the older runner remains for its original benchmark.

## Appearance

The application uses ordinary Plico widgets throughout, including toolbars,
login, settings and dialogs. White panels sit on a neutral gray background;
controls use MyGo's light palette, 6px corners and blue accents. Circular icon
buttons keep their circular shape. Borders and subtle shadows separate surfaces.
No backdrop blur, refraction or ambience shader is declared by the application.

Interface scale remains persistent. Older appearance files are accepted: obsolete
glass preferences are ignored, while the saved scale is retained. The legacy
`opaque` preview input is accepted for existing recipes; all previews now use
solid surfaces.
