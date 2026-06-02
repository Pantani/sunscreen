---
name: sunscreen-docs-orchestrator
description: Orquestra o time de documentação do CLI sunscreen — site mdBook, GitHub Pages, trilhas Learn/Guides/Reference/Concepts, identidade visual e QA de docs. Use sempre que o usuário pedir para criar, escrever, expandir, revisar, atualizar, polir, redesenhar ou publicar documentação do sunscreen — incluindo "site de docs", "GitHub Pages", "tutoriais", "quickstart", "reference", "primer", "glossário", "landing", "tema das docs", "docs bonita", "docs tipo TMDCP", "Phase 8 docs", "documentação para iniciantes", "documentação profissional", "guia", "como usar", "manual", "mdBook", "publicar docs". Não use para ADRs (esses ficam com sunscreen-orchestrator → docs-writer) nem para implementação de código.
---

# Sunscreen Docs Orchestrator

Coordena 5 agentes para entregar o site de documentação do sunscreen, alvo de Phase 8 do `ROADMAP.md`. Estilo-alvo: TMDCP — premium, editorial, acessível para iniciantes e denso para profissionais.

## Phase 0: Contexto

1. Releia `CLAUDE.md`, `ROADMAP.md`, `README.md`, `docs/adr/ADR-0003-documentation-strategy.md` e `docs/reference/*`.
2. Liste o que já existe em `docs/site/` (se existir) e em `_workspace/`.
3. Decida modo de execução:
   - `docs/site/` não existe → **execução inicial completa** (Phases 1→6).
   - `docs/site/` existe + pedido específico (ex.: "atualiza reference de chain serve") → **execução parcial** (apenas o agente dono daquela área).
   - `_workspace/docs-review.md` existe e tem bloqueadores → **execução de correção** (chamar o autor original do defeito).

## Time

| Agente | Domínio |
|--------|---------|
| `docs-architect` | `docs/site/book.toml`, `SUMMARY.md`, workflow Pages, tema base |
| `docs-tutorial-writer` | `learn/`, `guides/` |
| `docs-reference-writer` | `reference/`, `concepts/` |
| `docs-designer` | tema CSS, landing, logo, diagramas |
| `docs-reviewer` | QA cross-doc, link check, build check |

**Execução: hybrid.** Em ambiente com subagentes, spawn em paralelo onde possível; sem subagentes, executa localmente seguindo a ordem.

## Phases

### Phase 1: Arquitetura (sequencial, bloqueia o resto)
**Owner**: `docs-architect`.
Gera `book.toml`, `SUMMARY.md`, esqueleto de diretórios, workflow Pages. Output: `_workspace/done_docs-architect.md` listando rotas e gaps.

### Phase 2: Identidade visual (paralelo com Phase 3)
**Owner**: `docs-designer`.
Primeiro entrega `_workspace/palettes.md` com 3 paletas. **Orquestrador pausa e pede escolha ao usuário** (via `AskUserQuestion`) antes de aplicar tema. Depois: theme CSS, logo, favicon, landing.

### Phase 3: Conteúdo (paralelo)
**Owners**: `docs-tutorial-writer` + `docs-reference-writer`.
Trabalham em diretórios disjuntos — sem conflito de arquivo. Cada um sinaliza pronto via `_workspace/done_<agent>.md`.

### Phase 4: Diagramas (depende de Phase 3 + Phase 2)
**Owner**: `docs-designer` (mermaid) em colaboração com `docs-reference-writer` (conteúdo dos diagramas).
Diagramas de: arquitetura, build pipeline, plugin runtime, marker lifecycle.

### Phase 5: Review (sequencial, depois de tudo)
**Owner**: `docs-reviewer`.
Roda auditoria completa, gera `_workspace/docs-review.md`. Se houver bloqueadores → orquestrador re-aciona os autores (loop max 2 iterações; depois reporta ao usuário).

### Phase 6: Build & Deploy check
- `mdbook build docs/site/` local sem warnings.
- `mdbook test docs/site/` (valida snippets Rust marcados).
- Validar workflow Pages com `act` se disponível, senão inspeção manual.
- Não fazer deploy automático — reportar ao usuário "pronto para merge; Pages publicará no push em main".

## Data flow

- **`_workspace/`** é a área compartilhada. Cada agente escreve `done_<agent>.md` ao terminar.
- Sinalizações importantes (paleta escolhida, gaps de conteúdo) vão em arquivos dedicados (`_workspace/palettes.md`, `_workspace/content-gaps.md`).
- Site final em `docs/site/`. Não tocar em `docs/adr/` nem `docs/reference/` (esses são do harness principal — apenas linkar/republicar).

## Error handling

- Agente falha → 1 retry com a mensagem do erro.
- Falha persistente → reportar ao usuário com arquivo, comando, output. Não tentar contornar com edição manual genérica.
- Review encontra bloqueador → re-aciona autor original (max 2 iterações). Se persistir, deixa documentado em `docs-review.md` e segue.
- Conflito de design entre `docs-designer` e `docs-architect` → orquestrador decide a favor do `docs-architect` (estrutura > estética).

## Relatório

Ao terminar resuma:
- Arquivos criados em `docs/site/` agrupados por trilha (learn/guides/reference/concepts).
- Status do `mdbook build` e `mdbook-linkcheck`.
- Bloqueadores remanescentes do review.
- URL onde ficará publicado (`https://<org>.github.io/sunscreen/`) — pedir confirmação da org.
- Próximos passos (deploy, screenshot, anúncio).

## Re-run / pedidos parciais

Quando o usuário pede "atualiza só X":
1. Identifique o agente dono.
2. Pule Phase 1 (arquitetura já existe).
3. Execute só o agente dono + `docs-reviewer` no final (review escopado à área alterada).
4. Atualize variação log no CLAUDE.md.

## Não use esta skill para

- ADRs → `sunscreen-orchestrator` (agente `docs-writer`).
- Implementação de feature de CLI → `sunscreen-orchestrator`.
- Perguntas conceituais sobre Solana sem mudança em arquivo → resposta direta.
