---
name: sunscreen-orchestrator
description: Orquestra o time de implementação do CLI sunscreen (Rust + Solana tooling). Use sempre que o usuário pedir para implementar, continuar, expandir, corrigir, refatorar, atualizar, revisar, reexecutar ou completar qualquer parte do sunscreen CLI — incluindo "próxima fase", "pendências", "Phase 2", "Phase 3", "Phase 4", "chain serve", "chain build", "generate", "codama", "scaffold", "doctor", "markers", "rodar de novo", "corrigir", "atualizar roadmap" ou trabalho contínuo no projeto. Coordena cli-architect, config-engineer, toolchain-detector, template-engineer, docs-writer e qa-integrator. Não use para perguntas conceituais simples sobre Solana — só para mudanças concretas no codebase sunscreen.
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

- Phase 0, Phase 1 e Phase 2 estão concluídas.
- Phase 2 não tem carry-overs conhecidos: marker hardening, no-accounts instruction compile test e R5 polish estão fechados.
- Phase 3 está concluída neste PR: `chain build`, `chain serve`, watcher, runtime supervisionado, fallback Surfpool→test-validator, frontend notify, serve model e teardown Ctrl-C.
- Phase 4 (Codegen & Frontend Hooks) é a próxima fase.

## Execução

**Modo de execução: hybrid.** Use subagentes somente quando o ambiente disponibilizar essa capacidade; sem subagentes, execute localmente seguindo os donos abaixo.

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

- `chain build --headless` emite NDJSON e roda build → Codama.
- Watcher faz debounce e aciona pipeline com paths relativos.
- `chain serve` lança runtime Surfpool/test-validator com fallback quando Surfpool implícito está ausente.
- Frontend notify toca `app/.sunscreen/reload`.
- `src/tui/serve_model.rs` cobre painéis validator/build/faucet/frontend/logs e 80x24.
- Ctrl-C para process group Unix com fallback SIGKILL.

### Phase 4 opening

1. Começar por `src/codegen/codama.rs` e `codama_config.rs`, reutilizando `CommandSpec`/`ProcessRunner`.
2. Adicionar `sunscreen generate clients` e `generate idl` antes de hooks de frontend.
3. Manter Codama como subprocesso fino (`pnpm exec codama run`) até a API exigir wrapper mais rico.
4. Só depois adicionar `generate frontend-hooks`, com compile test em projeto Next.js/Vite gerado.

## Relatório

Resuma ao usuário:

- Arquivos criados/alterados agrupados por módulo.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build`, `cargo test` status.
- Pendências remanescentes do roadmap.
- Próximo passo sugerido.

## Error Handling

- Etapa falha → 1 retry com a mensagem de erro.
- Repetiu → reporte bloqueio com comando, saída e arquivo provável.
- Conflito de design → preserve alternativas no relatório e escolha o caminho que mantém `ROADMAP.md` coerente.
