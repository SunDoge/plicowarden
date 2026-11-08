# Plicowarden recipe authoring

TypeScript imports and helpers generate a versioned JSON plan for a statically
compiled Rust runner. Bun manages dependencies and executes TS and tests;
TypeScript separately checks types. Use Bun 1.4.4 or later.

From the repository root:

```sh
bun install --frozen-lockfile
bun run check
bun run test
bun run recipes
# Reuse the compiled Rust runner after changing TS only:
python3 scripts/run-recipes.py --no-build --semantic
```

The root `bun.lock` is the only JavaScript lockfile. `tools/recipes` and `recipes`
are Bun workspaces. The pipeline checks the frozen lockfile before each run;
Bun's cached install avoids reinstalling unchanged dependencies. Cargo is still
used once when the Rust runner needs a rebuild. No separate Node/npm install
is required. `node:` imports use Bun's compatible built-in modules.

[Protocol, examples, reports and limits](../../docs/recipes.md).
