# Official desktop client parity

Reference checkout: local `bitwarden/clients`, read-only. This is a feature inventory,
not a claim that Plicowarden is already a complete replacement. “Implemented” means
there is a UI → host → SDK path; real-account/network/platform verification is
listed separately. Automated UI fixtures use synthetic data; a separate, explicitly
authorized live sandbox check is recorded in `experiments/live-vault-validation.md`.

## Implemented application paths

| Capability | Current behavior | Verification limit |
| --- | --- | --- |
| Vault navigation | Navigation, list and details default to three columns down to 820 logical px; explicit sidebar toggle retained | UI and viewport tests |
| Search and filters | Text, type, favorites, folder, archive, trash and TOTP | Synthetic input/recipes |
| Login | Email → password, US/EU/self-hosted endpoints, supported 2FA, cancel, retry, animated busy indicator and worker-driven authentication/sync/unlock stages | Synthetic replies/configuration; authorized Edgewarden account login via SDK and UI host, authenticator challenge/invalid-code/valid-code paths checked |
| Item operations | Personal login/card/identity/note/SSH create/edit, revision patches, favorites, clone, archive, trash, restore/purge | SDK guards/synthetic forms; authorized live personal login/card/identity/SSH CRUD checked; note delete required cleanup after transport failure (see live validation); organization items remain read-only |
| Bulk actions | Multi-select/select all, favorite/unfavorite, archive/unarchive, trash, restore/purge, move to/from personal folders; destructive confirmation; per-item failures | SDK locked/ownership guards and UI command tests; no real server mutation |
| Folders | Create/rename/delete, single/bulk move, folder/no-folder filtering | Authorized live create/rename/delete and single-item move in/out; bulk/filter tests remain synthetic |
| Secrets | Explicit reveal/copy, password history, TOTP generation/expiry | Synthetic tests and worker boundaries |
| Site icons | SDK icon URLs retained; host downloads images asynchronously from the configured icon service, with first-letter fallback | Public Edgewarden icon endpoint checked; bounded worker/cache and dimension guards; no vault tokens attached |
| Export | Personal non-trashed items/folders, official JSON or password-protected JSON; item count checked before writing; no overwrite; Unix file mode 0600 | File privacy/guard tests; no real-account export/restore roundtrip |
| Import | Official SDK KDBX parser/encryption/upload, optional key file path, personal destination, 10 MiB bound, refresh after upload | Locked/size guards and form tests; no real-account import |
| Generator | Password, passphrase, username, copy and in-memory history | OS randomness and native-input tests |
| SSH agent | Unix signing approval, timeout/lock revocation | Local synthetic agent tests; no real vault integration |
| Tray / notifications | Native menu (open/lock/quit), close-to-tray with fallback, saved close/notification preferences, generic SSH notification activation | Isolated Linux X11/D-Bus smoke and host lock tests; native macOS/Windows and packaged notification identity pending |
| Window chrome | App-drawn controls and OS drag/minimize/close/maximize commands; content zoom independent | Command/geometry tests; macOS runtime pending |
| Security settings | Device-local auto-lock (1–60 minutes), clipboard expiry (10–120 seconds), minimize after successful copy; meaningful input refreshes a live session | Synthetic expiry, persistence/error and copy command tests; no real clipboard expiry or macOS runtime test |
| Appearance | Solid MyGo-style controls, subtle borders/shadows and persistent interface zoom | Render recipes; native macOS appearance pending |

Bulk writes are not transactions: ownership/reprompt guards run before any write,
then independent server operations can succeed or fail. The result retains failed
IDs and reports partial failures. A failed final sync explicitly says to sync before
retrying. Import can likewise create duplicates when repeated; it does not merge.

Export excludes organization items, trash and attachment bytes. An unreadable,
reprompt-protected or omitted item prevents the export rather than silently producing
an incomplete backup. The pinned SDK controls format details and crypto buffering;
this app does not claim byte-for-byte parity or full transient-memory erasure.

## Remaining work toward the full official desktop client

| Area | Missing capability |
| --- | --- |
| Account/session | Multiple accounts, offline encrypted cache, password/PIN/biometric unlock, timeout action choice, logout-vs-lock semantics |
| Authentication | SSO, passkey/device login, WebAuthn/Duo native flows, recovery, registration, password hint/change and device approval |
| Vault types | Bank account, passport and driver's license UI/mutations; richer URI matching and master-password reprompt |
| Vault actions | Remaining official context actions; persistent sort/view preferences |
| Organizations | Vault/collection scope, shared folder membership and permissions, assignment, ownership transfer and org-specific policies |
| Attachments | Upload/download/delete and file encryption, account storage/quota feedback |
| Import formats | Bitwarden JSON/encrypted JSON/CSV, browser/other manager parsers, destination/review/duplicate controls |
| Export formats | CSV, organization exports, authenticated export re-prompt and tested restoration compatibility |
| Send | Text/file Send create/edit/delete, expiration/access count, password and share links |
| Autofill | Browser/native messaging, system credential provider/passkeys and desktop autotype |
| OS integration | Touch ID/Windows Hello, keychain storage, launch-at-login, screen-lock/session events, deep links and app shortcuts |
| Settings | Remaining security/account/app preferences, language/theme, favicon/privacy preferences |
| Packaging | Signed/notarized macOS bundle, Windows/Linux packaging, updater and application menus |
| Verification | Broader real-account sandbox coverage (imports/exports, attachments, organizations), native macOS visual/input/window tests, accessibility/platform review |

## Source entry points inspected

- `apps/desktop/src/vault/app/vault-v3/vault.component.html`
- `apps/desktop/src/vault/app/vault-v3/vault-list.component.html`
- `apps/desktop/src/vault/app/vault-v3/vault-filter/vault-filter.component.html`
- `apps/desktop/src/app/accounts/settings-dialog.component.html`
- `apps/desktop/src/app/tools/import/import-desktop.component.html`
- `apps/desktop/src/app/tools/export/export-desktop.component.html`
- `apps/desktop/src/app/tools/send/send.component.html`
- `libs/auth/src/angular/login/login.component.html`
- `libs/auth/src/angular/two-factor-auth/two-factor-auth.component.html`

The application is GPL-3.0-only. Framework effects remain in Plico; no official
Angular application code was copied into the Apache-licensed framework.

Login progress events retain their request ticket and do not finish the in-flight
request. Late events from cancelled/older requests are ignored. The worker's
90-second timeout still closes the session; its following lock event preserves
the sign-in error rather than clearing it. Stage labels describe actual operation
boundaries; authentication includes prelogin, key derivation and token exchange,
with no fabricated percentage or per-request breakdown inside the pinned SDK.

Detail-only reads do not set the global busy state. Cached item details remain
visible while revalidating, and loading placeholders appear only when no cached
details exist. The detail toolbar reserves its space during the initial read.
Login submission retains the masked secret buffer for same-account retries; a
timeout cleanup preserves it as well. Successful login, explicit lock/cancel and
returning to the account step clear it. The buffer is not persisted.

Site icon downloads use a separate host-owned worker (four concurrent requests,
a bounded queue, up to 512 cached success/failure entries per session). Encoded
images are capped at 128 KiB and 256×256 pixels. Failures retain the initial
fallback and do not fail account operations; locking drops the loader/cache.
