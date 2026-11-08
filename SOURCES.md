# Sources and licenses

Warden Rust and its SDK service are GPL-3.0-only (see LICENSE). Plico framework
crates remain Apache-2.0 and do not depend on this application.

- UI and synthetic fixture adapted from independent C# Warden's uncommitted
  source snapshot read on 2026-11-08 (that repository has no HEAD commit).
- `sdk/src/service.rs`, `sdk/src/service/`, `sdk/src/server.rs`, and
  `sdk/src/clipboard.rs` copied from its GPL-3.0-only native backend; transport
  replaced with a typed Rust actor, without C ABI.
- Original SDK adaptation: Warden Desktop
  `43f390e7a800845c66872df377767ab57f7c67b6`. Official Bitwarden SDK pinned to
  `fbd21679c1c3690cba424ad3dbb0ba662eb9863e`.
- Brand SVGs: Simple Icons 16.0.0, CC0 (asset notice included), via C# Warden.
  `ambience.wgsl`: C# Warden's app-specific background composition.
- Glass and elevation are consumed from `plico::materials`; Warden no longer
  embeds a glass shader or maintains optical formulas. The framework records
  MyGO/Plico provenance and licenses in `crates/plico/third_party/README.md`.


All reused files are local copies, with no dependency on the original checkouts.

## Reference snapshot SHA-256

- `Vault.cs`: `ec6a06779553154cc260674d4226fd4a916bf9a6342c043b7e46065959be4a1f`
- `Account.cs`: `5c8bf1c12458fbd0322e2b67be8091c8cdfbf7ccb7cfa8c49868b864c3d85491`
- `Editor.cs`: `aa9ad3e446bb2a31fe3e56d77eba2b3dc8c174214882cd7749d82bdc102f16a7`
- `Settings.cs`: `58c82485b60db058551a199bbc14e19a78e6626ac8f244119aa21fc653155070`
- `Actions.cs`: `f8f94e03fc7fbc9b0fe242fdf6dfd4e1204becfc26874851bc54686db5798def`
- `native/src/service.rs`: `26369f62814ef1ed5ed503695541253a5b4186ceeb6c946aa9903603c0ecfd6f`
- `native/src/service/items.rs`: `4e273d6f0b3ef5a2b5f93657baee5f8353098480f5c50f270f3842ded4e86d6a`
- `native/src/service/writes.rs`: `f30a889a2be2c835fedee921f28a384f3afae009581d70079f1849e2ca3b25f7`

## EFF short wordlist

`assets/eff-short-words.txt` contains EFF Short Wordlist #1 by Joseph Bonneau / Electronic Frontier Foundation, with dice-number columns removed. Source: https://www.eff.org/files/2016/09/08/eff_short_wordlist_1.txt (retrieved 2026-11-08). Licensed under CC BY 4.0: https://creativecommons.org/licenses/by/4.0/ ; EFF policy: https://www.eff.org/copyright . No endorsement is implied.

## SSH agent and protected buffer

`sdk/src/ssh_agent.rs`, `sdk/src/protected.rs`, `sdk/tests/ssh_agent.rs` and
VaultService SSH methods were copied/adapted from GPL-3.0-only Warden Desktop
commit `43f390e7a800845c66872df377767ab57f7c67b6`. No original checkout dependency.
`tools/recipes/` is copied from Apache-2.0 Plico development sources on
2026-11-08; its Apache license is retained in `tools/recipes/LICENSE`.
