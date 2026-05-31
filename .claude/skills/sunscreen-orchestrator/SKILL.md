---
name: sunscreen-orchestrator
description: Orquestra o time de implementação do CLI sunscreen (Rust + Solana tooling). Use sempre que o usuário pedir para implementar/continuar/expandir/corrigir/refatorar/atualizar/revisar qualquer parte do sunscreen CLI — incluindo "começar implementação", "phase 0", "adicionar scaffold X", "rodar de novo", "corrigir doctor", "atualizar config", ou trabalho contínuo no projeto. Coordena cli-architect, config-engineer, toolchain-detector, template-engineer e qa-integrator em paralelo via TeamCreate quando possível, ou via Agent calls com run_in_background. Não use para perguntas conceituais simples sobre Solana — só para mudanças concretas no codebase sunscreen.
---

# Sunscreen Orchestrator

Coordena a implementação do CLI `sunscreen` (Rust, inspirado em Ignite CLI, alvo: ecossistema Solana). Fonte de verdade de design: `ADR-0001-solis-cli.md` (substituir "Go"→"Rust" e "solis"→"sunscreen"). Roadmap tático: `IMPLEMENTATION-KICKOFF.md`.

## Phase 0: Contexto

Antes de qualquer ação, determine o modo de execução:

1. `ls _workspace/ 2>/dev/null` — existe?
2. Se **não existe** → modo **inicial**. Crie `_workspace/`. Vá para Phase 1.
3. Se **existe** + usuário pediu mudança específica (ex: "corrige doctor", "adiciona campo X") → modo **parcial**. Releia `_workspace/done_*.md` e `_workspace/qa_final.md`. Acione só os agentes afetados.
4. Se **existe** + usuário pediu reinício completo → modo **novo**. Mova `_workspace/` para `_workspace_prev_<timestamp>/` (timestamp passado via args, não use Date.now). Vá para Phase 1.

## Phase 1: Plano

Identifique o escopo dentro do roadmap (Phase 0 Week 1? Week 2? Scaffold de instrução? Plugin system?). Atualmente, escopo padrão para primeira execução = **Phase 0 Week 1 completa** (cli skeleton + config + doctor + templates + golden infra).

## Phase 2: Execução paralela (modo hybrid)

**Modo de execução: hybrid.**

### Stage A — Fan-out paralelo (Agent calls, run_in_background)

Dispare em paralelo, **um único bloco com 4 Agent calls simultâneos**:
- `cli-architect` (subagent_type: cli-architect, model: opus)
- `config-engineer` (subagent_type: config-engineer, model: opus)
- `toolchain-detector` (subagent_type: toolchain-detector, model: opus)
- `template-engineer` (subagent_type: template-engineer, model: opus)

Cada um recebe: escopo do round, instrução de coordenação via `_workspace/` (markers), e a referência ao ADR/kickoff.

### Stage B — Barreira

Aguarde os 4 sinalizarem (`_workspace/done_<agent>.md`). Resolva conflitos no `Cargo.toml` se houver (merge manual de seções `[dependencies]`).

### Stage C — QA integrador

Dispare `qa-integrator` (model: opus). Se reportar defeitos em `_workspace/qa_report_*.md`:
- Para cada defeito, re-dispare o agente responsável com o report como input.
- Loop até `qa_final.md` indicar green ou após 3 rounds (escalar ao usuário).

## Phase 3: Relatório

Resuma ao usuário:
- Arquivos criados (lista compacta agrupada por módulo)
- `cargo build` status
- `cargo test` resumo (n passed)
- Comandos verificados manualmente
- Próximo passo sugerido do roadmap

## Data Protocol

- **Workspace**: `_workspace/` na raiz do projeto.
- **Sinais**: `_workspace/done_<agent>.md` quando agente termina.
- **QA**: `_workspace/qa_report_<round>.md`, `_workspace/qa_final.md`.
- **Conflitos de Cargo.toml**: cli-architect é o owner, demais agentes propõem deps via `_workspace/deps_<agent>.toml`.

## Error Handling

- Agente falha → 1 retry com error message como input → se falhar de novo, marcar `_workspace/failed_<agent>.md` e seguir sem ele (reportar ao usuário).
- QA falha persistente (3 rounds) → escalar.
- Conflito de design entre agentes → orquestrador decide; registra em `_workspace/decisions.md`.

## Test Scenarios

**Normal**: usuário diz "implementa Phase 0 Week 1" → 4 paralelos + QA + report. Resultado: `cargo build` passa, `sunscreen --help`, `sunscreen version`, `sunscreen doctor --json` funcionam.

**Erro**: `toolchain-detector` falha porque `Config::toolchain` não existe ainda → QA detecta no round 1 → SendMessage para `config-engineer` adicionar campo → re-run toolchain-detector → green.
