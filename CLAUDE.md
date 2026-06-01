# Sunscreen — CLI de tooling para Solana (Rust)

Greenfield Rust CLI inspirado em Ignite CLI. Escopo: scaffolding incremental de programas Anchor 1.0, orquestração do dev loop (Surfpool + Codama + frontend), e plugin system. Fonte de verdade de design: `docs/adr/ADR-0001-solis-cli.md` (com nome adaptado de "solis" → "sunscreen" e linguagem Go → Rust). Roadmap tático: `IMPLEMENTATION-KICKOFF.md`.

## Harness: sunscreen

**Objetivo:** Implementar e evoluir o CLI sunscreen usando time paralelo de agentes especializados.

**Trigger:** Qualquer pedido de implementação, expansão, correção ou refatoração do CLI sunscreen → invoque o skill `sunscreen-orchestrator`. Perguntas conceituais sobre Solana/Anchor podem ser respondidas diretamente.

**Variação chave do ADR:** o ADR fala em Go/solis; este projeto é Rust/sunscreen. Mantenha decisões estratégicas (Anchor IDL como fonte de verdade, marker-based editing, plugin protocol, etc.) mas troque toda referência de stack para Rust (clap, serde, minijinja, rust-embed, tokio, insta).

**Variation log:**
| Data | Mudança | Alvo | Motivo |
|------|---------|------|--------|
| 2026-05-31 | Configuração inicial do hareness | agents/* + skills/sunscreen-orchestrator | bootstrap |
| 2026-05-31 | Phase 0 Week 1 implementada | src/{cli,config,toolchain,templates,error} | 16/16 testes verdes |
| 2026-05-31 | docs-writer agent adicionado | agents/docs-writer.md | ADRs Week 2 |
| 2026-05-31 | Phase 0 Week 2 implementada | .github/, docs/adr/0002-0003, benches/, migration v0→v1 | 22/22 testes, cold-start 3.18ms |
| 2026-05-31 | Phase 1 (Workspace Bootstrap) implementada | src/fsutil/, src/cli/chain.rs, templates/workspace/, Config v1 expandido, preflight | 59/59 testes, `chain new` E2E funcional |
| 2026-05-31 | Phase 2 R1 (rustpatch + scaffold instruction) implementada | src/rustpatch/, src/workspace/, src/cli/scaffold.rs, src/templates/instruction.rs, templates/scaffold/instruction/, docs/reference/markers.md, docs/adr/ADR-0004 | 96/96 testes, idempotência+drift detection, fmt/clippy clean. Carry-over R2: `dispatch` segment em `lib.rs.j2` |
| 2026-05-31 | Phase 2 R2 dispatch carry-over | templates/workspace/anchor-multiple/programs/__program__/src/lib.rs.j2, templates/workspace/anchor-multiple/programs/__program__/src/instructions/mod.rs, tests/scaffold_instruction.rs, tests/golden/snapshots/* | Workspace recém-criado já vem com `segment=dispatch` em `lib.rs` e `segment=instructions` em `instructions/mod.rs`; primeiro `scaffold instruction` patcha sem warning (`lib_rs_patched=true`). 103/103 testes, snapshots golden atualizados. |
| 2026-05-31 | Phase 2 R3 (account/event/error scaffolders) | src/cli/scaffold.rs, src/templates/{account,event,error}.rs, templates/scaffold/{account,event,error}/*, tests/scaffold_{account,event,error}.rs | 3 scaffolders restantes da Phase 2 entregues com mesma arquitetura do instruction (idempotência, --dry-run, --json, conflito → exit 4 user_input). Generator tag `account`/`event`/`error` em todos markers (D2 fix). Account conflict agora retorna erro claro (D3 fix). 115/115 testes, fmt+clippy clean. **Carry-over R4:** `program` scaffolder + `chain doctor --fix-markers` + auto-inclusão de `pub mod events/errors/state` em `lib.rs`. |
| 2026-06-01 | ADR-0005 (Beginner Onboarding Surface) proposto | docs/adr/ADR-0005-beginner-onboarding.md | Formaliza Phase 5.5 — Onboarding Layer (init wizard, examples, quickstart, wallet, deploy, learn, erros acionáveis). Status: Proposed. DoD: novato → NFT em devnet em < 10 min. PR #6. |
