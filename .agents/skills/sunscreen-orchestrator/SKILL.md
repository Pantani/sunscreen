---
name: sunscreen-orchestrator
description: Orquestra o time de implementação do CLI sunscreen (Rust + Solana tooling). Use sempre que o usuário pedir para implementar, continuar, expandir, corrigir, refatorar, atualizar, validar, revisar, reexecutar ou completar qualquer parte do sunscreen CLI — incluindo "próxima fase", "pendências", "Phase 6", "plugins", "app", "marketplace", "stdio", "gRPC", "Phase 8", "CI", "integration", "chain serve", "chain build", "generate", "codama", "scaffold", "recipes", "crud", "spl-token", "metaplex-nft", "onboarding", "quickstart", "doctor", "markers", "rodar de novo", "corrigir", "atualizar roadmap" ou trabalho contínuo no projeto. Coordena cli-architect, config-engineer, toolchain-detector, template-engineer, docs-writer e qa-integrator. Não use para perguntas conceituais simples sobre Solana — só para mudanças concretas no codebase sunscreen.
---

# Sunscreen Orchestrator

Coordena a implementação do CLI `sunscreen` (Rust, inspirado em Ignite CLI, alvo: ecossistema Solana). Fonte de verdade viva: `ROADMAP.md`. `docs/adr/ADR-0001-solis-cli.md` e `IMPLEMENTATION-KICKOFF.md` são contexto histórico; preserve as decisões estratégicas, mas traduza Go/solis para Rust/sunscreen.

## Phase 0: Contexto

Antes de qualquer ação:

1. Releia `CLAUDE.md`, `AGENTS.md`, `ROADMAP.md` e `git status`.
2. Trate `ROADMAP.md` como fonte viva de escopo/status.
3. Se houver drift entre harness, AGENTS/CLAUDE e roadmap, sincronize no mesmo PR.
4. Preserve mudanças locais do usuário.

## Estado Atual

- Phase 0, Phase 1, Phase 2, Phase 3, Phase 4 e Phase 5 estão concluídas.
- Phase 2 não tem carry-overs conhecidos: marker hardening, no-accounts instruction compile test e R5 polish estão fechados.
- Phase 3 está concluída: `chain build`, `chain serve`, watcher, runtime supervisionado, fallback Surfpool→test-validator, frontend notify, serve model e teardown Ctrl-C.
- Phase 4 está concluída: `generate {clients, idl, frontend-hooks}`, wrapper Codama, export IDL determinístico, React/Solid Query hooks e pipeline compartilhado.
- Phase 5 está concluída: `scaffold {crud, spl-token, metaplex-nft}` como receitas compostas sobre os scaffolders Phase 2.
- Phase 5.5 está concluída: `init`, `examples`, `quickstart`, `wallet`, `deploy`, `learn` e erros com `next_step`.
- Phase 6 está concluída: lifecycle `app`, manifesto `sunscreen-plugin.json`, runtime manager, stdio JSON-RPC, contrato gRPC, sandbox/trust model, marketplace local/reference, hooks e comando dinâmico `scaffold <noun>`.
- Phase 8 (Distribution & Docs / v1.0) é a próxima fase: cargo-dist multi-OS completo, docs site, shell completions, changelog/SemVer e release polish.
- A camada Ignite-style de integração CLI já existe e roda no CI: `tests/integration_{chain,scaffold,generate,onboarding}.rs` com `tests/support/mod.rs`.
- O CI principal já tem smoke explícito de integração, `--locked`, check `--no-default-features`, permissões read-only, concorrência e timeouts.

## Execução

**Modo de execução: hybrid.** Use subagentes somente quando o ambiente disponibilizar essa capacidade e o pedido autorizar trabalho por harness/equipe. No Codex, prefira `multi_agent_v1.spawn_agent` com agentes de QA/docs/arquitetura; sem subagentes, execute localmente seguindo os mesmos donos abaixo.

### Donos por área

- `cli-architect`: `src/cli/**`, contratos de comando, exit codes.
- `config-engineer`: `src/config/**`, schemas, migrações.
- `toolchain-detector`: `src/toolchain/**`, `sunscreen doctor`.
- `template-engineer`: `src/templates/**`, `templates/**`, golden tests e marker templates.
- `docs-writer`: ADRs, `ROADMAP.md`, docs de referência.
- `qa-integrator`: verificação cruzada, fmt/clippy/build/test e comandos do binário.

## Checklists

### Phase 2 closure

- `tests/rustfmt_roundtrip.rs` preserva todos os segmentos documentados.
- `chain doctor --fix-markers` repara `dispatch` e `error_variants` apenas em casos seguros.
- `tests/compile_generated.rs` cobre 25 cenários de workspaces gerados.
- `tests/integration_anchor.rs` contém 5 cenários reais ignorados por padrão com skip de toolchain.

