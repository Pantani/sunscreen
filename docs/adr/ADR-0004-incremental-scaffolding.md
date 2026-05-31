# ADR-0004 — Incremental Scaffolding Strategy for `sunscreen`

| Field | Value |
|---|---|
| **Status** | Proposed |
| **Date** | 2026-05-31 |
| **Authors** | Danilo Lacombe |
| **Tags** | scaffolding, codegen, markers, anchor, rust-mutation |
| **Supersedes** | — |
| **Superseded by** | — |
| **Related** | ADR-0001 § 7.1 e § 8.4 (sunscreen CLI), ADR-0002 (CLI Design Conventions), ADR-0003 (Documentation Strategy), `docs/reference/markers.md` |

---

## TL;DR

`sunscreen` adota **scaffolding incremental por edição baseada em markers** como estratégia primária para mutações em arquivos Rust de workspaces Anchor 1.0 existentes (`lib.rs`, `instructions/mod.rs`, `state/mod.rs`, `errors.rs`, `events.rs`). Cada scaffolder (`instruction`, `account`, `event`, `error`, `program`) opera exclusivamente dentro de regiões delimitadas por comentários `// === sunscreen:auto-generated:begin segment=… ===` (cf. `docs/reference/markers.md`). Código de usuário vive em regiões `user-region` e é tratado como imutável. Para casos raros em que insertion em território não-marcado é inevitável (referenciar uma struct nomeada pelo usuário), `sunscreen` faz fallback para **`ast-grep` como subprocess** — sem dependência de toolchain Rust em tempo de execução. Esta decisão segue Sub-ADR-001 do ADR-0001 § 7.1 e formaliza o roteiro R1→R5 de implementação dos scaffolders na Phase 2.

---

## 1. Context

### 1.1 Problem framing

O diferencial de `sunscreen` versus um simples template-renderer (à la `cargo generate`) é a capacidade de **continuar gerando código depois do bootstrap**. Um usuário roda `sunscreen chain new escrow`, edita os handlers, e depois roda `sunscreen scaffold instruction deposit` — a ferramenta precisa:

1. Adicionar `pub mod deposit;` em `instructions/mod.rs`.
2. Adicionar uma arm em `#[program] pub mod escrow { … }` dentro de `lib.rs`.
3. Criar `instructions/deposit.rs` com `#[derive(Accounts)]` struct + skeleton de `handler`.
4. **Não tocar** em nenhum byte de lógica que o usuário já escreveu nas outras instruções.

Esta restrição (preservar código do usuário) é a tensão central. Anchor 1.0 + IDL como fonte de verdade reduz o problema (a maior parte do código gerado é mecanicamente derivável dos args do CLI), mas não o elimina: `lib.rs` mistura código gerado (dispatch arms) com código que pode ter sido tocado pelo usuário (imports, attributes do `#[program]`).

### 1.2 Restrições

- **Reentrância.** Rodar o mesmo comando de scaffold duas vezes com mesmos args ⇒ no-op binário (sem diff).
- **Segurança.** Não corromper o workspace silenciosamente. Falhas devem ser detectadas antes de qualquer escrita em disco.
- **Sem dependência de toolchain Rust em runtime para edição.** `sunscreen` é um binário standalone; não pode depender de `cargo` ou de uma biblioteca C dinâmica de `tree-sitter`.
- **Anchor 1.0 + IDL primeiro.** A maior parte das mutações é mecanicamente derivável dos args + IDL — não precisamos de AST completo para inserir uma `pub mod` em um lugar conhecido.

---

## 2. Decision Drivers

- **DD1 — Idempotência.** Re-rodar o mesmo scaffold é um no-op.
- **DD2 — Preservação do código do usuário.** Nunca sobrescrever bytes que o usuário escreveu.
- **DD3 — Sem dep runtime pesada.** Nada de linkar `libclang`, `tree-sitter` dinâmico, ou exigir `cargo` na máquina do usuário só para `scaffold`.
- **DD4 — Velocidade.** `scaffold instruction X` deve completar em < 100 ms num projeto de 50 arquivos. Cold-start já está em 3.18 ms (Phase 0 W2).
- **DD5 — Predictability / debuggabilidade.** O usuário deve conseguir abrir o arquivo e ver, literalmente, qual região é da ferramenta e qual é dele. Um comentário `// === sunscreen:auto-generated:begin … ===` é autodocumentado.
- **DD6 — Robustez frente a `rustfmt`.** Qualquer estratégia que use anchors textuais precisa sobreviver à formatação automática.

---

## 3. Considered Options

