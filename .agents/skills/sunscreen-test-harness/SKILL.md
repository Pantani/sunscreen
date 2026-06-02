---
name: sunscreen-test-harness
description: Use sempre que o usuario pedir testes de verdade, validacao pesada, integracao real, test harness, QA end-to-end, stress, anti-flake, release QA, cargo-dist, Anchor/Solana/Codama real, Pinocchio SBF real, Surfpool/test-validator, frontend typecheck, plugin runtime, CI hardening ou provar que o app sunscreen esta funcionando. Tambem use para reexecutar, atualizar, corrigir, expandir ou auditar ondas de testes do sunscreen.
---

# Sunscreen Test Harness

Orquestra o time de validacao pesada do `sunscreen`. A meta e provar comportamento real sem transformar todo teste em uma dependencia fragil de rede/toolchain. Separe sempre o que e smoke offline, o que e heavy local gated, e o que validou uma toolchain Solana real.

## Team

- `test-harness-orchestrator`: lider da rodada, le `summary.json`, delega tiers e consolida status.
- `qa-integrator`: lider de qualidade e fechamento da rodada.
- `test-strategist`: matriz de risco, tiers, criterios de aceite e donos.
- `offline-ci-owner`: fmt/clippy/test/build/no-default e command-group smokes.
- `real-anchor-codama-owner`: Anchor/Solana/Codama/pnpm/node reais.
- `pinocchio-sbf-owner`: Pinocchio e `cargo build-sbf` real.
- `serve-runtime-owner`: Surfpool/test-validator, watcher, portas, build trigger e teardown.
- `plugin-runtime-qa`: manifesto, stdio JSON-RPC, gRPC, sandbox, marketplace e dynamic scaffold.
- `frontend-codegen-owner`: hooks/clientes, Next/Vite, pnpm install e typecheck.
- `release-distribution-qa`: cargo-dist, binario release, instalador, changelog, docs e completions.
- `flake-perf-auditor`: repeticao, timeouts, cold-start e instabilidade.

## Phase 0: Current State

1. Leia `AGENTS.md`, `CLAUDE.md`, `ROADMAP.md`, `.github/workflows/ci.yml`, `tests/**`, `scripts/integration-heavy.sh` e `git status`.
2. Confirme se o pedido e uma rodada de teste, expansao do harness, auditoria de CI ou validacao de release.
3. Se existirem logs em `_workspace/test-harness/`, trate como historico, nao como prova atual.
4. Preserve mudancas locais do usuario.

## Execution Mode

Use modo hibrido:

- Se subagentes estiverem disponiveis e o usuario pediu harness/equipe, delegue auditorias independentes para os especialistas.
- Se subagentes nao estiverem disponiveis, execute localmente seguindo os donos acima.
- Nunca marque um tier como aprovado apenas porque um teste ignored/skipped retornou sucesso.

## Orchestrator Flow

1. `test-harness-orchestrator` abre a rodada e registra o escopo em `_workspace/test-harness/orchestrator-report.md`.
2. `test-strategist` cria a matriz de risco quando o pedido for amplo.
3. `offline-ci-owner` roda `bash scripts/integration-heavy.sh`.
4. O orquestrador le o `*.summary.json` mais recente e classifica cada tier.
5. Tiers skipped ou blocked sao delegados aos especialistas certos apenas quando o usuario pediu aquela validacao.
6. `qa-integrator` fecha o relatorio final com evidencias e proximo menor passo.

## Test Tiers

### Tier 1: Offline Deterministic Gate

Roda em qualquer maquina e no CI normal.

```bash
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo check --locked --no-default-features --all-targets
cargo test --locked --all --all-features --no-fail-fast
cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding --test app_lifecycle
cargo test --locked --test compile_generated_workspace
cargo build --locked --release --all-features
```

Aceite: todos passam, sem snapshot drift, sem warning clippy, sem feature-gate quebrado.

### Tier 2: Generated Workspace Compile Gate

Valida que workspaces gerados continuam compilaveis com dependencias reais/cache local quando aplicavel.

```bash
SUNSCREEN_COMPILE_TESTS=1 cargo test --locked --test compile_generated -- --nocapture
cargo test --locked --test compile_generated_workspace -- --nocapture
```

Aceite: suites executam de verdade. Se `compile_generated` pular por cache/dependencia ausente, registre bloqueio.

### Tier 3: Real Anchor And Codama Gate

Valida Anchor/Solana/Codama/pnpm/node reais.

```bash
SUNSCREEN_REAL_TOOLCHAIN=1 bash scripts/integration-heavy.sh
cargo test --locked --test integration_anchor -- --ignored --nocapture
SUNSCREEN_FRONTEND_COMPILE_TESTS=1 cargo test --locked --test generate generated_frontend_hooks_typecheck_vanilla_next_project_when_dependencies_are_installed -- --ignored --nocapture
```

Aceite: `anchor`, `solana`, `pnpm`, `node`, `cargo`, `rustc` e `codama` foram encontrados; os testes ignored executaram cenarios reais e nao apenas imprimiram SKIP.

### Tier 4: Pinocchio SBF Gate

Valida Pinocchio com Solana SBF real.