### Phase 3 closure

- `chain build --headless` emite NDJSON e roda build -> Codama.
- Watcher faz debounce e aciona pipeline com paths relativos.
- `chain serve` lança runtime Surfpool/test-validator com fallback quando Surfpool implícito está ausente.
- Frontend notify toca `app/.sunscreen/reload`.
- `src/tui/serve_model.rs` cobre painéis validator/build/faucet/frontend/logs e 80x24.
- Ctrl-C para process group Unix com fallback SIGKILL.

### Phase 4 closure

- `src/codegen/{codama,codama_config,idl,frontend_hooks}.rs` existe e é usado pelo CLI.
- `sunscreen generate clients`, `generate idl` e `generate frontend-hooks` estão implementados.
- `chain build` e `chain serve` reutilizam o wrapper Codama compartilhado.
- Hooks React Query e Solid Query são determinísticos e cobertos por testes.

### Phase 5 closure

- `sunscreen scaffold crud <Resource> --program <p>` gera state, `create/read/update/delete`, events, errors, teste TS e hook opcional de frontend.
- `sunscreen scaffold spl-token <Name> --program <p>` gera slice SPL token interno.
- `sunscreen scaffold metaplex-nft <Name> --program <p>` gera slice Token Metadata interno.
- Recipes fazem preflight dry-run dos primitives antes de escrever e mantêm um único objeto JSON sob `--json`.
- `docs/reference/recipes.md`, `ROADMAP.md`, `AGENTS.md` e `CLAUDE.md` refletem Phase 5 fechada.

### Phase 5.5 closure

- `init`/wizard e `--non-interactive` reutilizam `chain new`.
- `quickstart {token,nft,dao,blog}` compõe recipes Phase 5.
- `examples`, `wallet`, `deploy`, `learn` e contrato `next_step` estão implementados.
- ADR-0002 cobre `PathConflict` e `Network`.

### Phase 6 closure

- `sunscreen app commands` lista comandos dinâmicos de manifestos locais sem iniciar processos.
- `sunscreen app run <plugin> <command> -- ...` executa comandos `kind=app` via stdio JSON-RPC com framing `Content-Length`.
- `sunscreen scaffold <noun> -- ...` roteia comandos `kind=scaffold` declarados por plugins sem adicionar cada noun ao core.
- `sunscreen app marketplace` lista os plugins de referência `spl-token-2022` (gRPC) e `yellowstone-indexer` (stdio).
- `src/plugin/{manifest,manager,stdio,grpc,sandbox,marketplace}.rs` existe e mantém uma interface interna única para transportes.
- `proto/plugin.proto` define `initialize`, `capabilities`, `run_command`, `run_hook` e `shutdown`.
- Falhas de runtime/sandbox usam exit 9 (`plugin_runtime`); exit 7 continua reservado a `path_conflict`.
- `tests/app_lifecycle.rs` cobre lifecycle + runtime local, falha não-zero, sandbox traversal e dynamic scaffold.

### Phase 8 / CI QA

- CI deve rodar `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo test --locked --all --all-features --no-fail-fast`, `cargo build --locked --release --all-features` e `cargo check --locked --no-default-features --all-targets`.
- O smoke Ignite-style deve rodar explicitamente os quatro grupos `integration_chain`, `integration_scaffold`, `integration_generate` e `integration_onboarding`, além de `app_lifecycle` para o runtime de plugins.
- Testes reais de Anchor/Codama em `tests/integration_anchor.rs` continuam gated/ignored por padrão; quando executados, reporte se validaram de verdade ou apenas pularam por toolchain ausente.
- Phase 8 ainda tem lacunas: docs site no CI, completions, changelog/SemVer, Windows/cargo-dist completo, Homebrew/binstall e validação `cargo dist plan`.

## Relatório

Resuma ao usuário:

- Arquivos criados/alterados agrupados por módulo.
- `cargo fmt --all -- --check`, `cargo clippy --locked --all-targets --all-features -- -D warnings`, `cargo build --locked --release --all-features`, `cargo test --locked --all --all-features --no-fail-fast` status.
- Status do smoke `cargo test --locked --test integration_chain --test integration_scaffold --test integration_generate --test integration_onboarding`.
- Status do feature gate `cargo check --locked --no-default-features --all-targets`.
- Pendências remanescentes do roadmap.
- Próximo passo sugerido.

## Error Handling

- Etapa falha -> 1 retry com a mensagem de erro.
- Repetiu -> reporte bloqueio com comando, saída e arquivo provável.
- Conflito de design -> preserve alternativas no relatório e escolha o caminho que mantém `ROADMAP.md` coerente.
