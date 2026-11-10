# Authorized live vault validation

Date: 2026-11-10. Target: a user-authorized disposable account on an
Edgewarden deployment. Credentials, session tokens and recovery material are not
part of this report or repository.

## Confirmed results

- Password login succeeded through the pinned native SDK and the application
  UI/host command path. The initial vault contained zero items and zero folders.
- Personal login item: create/read/update, original password preservation,
  favorite/unfavorite, archive/unarchive, trash/restore and permanent deletion.
- Card, identity and SSH-key storage: create/read/update, verify type-specific
  fields after reading, trash and permanent deletion.
- Folder: create, rename, move an item in/out, delete.
- Secure note: create/read/update and trash succeeded. The first permanent-delete
  response failed at the transport layer; cleanup subsequently completed the
  deletion. This is not a clean, uninterrupted CRUD pass.
- Authenticator 2FA: temporarily enable, require challenge on password-only login,
  reject an incorrect code, accept a current code and decrypt the vault. The
  temporary authenticator was then removed; a new SDK client authenticated with
  the password alone. Private rollback material was removed after verification.
- Vault passkey storage: encrypt a synthetic credential with the official SDK,
  upload, sync and decrypt every credential field; edit the surrounding login
  through the application's normal save path, then sync and compare again.
  The fixture was permanently removed afterward.

The passkey test uses a storage-only synthetic key and a reserved `.invalid` RP.
It does not exercise registration, assertion, account passkey login, authenticator
hardware, or an application UI for creating passkeys. SSH fixtures likewise test
storage, not agent signing with a genuine key.

Final fresh sync after cleanup: zero items, zero folders and zero decryption
failures, matching the initial empty vault.

## Timestamp precision difference and local improvement

The deployed Edgewarden normalizes passkey `creationDate` through JavaScript
`Date.toISOString()`, losing sub-millisecond precision from native SDK timestamps.
Other credential fields, including encrypted key material, survived both the
initial round trip and ordinary item editing.

The local Edgewarden change in `apps/api/src/services/ciphers/presentation.ts`
preserves valid UTC RFC3339 timestamps with up to nine fractional digits. It
retains normalization of other date formats and legacy invalid-date fallback.
This is a precision-preservation improvement, not evidence that passkey
authentication is broken. The local official server (`/home/me/code/csharp/server`,
revision `e93b96237`) uses `DateTime` in both `CipherFido2CredentialModel` and
`CipherLoginFido2CredentialData`. Its System.Text.Json round trip was checked
using that actual data model: `.123456789Z` becomes `.1234567Z` (100ns
precision), while seven-digit, millisecond and whole-second timestamps round
trip intact. Edgewarden preserving nine digits is therefore a compatible
precision extension, not an exact reproduction of official timestamp output.
Requiring byte-for-byte equality of a nine-digit date was too strict for an
official-server compatibility assertion; the encrypted credential fields remain
the critical preservation checks.

Regression tests cover nine-, seven- and three-digit precision, unchanged credential fields, offset dates and
invalid calendar dates. Eight presentation tests, nine related compatibility
and passkey utility tests, and API TypeScript checking passed. This change has
not been deployed; the live server still exhibits precision loss.

## Method and limits

Live probes call the actual vault service, pinned SDK crypto and server APIs.
Seeding/decrypting passkey fixtures and managing account 2FA additionally require
SDK/API-level access; these are not GUI end-to-end tests. Fixture names use a
unique prefix; cleanup targets only that prefix.

Transport was intermittently unreliable: TCP resets and sync body timeouts
sometimes occurred even after HTTP 200 headers. The diagnostic capsule used
HTTP/1-only requests without idle connection reuse, per-request deadlines and
bounded retries for read-only sync requests and authentication. These transport
settings are not production changes and did not eliminate all failures.
Therefore this validates the successful functional paths, not production
transport reliability or an identified HTTP/2/server defect. Writes were not
blindly retried: a failed response can follow a successful server mutation.

To repeat safely, use a dedicated account and record the initial vault counts;
create uniquely named synthetic fixtures, verify fresh sync/decryption after
writes, then delete only those fixtures. Do not change pre-existing 2FA. If
exercising temporary 2FA, save private recovery material before enabling it and
require a fresh password-only login after removal before deleting that material.
