---
name: sunscreen-orchestrator
description: Orquestra o time de implementação do CLI sunscreen (Rust + Solana tooling). Use sempre que o usuário pedir para implementar, continuar, expandir, corrigir, refatorar, atualizar, revisar, reexecutar ou completar qualquer parte do sunscreen CLI — incluindo "próxima fase", "pendências", "R5", "Phase 2", "Phase 3", "chain serve", "chain build", "scaffold", "doctor", "markers", "rodar de novo", "corrigir", "atualizar roadmap" ou trabalho contínuo no projeto. Coordena cli-architect, config-engineer, toolchain-detector, template-engineer, docs-writer e qa-integrator. Não use para perguntas conceituais simples sobre Solana — só para mudanças concretas no codebase sunscreen.
---

# Sunscreen Orchestrator

Coordena a implementação do CLI `sunscreen` (Rust, inspirado em Ignite CLI, alvo: ecossistema Solana). Fonte de verdade viva: `ROADMAP.md`. `docs/adr/ADR-0001-solis-cli.md` e `IMPLEMENTATION-KICKOFF.md` são contexto histórico; preserve as decisões estratégicas, mas traduza Go/solis para Rust/sunscreen.

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
- Phase 2 está em R5 polish. Antes de iniciar Phase 3, feche ou registre explicitamente as pendências R5.
- Phase 3 (Runtime Orchestration: `chain build`, `chain serve`, watcher, runtime e TUI) é a próxima fase planejada, mas depende do fechamento da Phase 2.

Escopo padrão para pedidos como "seguir próximos passos", "próxima fase" ou "pendências":

1. Auditar R5 da Phase 2.
2. Implementar uma fatia testável da R5 com TDD.
3. Atualizar `ROADMAP.md` e `CLAUDE.md` quando o estado mudar.
4. Só então abrir uma fatia inicial de Phase 3.

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
- Non-appendable recovery: `chain doctor --fix-markers` deve reparar sites seguros sem escrever Rust inválido; `dispatch` deve ser reconstruído dentro de `#[program]` quando houver instruções suficientes, e `error_variants` deve ser reembrulhado dentro de `#[error_code] pub enum` preservando variantes existentes.
- Aumentar cobertura gradualmente rumo às metas R5: >=75 golden, >=25 compile e >=5 integração. Registre gaps sem mascarar como concluídos.

### Phase 3 opening checklist

Só iniciar Phase 3 depois que R5 estiver em estado claro. Primeira fatia recomendada:

1. `chain build` headless e JSON-first, reusando workspace discovery.
2. `src/runtime/` com trait mínimo e runner de subprocesso testável.
3. Depois `watcher` e `serve`; TUI ratatui vem após o caminho headless funcionar.

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
