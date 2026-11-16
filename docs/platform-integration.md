# Platform integration

## Window and interface scale

Native macOS windows start at 1224×850 logical points and 85% interface scale;
other platforms default to 1440×1000 and 100%. Settings → Appearance changes
the whole interface, including pointer, accessibility and IME geometry. Use
Cmd +/−/0 on macOS and Ctrl +/−/0 elsewhere.

Linux normally requests native decorations, leaving the outer contour and
shadow to the desktop. macOS and Windows use Plicowarden's custom close,
minimize and maximize/restore controls. Synthetic previews enable custom
controls explicitly for deterministic captures.

```sh
cargo run --locked --example display_probe -- --output reports/display
```

Ordinary widgets use solid surfaces. Scene-backed material is limited to
application chrome and dialogs; secret fields and vault cards remain opaque.
Reduced-transparency and reduced-motion preferences are handled by Plico.

## Authentication UI

Login follows the official client's email → master-password flow. The server
selector supports US, EU and self-hosted services. Returning to email preserves
the address and clears secrets. The form supports keyboard submission, password
visibility, cancellation, focus recovery and the implemented two-factor
providers. Unsupported SSO/passkey login is not advertised.

```sh
python3 scripts/run-recipes.py --suite recipes/login-review.ts
```

Transport diagnostics intentionally expose phases and fixed endpoint categories,
not request bodies or credentials. See [login-diagnostics.md](login-diagnostics.md).

## Tray and notifications

Plico provides the native tray, menu, notification callback and window lifecycle.
Plicowarden supplies branding and business events. The tray can open the window,
lock the vault or quit; close-to-tray and approval notifications are device-local
preferences. If Linux loses its tray host while the window is hidden, the window
is shown again.

Approval notifications contain no item names, fingerprints, process names or
secrets. Activating one opens the application but never approves a request.
Packaged notification identity still requires validation on macOS and Windows.

The Linux smoke test uses isolated D-Bus/X11 services and synthetic data:

```sh
cargo build --locked --example desktop_probe
/usr/bin/python3 scripts/check-native-tray.py \
  --binary target/debug/examples/desktop_probe
```

It requires Xvfb, xdotool, dbus-run-session and system Python D-Bus/GLib
bindings. It does not substitute for native macOS or Windows testing.

## Unix SSH agent

After connecting an account, enable the SSH agent in Settings and export the
shown socket as `SSH_AUTH_SOCK`. Plicowarden asks for explicit approval for every
signature. Dismissal, denial, timeout, locking, disconnecting or exiting rejects
pending requests; there is no standing application permission.

The in-memory history keeps at most 200 completed decisions and is cleared on
lock, sign-out, account switch, restart or explicit removal. It records public
metadata and outcomes, never key listings or signatures.

Eligible keys must be active vault SSH items without master-password reprompt
and with password-view permission. Ed25519, supported ECDSA and RSA SHA-2 are
accepted; RSA SHA-1 and RSA keys below 2048 bits are rejected. Imported keys,
FIDO operations, destination constraints and Windows named pipes are not
supported.

Private keys are decrypted only in the service worker for an approved request;
the UI receives public request metadata. A locked buffer protects OpenSSH input,
but the application does not claim that all SDK or parser allocations are
memory-locked.

The socket directory must be owned by the current user without group/other
permissions; the socket uses mode 0600 and existing paths are never overwritten.
`PLICOWARDEN_SSH_AUTH_SOCK`, when set, must point inside such a private absolute
directory.