| # | Opção | Resumo |
|---|---|---|
| (a) | **Starter-only** | `sunscreen chain new` cria projeto e acabou; toda mutação posterior é manual |
| (b) | **Marker-based editing** *(escolhido)* | Regiões delimitadas por comentários estruturados; sunscreen só edita dentro delas |
| (c) | **AST via `tree-sitter-rust` linkado** | Parse CST do arquivo, query AST, emite modificações em código Rust |
| (d) | **`ast-grep` CLI como subprocess** | Mesmo poder de (c), mas via binário externo, regras YAML |

### 3.1 Opção (a) — Starter-only

**Prós:** trivial de implementar; zero risco de corromper código do usuário.
**Contras:** elimina o diferencial competitivo. O valor de Ignite/Cosmos vem do scaffolding incremental — `sunscreen scaffold instruction` deve funcionar no dia 30 do projeto, não só no dia 1.

**Rejeitado.** Reduz o produto a um `cargo generate` com tema Anchor.

### 3.2 Opção (b) — Marker-based editing

**Prós:**
- Implementação simples: scan de linhas para achar `begin`/`end`, substituir miolo, escrever.
- Sem dependência de toolchain Rust em runtime.
- Auto-documentado: o usuário **vê** o que é gerenciado.
- Determinístico, rápido, fácil de testar (golden tests).
- Sobrevive a `rustfmt` (line comments fora de expressões — ADR-0001 § 9.5.1).
- Compõe naturalmente com `user-region` para preservar handlers.

**Contras:**
- Só funciona em arquivos que `sunscreen` gera. Mutação de código user-authored fica fora do alcance.
- Renomes/movimentações de arquivos pelo usuário podem "perder" markers.
- Regex não pode ser usado — match deve ser exact-string por linha.

**Mitigação:** o conjunto de arquivos que `sunscreen` realmente precisa editar é pequeno e canônico (`mod.rs` dos sub-módulos, `lib.rs` dispatch, `errors.rs`, `events.rs`). Tudo o resto é arquivo-por-instrução, criado uma vez e protegido por `user-region` no handler.

### 3.3 Opção (c) — `tree-sitter-rust` linkado

**Prós:** estruturalmente "correto"; entende sintaxe Rust de verdade; não depende de comentários sobreviverem.
**Contras:**
- Binding nativo (`tree-sitter` C lib) adiciona complicação de build cross-platform.
- Parse + query + emit é ≥10× mais lento que line-scan para uma operação trivial (adicionar `pub mod X;`).
- Não resolve o problema de **escolher onde** inserir — ainda precisamos de uma convenção (a string `// instructions go here` ou um comentário marker equivalente). Acabamos reinventando markers só que sem o benefício de auto-documentação.
- Difícil de testar — golden tests viram comparações de CST, não de texto.

**Rejeitado como primário.** AST é overkill para 95% das operações que `sunscreen` precisa fazer.

### 3.4 Opção (d) — `ast-grep` CLI como subprocess

**Prós:** tree-sitter por baixo, mas distribuído como binário standalone; regras em YAML; cobre os 5% de casos onde precisamos referenciar identificadores user-authored.
**Contras:** dependência externa que precisa estar instalada (mitigado: pode ser baixado pelo `sunscreen doctor`); regras YAML são uma DSL a mais para o contribuidor aprender.

**Aceito como escape hatch.** Não primário, mas disponível quando necessário.

---

## 4. Decision

`sunscreen` adota:

1. **Marker-based editing como estratégia primária** para toda mutação de arquivos Rust gerados pelo próprio `sunscreen`. Formato canônico em `docs/reference/markers.md`.
2. **`ast-grep` como subprocess de escape hatch** para o caso raro de inserção em território user-authored (ex.: adicionar `#[event]` que referencia uma struct em módulo nomeado pelo usuário).
3. **Pipeline em três fases** para toda operação de scaffold (alinhado com ADR-0001 § 8.x):
   - **Plan** — computa `FileSetPlan` em memória (creates, updates, marker-region edits) sem tocar disco.
   - **Validate** — dry-run: schema-check, lint de markers, paths dentro do workspace, conflitos.
   - **Commit** — escrita atômica por arquivo (`<path>.sunscreen-tmp.<pid>` + rename); undo log para rollback.
4. **Hooks pós-commit** opcionais: `cargo fmt --files-with-diff <changed>`, `cargo check` (gated por flag).
5. **Markers versionados** (`version=<n>`); bumps acionam migrators automáticos.

Esta decisão é consistente com Sub-ADR-001 (ADR-0001 § 7.1).

---

## 5. Consequences

### 5.1 Positivas

- Implementação simples e auditável.
- Zero dependência de toolchain Rust em runtime para o caminho primário.
- Determinístico e rápido (line-scan O(n)).
- Auto-documentado: o usuário lê o arquivo e vê literalmente o contrato.
- Testes ficam triviais (golden + snapshot via `insta`).
- Suporta naturalmente o conceito de `user-region` que viabiliza preservação de handlers.

