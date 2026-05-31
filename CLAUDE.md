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
