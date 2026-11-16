# Development and architecture

## Repository boundary

Plicowarden is an independent GPL-3.0-only Cargo workspace. Plico stays
Apache-2.0 and does not depend on Plicowarden or the Bitwarden SDK. All Plico
dependencies are pinned to one Git revision in the root `Cargo.toml`; Cargo does
not require a sibling framework checkout.

The root package is the application and binary. Stable, heavy service code lives
in `crates/plicowarden-sdk/`, so ordinary UI edits do not rebuild its
implementation. Workspace package metadata owns the shared version, edition,
license and repository. Both packages are private Cargo packages; release-plz
creates one Git tag and GitHub Release without publishing to crates.io.

## Code layout

- `src/model.rs` contains ordinary Rust UI state and commands. UI declarations
  contain no IO or SDK session objects.
- `src/ui.rs`, `src/detail.rs` and `src/forms.rs` contain context-first Plico
  components.
- `src/host.rs` owns account and preference workers and forwards typed replies
  to the UI. Workers never retain UI-state references.
- `crates/plicowarden-sdk/` owns the copied GPL application service, encrypted
  cache, network operations, clipboard expiry and Unix SSH agent.
- `src/appearance.rs` persists appearance-only settings independently of account
  data.
- `tests/` drives declarations, layout and native input through Plico's
  deterministic runtime.

Development profiles use optimized dependencies with reduced debug information;
the application package itself stays at opt-level 0 for quick UI iteration.
Release profiles use Cargo defaults.

## Validation

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
bun install --frozen-lockfile
bun run check
bun run test
```

Tests use synthetic fixtures and never access a real vault. The application
tests cover layout, semantics, input, preferences, stale replies and UI state;
the service crate tests cache isolation, writes, transfers, authentication
timeouts and SSH-agent policy.

## Memory regression probe

The vault uses a variable-height virtual list, so retained UI nodes scale with
the viewport rather than the total account size. Run the offscreen probe without
opening an account or native window:

```sh
cargo run --locked --example memory_probe -- --frames 600 --scroll
cargo run --locked --example memory_probe -- --frames 60
```

The probe reports process RSS, render targets and CPU/GPU cache estimates. It
includes readback and driver overhead, so its RSS is not directly comparable to
platform activity monitors. In the 108-item, 1440×1000, 2× synthetic probe,
virtualization reduced the warm retained-node median from 1,036 to 372. Visible
GPU work remained nearly flat because the same pixels were rendered.

## Recipe runners

Rust tests own behavior assertions. TypeScript recipes organize deterministic
visual states and batch screenshots; see [recipes.md](recipes.md).

The older `recipe_probe` example remains only for the original shared-renderer
experiment:

```sh
cargo build --locked --example recipe_probe
target/debug/examples/recipe_probe --manifest scenarios/recipes.json \
  --mode shared --output reports/recipes
```

Its `semantic`, `fresh` and `shared` modes are experimental, not a public CLI
contract. Historical measurements are documented in
[experiments/recipe-batching.md](experiments/recipe-batching.md).
