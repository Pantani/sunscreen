---
name: sunscreen-orchestrator
description: Orquestra o time de implementação do CLI sunscreen (Rust + Solana tooling). Use sempre que o usuário pedir para implementar, continuar, expandir, corrigir, refatorar, atualizar, revisar, reexecutar ou completar qualquer parte do sunscreen CLI — incluindo "próxima fase", "pendências", "Phase 3", "Phase 4", "Phase 5", "Phase 5.5", "chain serve", "chain build", "generate", "codama", "scaffold", "recipes", "crud", "spl-token", "metaplex-nft", "onboarding", "quickstart", "doctor", "markers", "rodar de novo", "corrigir", "atualizar roadmap" ou trabalho contínuo no projeto. Coordena cli-architect, config-engineer, toolchain-detector, template-engineer, docs-writer e qa-integrator. Não use para perguntas conceituais simples sobre Solana — só para mudanças concretas no codebase sunscreen.
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

- Phase 0, Phase 1, Phase 2, Phase 3, Phase 4 e Phase 5 estão concluídas.
- Phase 2 está concluída e sem carry-overs conhecidos.
- Phase 3 está concluída: `chain build`, `chain serve`, watcher, runtime supervisionado, fallback Surfpool→test-validator, frontend notify, serve model e teardown Ctrl-C.
- Phase 4 está concluída: `generate {clients, idl, frontend-hooks}`, wrapper Codama, export IDL determinístico, React/Solid Query hooks e pipeline compartilhado.
- Phase 5 está concluída: `scaffold {crud, spl-token, metaplex-nft}` como receitas compostas sobre os scaffolders Phase 2.
- Phase 5.5 (Onboarding Layer) é a próxima fase.

Escopo padrão para pedidos como "seguir próximos passos", "próxima fase" ou "pendências":

1. Confirmar que Phase 2/3/4/5 continuam fechadas no `ROADMAP.md` e no `git status`.
2. Implementar uma fatia testável de Phase 5.5 com TDD.
3. Atualizar `ROADMAP.md` e `CLAUDE.md` quando o estado mudar.
4. Manter `quickstart` como wrapper sobre recipes Phase 5, sem duplicar scaffolders ou codegen.

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

### Phase 2 closure checklist

- `tests/rustfmt_roundtrip.rs`: roda `rustfmt --edition=2021` sobre fixture com todos os segmentos documentados e reescaneia os markers.
- Non-appendable recovery: `chain doctor --fix-markers` deve reparar sites seguros sem escrever Rust inválido; `dispatch` deve ser reconstruído dentro de `#[program]` quando o segmento inteiro tiver sumido e houver instruções suficientes, e `error_variants` só deve inserir markers quando o enum estiver vazio ou a região marcada existente for inequívoca.
- Compile tests offline: `tests/compile_generated.rs` cobre 25 cenários com `cargo check` em workspaces gerados.
- Golden tests: `tests/golden/render_{account,event,error,instruction_matrix,program_matrix}.rs` mantêm a cobertura de scaffolders.
- Integrações reais: `tests/integration_anchor.rs` contém 5 cenários `anchor build`/IDL/codama, `#[ignore]` e skip quando a toolchain está ausente.

### Phase 3 closure checklist

Phase 3 deve permanecer fechada quando estes itens estiverem verdes:

1. `chain build --headless` JSON-first reusa workspace discovery e chama `anchor build`.
2. `src/runtime/subprocess.rs` já existe com trait mínimo e runner de subprocesso testável.
3. `src/runtime/pipeline.rs` faz build → codama → frontend notify e aceita `--no-codama`/`--no-frontend` nos comandos relevantes.
4. `src/runtime/watcher.rs` tem debounce determinístico testável para mudanças Rust/config relevantes, dedupe/sort de caminhos e filtro para saídas geradas/unrelated.
5. `WatchBuildLoop` já conecta `notify::Event` → debounce → `BuildPipeline`, relativizando paths absolutos pelo workspace antes de filtrar.
6. `src/runtime/serve.rs` e `chain serve --headless` instanciam `notify`, recebem eventos, fazem tick do debounce e emitem JSON para builds acionados pelo watcher.
7. `src/runtime/surfpool.rs`, `testvalidator.rs`, `validator.rs` e `supervisor.rs` cobrem `Runtime` trait, endpoints, comandos de Surfpool/test-validator e supervisor start/stop com spawner injetável.
8. `chain serve` integra runtime supervisor com seleção/fallback e eventos JSON de start/stop.
9. `src/tui/serve_model.rs` cobre os painéis validator/build/faucet/frontend/logs e guarda 80x24.
10. Ctrl-C para o runtime como process group Unix, com fallback SIGKILL depois de SIGTERM.

### Phase 4 closure checklist

- `src/codegen/{codama,codama_config,idl,frontend_hooks}.rs` existe e é usado pelo CLI.
- `sunscreen generate clients`, `generate idl` e `generate frontend-hooks` estão implementados.
- `chain build` e `chain serve` reutilizam o wrapper Codama compartilhado.
- Hooks React Query e Solid Query são determinísticos e cobertos por testes.

### Phase 5 closure checklist

- `sunscreen scaffold crud <Resource> --program <p>` gera state, `create/read/update/delete`, events, errors, teste TS e hook opcional de frontend.
- `sunscreen scaffold spl-token <Name> --program <p>` gera slice SPL token interno.
- `sunscreen scaffold metaplex-nft <Name> --program <p>` gera slice Token Metadata interno.
- Recipes fazem preflight dry-run dos primitives antes de escrever e mantêm um único objeto JSON sob `--json`.
- `docs/reference/recipes.md`, `ROADMAP.md`, `AGENTS.md` e `CLAUDE.md` refletem Phase 5 fechada.

### Phase 5.5 opening checklist

1. Implementar `init`/wizard e `--non-interactive` sem duplicar `chain new`.
2. Implementar `quickstart {token,nft,dao,blog}` como wrappers sobre recipes Phase 5.
3. Adicionar `examples`, `wallet`, `deploy`, `learn` e contrato `next_step` em erros.
4. Atualizar ADR-0002 exit-code table somente quando Phase 5.5 adicionar `PathConflict`/`Network`.

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

**Normal**: usuário diz "seguir próximos passos e pendências" → auditar `ROADMAP.md`, fechar uma fatia Phase 5.5, rodar QA e reportar o que ainda bloqueia v1.0.

**Erro**: toolchain Solana/Anchor não está disponível → testes reais ignorados/gated devem pular ou relatar de forma explícita, e o relatório final deve dizer qual garantia não foi validada localmente.
