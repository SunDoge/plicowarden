# Plicowarden

[![CI](https://github.com/SunDoge/plicowarden/actions/workflows/ci.yml/badge.svg)](https://github.com/SunDoge/plicowarden/actions/workflows/ci.yml)
[![Release build](https://github.com/SunDoge/plicowarden/actions/workflows/release-publish.yml/badge.svg)](https://github.com/SunDoge/plicowarden/actions/workflows/release-publish.yml)
[![Latest release](https://img.shields.io/github/v/release/SunDoge/plicowarden?display_name=tag&sort=semver)](https://github.com/SunDoge/plicowarden/releases/latest)
[![License: GPL-3.0-only](https://img.shields.io/badge/license-GPL--3.0--only-blue.svg)](LICENSE)

Plicowarden is a native Rust password-manager client built with
[Plico](https://github.com/SunDoge/plico) and wgpu. It provides a three-column
vault, local encrypted cache and search, account switching, item editing,
password generation, import/export, device-login approval and an opt-in Unix
SSH agent.

> Plicowarden is under active development. Review the
> [capability inventory](docs/client-parity.md) and
> [security boundaries](docs/security.md) before using it with important data.

## Run

The repository is a Cargo workspace. The application remains at the root while
the Bitwarden service boundary lives in `crates/plicowarden-sdk/`.

```sh
cargo run --locked
```

Normal launch opens a clearly labelled synthetic demo vault. Account
credentials are accepted only through native inputs, never command-line flags,
environment variables or preview fixtures.

Generate an offscreen preview without starting account workers or reading saved
preferences:

```sh
cargo run --locked -- --scenario scenarios/vault.json \
  --capture reports/warden/vault.png
```

## Develop

```sh
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo fmt --all --check
bun install --frozen-lockfile
bun run check
bun run test
```

Visual recipes compile the Rust runner once and capture several deterministic UI
states:

```sh
python3 scripts/run-recipes.py
python3 scripts/run-recipes.py --no-build --semantic
```

## Documentation

- [Capability inventory and remaining parity work](docs/client-parity.md)
- [Accounts, encrypted cache and local search](docs/accounts-and-cache.md)
- [Platform integration, windows, tray and SSH agent](docs/platform-integration.md)
- [Native desktop packaging](docs/packaging.md)
- [Security and privacy boundaries](docs/security.md)
- [Architecture, workspace and validation](docs/development.md)
- [Visual recipe workflow](docs/recipes.md)
- [Interface layout](docs/triwarden-layout.md)
- [Login diagnostics](docs/login-diagnostics.md)
- [Source and license provenance](SOURCES.md)

## License

Plicowarden and its application service are licensed under
[GPL-3.0-only](LICENSE). Plico remains independently licensed under Apache-2.0.
