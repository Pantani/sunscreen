---
name: sunscreen-orchestrator
description: Orquestra o time de implementação do CLI sunscreen (Rust + Solana tooling). Use sempre que o usuário pedir para implementar, continuar, expandir, corrigir, refatorar, atualizar, revisar, reexecutar ou completar qualquer parte do sunscreen CLI — incluindo "próxima fase", "pendências", "R5", "Phase 2", "Phase 3", "chain serve", "chain build", "scaffold", "doctor", "markers", "rodar de novo", "corrigir", "atualizar roadmap" ou trabalho contínuo no projeto. Coordena cli-architect, config-engineer, toolchain-detector, template-engineer, docs-writer e qa-integrator. Não use para perguntas conceituais simples sobre Solana — só para mudanças concretas no codebase sunscreen.
---

# Sunscreen Orchestrator

Coordena a implementação do CLI `sunscreen` (Rust, inspirado em Ignite CLI, alvo: ecossistema Solana). Fonte de verdade viva: `ROADMAP.md`. `docs/adr/ADR-0001-solis-cli.md` e `IMPLEMENTATION-KICKOFF.md` é contexto histórico; preserve as decisões estratégicas, mas traduza Go/solis para Rust/sunscreen.

## Phase 0: Contexto

Antes de qualquer ação, determine o modo de execução:

1. `ls _workspace/ 2>/dev/null` — existe?
2. Se **não existe** → modo **inicial**. Crie `_workspace/`. Vá para Phase 1.
3. Se **existe** + usuário pediu mudança específica (ex: "corrige doctor", "adiciona campo X") → modo **parcial**. Releia `_workspace/done_*.md` e `_workspace/qa_final.md`. Acione só os agentes afetados.
4. Se **existe** + usuário pediu reinício completo → modo **novo**. Mova `_workspace/` para `_workspace_prev_<timestamp>/` (timestamp passado via args, não use Date.now). Vá para Phase 1.
5. Releia `CLAUDE.md`, `ROADMAP.md` e o `git status` antes de tocar em código. Se houver divergência entre o harness e o roadmap vivo, sincronize o harness primeiro.

## Phase 1: Plano

Identifique o escopo dentro de `ROADMAP.md`. Estado atual do projeto:

- Phase 0 e Phase 1 estão concluídas.
- Phase 2 está concluída via PR #7; o único carry-over ativo é o reparo pequeno de empty-account instruction.
- Phase 3 (Runtime Orchestration: `chain build`, `chain serve`, watcher, runtime e TUI) está em fatias iniciais.

Escopo padrão para pedidos como "seguir próximos passos", "próxima fase" ou "pendências":

1. Confirmar se há carry-over de Phase 2 que bloqueia a fatia.
2. Implementar uma fatia testável de Phase 3 com TDD.
3. Atualizar `ROADMAP.md` e `CLAUDE.md` quando o estado mudar.
4. Manter `chain serve --headless` como caminho primário antes da TUI.

## Phase 2: Execução

**Modo de execução: hybrid.**

Use subagentes somente quando o ambiente disponibilizar essa capacidade e o pedido autorizar trabalho por harness/equipe. No Codex, prefira `multi_agent_v1.spawn_agent` com agentes `explorer`/`worker`; não force nomes de modelo incompatíveis com o runtime. Se não houver subagentes, execute localmente seguindo os mesmos donos abaixo.

### Donos por área

- `cli-architect`: `src/cli/**`, contratos de comando, exit codes.
- `config-engineer`: `src/config/**`, schemas, migrações.
- `toolchain-detector`: `src/toolchain/**`, `sunscreen doctor`.
- `template-engineer`: `src/templates/**`, `templates/**`, golden tests e marker templates.
- `docs-writer`: ADRs, `ROADMAP.md`, docs de referência.
- `qa-integrator`: verificação cruzada, fmt/clippy/build/test e comandos do binário.

### R5 checklist

- `tests/rustfmt_roundtrip.rs`: roda `rustfmt --edition=2021` sobre fixture com todos os segmentos documentados e reescaneia os markers.
- Non-appendable recovery: `chain doctor --fix-markers` deve reparar sites seguros sem escrever Rust inválido; `dispatch` deve ser reconstruído dentro de `#[program]` quando o segmento inteiro tiver sumido e houver instruções suficientes, e `error_variants` só deve inserir markers quando o enum estiver vazio ou a região marcada existente for inequívoca.
- Compile tests offline: `tests/compile_generated.rs` cobre 25 cenários com `cargo check` em workspaces gerados.
- Golden tests: `tests/golden/render_{account,event,error,instruction_matrix,program_matrix}.rs` mantêm a cobertura de scaffolders.
- Integrações reais: `tests/integration_anchor.rs` contém 5 cenários `anchor build`/IDL/codama, `#[ignore]` e skip quando a toolchain está ausente.

### Phase 3 opening checklist

Estado atual de Phase 3:

1. `chain build --headless` JSON-first já existe, reusa workspace discovery e chama `anchor build`.
2. `src/runtime/subprocess.rs` já existe com trait mínimo e runner de subprocesso testável.
3. `src/runtime/pipeline.rs` já existe com build → codama e runner injetável; `chain build` usa esse pipeline e aceita `--no-codama`.
4. `src/runtime/watcher.rs` já existe com debounce determinístico testável para mudanças Rust/config relevantes, dedupe/sort de caminhos e filtro para saídas geradas/unrelated.
5. `WatchBuildLoop` já conecta `notify::Event` → debounce → `BuildPipeline`, relativizando paths absolutos pelo workspace antes de filtrar.
6. `src/runtime/serve.rs` e `chain serve --headless` já existem: o comando instancia `notify`, recebe eventos, faz tick do debounce e emite JSON para builds acionados pelo watcher.
7. `src/runtime/surfpool.rs`, `testvalidator.rs`, `validator.rs` e `supervisor.rs` já existem: `Runtime` trait, endpoints, comandos de Surfpool/test-validator e supervisor mínimo start/stop com spawner injetável.
8. Próxima fatia recomendada: integrar o `RuntimeSupervisor` ao `chain serve --headless`, com seleção/fallback de runtime e eventos JSON de start/stop.
9. Depois frontend notify e TUI ratatui; teardown Ctrl-C deve fechar o ciclo da Phase 3.

## Phase 3: Relatório

Resuma ao usuário:
- Arquivos criados (lista compacta agrupada por módulo)
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build`, `cargo test` status
- Comandos verificados manualmente
- Pendências remanescentes do roadmap
- Próximo passo sugerido

## Data Protocol

- **Workspace**: `_workspace/` na raiz do projeto.
- **Sinais**: `_workspace/done_<agent>.md` quando agente termina.
- **QA**: `_workspace/qa_report_<round>.md`, `_workspace/qa_final.md`.
- **Conflitos de Cargo.toml**: cli-architect é o owner, demais agentes propõem deps via `_workspace/deps_<agent>.toml`.

## Error Handling

- Agente ou etapa falha → 1 retry com error message como input → se falhar de novo, marcar `_workspace/failed_<agent>.md` ou reportar explicitamente o bloqueio.
- QA falha persistente (3 rounds) → escalar.
- Conflito de design entre agentes → orquestrador decide; registra em `_workspace/decisions.md`.

## Test Scenarios

**Normal**: usuário diz "seguir próximos passos e pendências" → auditar `ROADMAP.md`, fechar uma fatia R5, rodar QA e reportar se Phase 3 está desbloqueada.

**Erro**: `rustfmt` não está disponível → teste R5 deve pular/relatar de forma explícita, e o relatório final deve dizer que a garantia de rustfmt não foi validada localmente.