### 5.2 Negativas

- Mutação de código user-authored é fora do escopo (exceto via `ast-grep`).
- Markers visualmente "poluem" os arquivos. Mitigação: convenção visual `=== … ===` torna-os legíveis e segregáveis a olho nu.
- Usuário pode acidentalmente apagar um marker. Mitigação: `sunscreen chain doctor --fix-markers` (R2).
- Rename/move de arquivos pelo usuário pode descolar markers da expectativa do scaffolder. Ver § 7 Open Questions.

### 5.3 Mitigações

- Validação de markers roda em **toda** invocação de `sunscreen scaffold` antes de qualquer escrita.
- CI tem golden test específico de "markers sobrevivem a `rustfmt --edition=2024`" (ADR-0001 § 9.5.1).
- Migrators garantem que bumps de `version=` não quebram workspaces existentes.

---

## 6. Implementation Plan (Phase 2)

Ordem de implementação dos scaffolders durante Phase 2:

| Round | Scaffolder | Segments tocados | Notas |
|---|---|---|---|
| **R1** | `instruction` | `instructions` (mod.rs), `dispatch` (lib.rs), `file` + `handler` (instruction.rs) | bootstrap do mecanismo; cobre todos os tipos de marker |
| **R2** | `account` | `accounts` (state/mod.rs), `state` (state/<acc>.rs) | adiciona `chain doctor --fix-markers` |
| **R3** | `event` | `events` (events.rs) | primeiro uso potencial de `ast-grep` fallback |
| **R4** | `error` | `errors` (errors.rs) | variantes do `#[error_code]` enum |
| **R5** | `program` | sub-workspace inteiro | compõe R1–R4 sobre um programa novo dentro de workspace existente |

Cada round entrega: scaffolder + golden tests + entrada na tabela de `docs/reference/markers.md` se introduzir novo segment.

### 6.1 Componentes esperados

```
src/
├── rustpatch/
│   ├── marker.rs       # scan / validate / apply
│   ├── segment.rs      # registry de segments + versões
│   ├── migrate/        # migradores version=N -> version=N+1
│   ├── astgrep.rs      # subprocess wrapper (escape hatch)
│   └── fmt.rs          # invocação de rustfmt
├── scaffold/
│   ├── plan.rs         # FileSetPlan
│   ├── instruction.rs  # R1
│   ├── account.rs      # R2
│   ├── event.rs        # R3
│   ├── error.rs        # R4
│   └── program.rs      # R5
```

---

## 7. Open Questions

1. **Renames/movimentações pelo usuário.** Se o usuário move `instructions/deposit.rs` → `instructions/transfers/deposit.rs`, o scaffolder perde a referência. Opções:
   - (i) Detectar via `git mv` no histórico (frágil — usuário pode não usar git).
   - (ii) Manter um índice `_workspace/.sunscreen/manifest.json` com paths conhecidos e segments resolvidos.
   - (iii) Re-scan completo do `src/` procurando por todos os markers a cada invocação (O(n) — provável escolha).
   - **Tentativa atual:** (iii) + warning se um segment esperado some.
2. **Drift entre IDL e código.** Usuário pode editar o struct `Deposit<'info>` manualmente, divergindo do que `sunscreen` geraria. Como detectar?
   - Opção: re-rodar `scaffold instruction <name>` sempre regenera o segment `file` (que é `auto-generated`) — então a divergência é a *intenção* do usuário ao **não** rodar o comando. `chain doctor` pode comparar IDL gerado por Anchor vs. IDL inferido dos args originais persistidos em `.sunscreen/manifest.json` e avisar.
3. **Múltiplos programs no workspace.** R5 precisa decidir se markers carregam um qualifier de programa ou se o path do arquivo basta como contexto. Tendência: path basta; markers permanecem locais ao arquivo.
4. **Suporte a edition futura do Rust.** Se Rust 2027 mudar comportamento de line comments dentro de `mod`, golden test em CI quebra primeiro — política reativa, não preventiva.

---

## 8. Acceptance Criteria

- [ ] `docs/reference/markers.md` é a fonte de verdade do formato e está linkada do mdBook (ADR-0003).
- [ ] Scaffolders R1–R5 implementados com golden tests.
- [ ] Golden test específico "markers sobrevivem a `rustfmt --edition=2024`" passa em CI.
- [ ] Re-rodar qualquer scaffold com mesmos args produz diff vazio.
- [ ] `sunscreen chain doctor --fix-markers` recupera de marker corrompido em pelo menos os cenários listados em `docs/reference/markers.md` § 6.