```bash
cargo build --locked
tmp="$(mktemp -d)"
./target/debug/sunscreen chain new real_pin --framework pinocchio --frontend none --path "$tmp/real_pin"
(cd "$tmp/real_pin" && "$OLDPWD/target/debug/sunscreen" --json chain build --headless)
```

Aceite: `cargo build-sbf` real executa no workspace Pinocchio e Anchor-only guards continuam sem mutacao.

### Tier 5: Serve Runtime Gate

Valida runtime, watcher e teardown com Surfpool/test-validator quando a maquina tiver a toolchain.

```bash
cargo test --locked --test chain_serve -- --nocapture
cargo test --locked --test runtime_serve_loop --test runtime_watch_loop --test runtime_validator -- --nocapture
```

Aceite: runtime real sobe, portas ficam prontas quando verificaveis, watcher dispara build, eventos NDJSON sao parseaveis e Ctrl-C encerra filhos.

### Tier 6: Plugin Runtime Gate

Valida runtime, watcher, plugin lifecycle e comandos dinamicos.

```bash
cargo test --locked --test app_lifecycle -- --nocapture
cargo test --locked plugin::stdio plugin::grpc plugin::sandbox plugin::manifest
./target/release/sunscreen app marketplace --json
```

Aceite: plugin local executa, sandbox rejeita traversal, app/scaffold dinamico mantem exit codes, e gRPC e reportado como contrato/stub se ainda nao tiver runtime real.

### Tier 7: Release And Install Gate

Valida o binario que usuarios baixariam.

```bash
cargo build --locked --release --all-features
./target/release/sunscreen --help
./target/release/sunscreen version
SUNSCREEN_DIST=1 bash scripts/integration-heavy.sh
cargo dist plan
```

Aceite: release binary funciona, dist plan corresponde aos targets esperados, changelog/notas/docs estao coerentes. Nao crie tag/release sem pedido explicito.

### Tier 8: Flake And Performance Gate

Reexecuta suites criticas e mede cold-start.

```bash
SUNSCREEN_FLAKE_RUNS=5 bash scripts/integration-heavy.sh
RUNS=30 bash scripts/bench.sh
```

Aceite: nenhuma falha intermitente; cold-start p95 continua dentro do alvo documentado ou regressao fica reportada.

## Standard Runner

Prefira o runner unico para rodadas locais:

```bash
bash scripts/integration-heavy.sh
SUNSCREEN_COMPILE_TESTS=1 bash scripts/integration-heavy.sh
SUNSCREEN_REAL_TOOLCHAIN=1 SUNSCREEN_COMPILE_TESTS=1 bash scripts/integration-heavy.sh
SUNSCREEN_REAL_TOOLCHAIN=1 SUNSCREEN_DIST=1 SUNSCREEN_FLAKE_RUNS=5 bash scripts/integration-heavy.sh
```

Variaveis:

- `SUNSCREEN_COMPILE_TESTS=1`: liga compile tests gated.
- `SUNSCREEN_REAL_TOOLCHAIN=1`: exige toolchain real e roda `integration_anchor --ignored`.
- `SUNSCREEN_PINOCCHIO_SBF=1`: exige Solana/Cargo SBF e roda build Pinocchio real.
- `SUNSCREEN_DIST=1`: exige `cargo dist` e roda `cargo dist plan`.
- `SUNSCREEN_FLAKE_RUNS=N`: repete o smoke de CLI `N` vezes.
- `SUNSCREEN_HEAVY_LOG_DIR=path`: muda o diretorio de logs.

## Reporting

Relate sempre:

- Comandos executados.
- Versoes de ferramentas reais.
- Tiers aprovados, falhos, skipped e blocked.
- Evidencia de que testes ignored/gated executaram de verdade.
- Arquivos/logs em `_workspace/test-harness/`.
- `*.summary.json` da rodada, com status por tier.
- Proximo menor passo para transformar bloqueio em cobertura real.

## False Green Rules

- `#[ignore]` + `--ignored` nao e cobertura real se o corpo imprimiu `SKIP`.
- Fake `PATH` cobre contrato offline, nao comportamento real de Anchor/Solana.
- `cargo test --all` pode esconder suites gated por env var; registre isso explicitamente.
- `compile_generated_workspace` usa shims locais; ele nao substitui dependencias reais de Anchor/Pinocchio.
- `cargo dist plan` local nao equivale a release publicada.
- gRPC de plugin pode estar coberto como contrato/stub; nao chame isso de transporte real sem fixture runtime.
- `doctor --json` reportando tool ausente e diagnostico, nao falha do CLI.

## Test Scenarios

Normal:

1. Usuario pede "validar tudo com testes pesados".
2. Rode `bash scripts/integration-heavy.sh`.
3. Se o usuario quer real toolchain, rode com `SUNSCREEN_REAL_TOOLCHAIN=1`.
4. Entregue relatorio por tier.

Error flow:

1. `SUNSCREEN_REAL_TOOLCHAIN=1` falha porque `anchor` ou `codama` nao existe.
2. Marque `blocked_by_missing_tool`.
3. Nao chame a rodada de verde; proponha instalar/provisionar a toolchain ou mover esse tier para runner dedicado.
