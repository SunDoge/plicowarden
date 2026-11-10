# Local accounts, encrypted cache and search

Plicowarden keeps one active account and supports remembering/switching multiple
accounts. Switching locks the previous account, revokes its SSH access, clears
its decrypted UI/detail/search/icon state and asks for the next account's master
password. Identical emails on different API/identity endpoints have different
account IDs. Appearance/security preferences remain device-local.

## Storage and unlock

The native application enables the SDK service's account cache under the same
application data directory as persisted login identity:

```
accounts-v1/accounts.sqlite3
```

The SQLite database uses a versioned schema and WAL. `accounts` stores account
metadata (email, region/endpoints and cache time); `vault_snapshots` stores an
SDK-encrypted envelope. Both rows update in one transaction. Newer unsupported
schema versions are rejected. On Unix the directory is private (0700) and a new
database is 0600. SQLite sidecars live in that private directory.

The authenticated envelope contains the encrypted server snapshot plus SDK
access/refresh tokens and login method. Encryption uses the Bitwarden SDK user
key and authenticated EncString format. The header contains the SDK's KDF and
master-password-wrapped user key, allowing local master-password unlock. The
account identity is duplicated inside the authenticated payload and verified
against the header and requested endpoints. No master password, unwrapped user
key, decrypted vault fields or plaintext tokens are written to the on-disk database.

After one online login with this version, the application can locally unlock
that account in a new process. The SDK restores crypto state, folders and
ciphers without password authentication or sync, then the host requests a
background sync with restored SDK-managed tokens. The cancellable download task
runs outside the foreground actor queue, so slow/offline sync does not block
cached details, reveal or local search. It is aborted on lock/account changes,
and foreground writes/manual sync take priority by cancelling it. Background
completion does not extend the inactivity deadline. Download failure keeps the
local vault open. Background results are bound to the account ID and deferred
while an editor or foreground operation is active. Foreground snapshots discard
obsolete deferred results.

**Unlock** opens the saved local vault. **Connect to server instead** explicitly
reauthenticates and refreshes the saved cache, including when recovering from a
password change or damaged cache. A failed login preserves the masked input for
retry; successful unlock, account switch, explicit lock/cancel and Back clear it.
A cache-write error is reported without pretending that offline storage worked.
Accounts without supported SDK master-password unlock material can still sign
in online but receive a cache warning.

**Switch account** exposes remembered accounts, **Add account**, and **Forget**.
Forget removes that account's local database rows, not the server account. It is
logical deletion, not a forensic-erasure guarantee. Back up SQLite through a
consistent database backup/checkpoint, not by copying only its main file while
it is open: recent commits can live in WAL.

Local unlock cannot discover remote revocation, policy/password changes or
newer items while disconnected. Offline edits are not queued: mutations still
require the server. Attachments and multiple simultaneously unlocked accounts
are not included in this implementation.

## Search

An active, unlocked account lazily builds a separate SQLite **in-memory** FTS5
index on its first query. The SDK parallel-decrypts full views for indexing;
this cost is separate from the initial unlock measurement. The disk database
never receives the decrypted index. `temp_store=MEMORY` and an in-memory journal
keep SQLite temporary search data off disk. The connection is dropped on lock
and rebuilt after a fresh snapshot; indices are not shared across accounts.

Search targets names, login usernames/URIs, notes and text custom fields.
Reprompt-protected items do not contribute notes/custom values. Dedicated
password, TOTP, payment-secret and private-key fields are excluded. Matching is
case-insensitive and requires every whitespace-separated term to match, even
across different fields. Trigram FTS handles substring queries of three or more
characters; shorter terms use escaped LIKE predicates, supporting short Chinese
queries. Queries are quoted/parameterized, not treated as raw SQL or FTS syntax.
Lunr's advanced `>` query language, stemming/fuzzy ranking, and the complete
set of identity/card search fields are not implemented.

The host sends searches without setting global busy or replacing foreground
request tickets. Stale search replies are ignored. Previous matches remain
visible while an updated query is pending, preventing empty-state flashes.

## Validation and repeatable live probe

Tests cover authenticated cache round trips, wrong passwords, account/server
isolation, header tampering, device removal, absence of plaintext secrets in
SQLite files, multi-term/CJK/literal punctuation matching, stale responses and
the account picker. Render recipes cover shared sidebar selection transitions;
the application uses Plico's navigation-entry motion and grouped page entry
transition rather than maintaining a separate animation clock.

The opt-in live example uses an explicitly supplied server and credentials on
stdin. It always uses a temporary SQLite directory. Set `PLICOWARDEN_TEST_WRITE=1`
only for an authorized dedicated test account: this additionally creates one
uniquely named synthetic login, verifies offline password/notes/CJK search and
password-field exclusion, then removes that exact fixture. It does not change
2FA or other account settings. Disable terminal input echo while supplying
credentials and restore it afterward; do not put credentials in command args,
source files or committed logs.

```sh
PLICOWARDEN_TEST_SERVER=https://your-authorized-test-server.example \
  cargo run -p plicowarden-sdk --example cached_vault
```

On the authorized Edgewarden sandbox, a new client unlocked a cached nonempty
vault in 118ms, read the synthetic encrypted password, matched notes and Chinese
queries, excluded the password field from search, and authenticated a subsequent
online sync using restored tokens. The fixture and temporary database were
removed. This is one sandbox observation, not a large-vault performance claim
or a native macOS verification.
