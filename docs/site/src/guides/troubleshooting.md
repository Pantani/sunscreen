# Troubleshooting

Top issues you'll run into, with concrete fixes. Sunscreen errors include a `next_step` hint by default — this page is the longer form of those hints.

## `toolchain_missing: anchor`

Sunscreen needs `anchor` for `chain build`, `chain serve`, and most `scaffold` operations.

**Fix:** install via AVM:

```bash
cargo install --git https://github.com/coral-xyz/anchor avm --locked
avm install latest
avm use latest
```

Then `sunscreen doctor` to confirm.

## `toolchain_missing: solana`

Needed for `deploy`, `wallet airdrop`, and `chain serve --runtime test-validator`.

**Fix:**

```bash
sh -c "$(curl -sSfL https://release.solana.com/stable/install)"
```

## `invalid_config` (exit 3)

`sunscreen.yml` failed schema validation. The error message names the offending field.

**Fix:** run `sunscreen doctor --json` to see the parsed config, and compare to the [schema reference](../reference/config/schema.md). Common mistakes:

- Misspelled keys (`frontend: ract` instead of `react`).
- Missing required field for a chosen variant (e.g. `framework: anchor` requires a `programs:` list).
- Version string without semver shape.

## `user_input` (exit 4)

You asked sunscreen to do something that conflicts with the current state — e.g. scaffold an instruction that already exists, or install a plugin twice.

**Fix:** re-read the message. Sunscreen refuses to clobber. Pass `--force` only when you understand what gets overwritten.

## `missing_workspace` (exit 5)

You ran a workspace-scoped command from a directory that has no `sunscreen.yml` upward.

**Fix:** `cd` into a sunscreen workspace, or `sunscreen chain new` first.

## `plugin_runtime` (exit 9)

A plugin crashed or violated the sandbox.

**Fix:**

- Run `sunscreen app run <plugin> -- --help` to confirm the binary responds.
- Check the plugin manifest declares the network/filesystem permissions it actually needs.
- `sunscreen app describe <plugin>` shows the manifest sunscreen sees.

## `chain build` succeeds but Codama fails

Common when the IDL changed shape in a way that breaks Codama's config.

**Fix:**

```bash
sunscreen generate clients --rebuild-config
```

This regenerates the Codama config from the IDL. Then re-run `chain build`.

## `chain serve` doesn't pick up file saves

Usually one of:

- The file is inside a path sunscreen ignores (e.g. `target/`, `node_modules/`, `app/.sunscreen/`).
- macOS: too many files in the watch tree exceeds the descriptor limit. Run `ulimit -n 4096`.
- Linux: inotify limit. `echo fs.inotify.max_user_watches=524288 | sudo tee -a /etc/sysctl.conf && sudo sysctl -p`.

## Frontend doesn't hot reload

Sunscreen writes `app/.sunscreen/reload` after Codama runs. Your dev server must watch that file.

**Fix:** for Vite, add to `vite.config.ts`:

```ts
server: {
  watch: { ignored: ['!**/.sunscreen/reload'] }
}
```

For Next.js, place a `useEffect` polling the file mtime, or use the included `useReload` hook from `generate frontend-hooks`.

## `airdrop` is rate-limited

The public devnet faucet throttles aggressively.

**Fix:** use <https://faucet.solana.com/> (web), or a private RPC's faucet (Helius, QuickNode), or wait 10 minutes.

## "I forgot to deploy after a code change"

```bash
sunscreen chain build && sunscreen deploy devnet
```

Sunscreen detects out-of-date `target/deploy/*.so` and rebuilds automatically before deploy.

## Still stuck?

- Search existing issues: <https://github.com/Pantani/sunscreen/issues>.
- Open a new issue with `sunscreen doctor --json` output and the command you ran.
