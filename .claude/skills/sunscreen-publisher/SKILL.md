---
name: sunscreen-publisher
description: Orquestra o time de publicação multi-canal do sunscreen CLI — Homebrew, Snap Store, APT — sobre o pipeline `release.yml` existente (cargo-dist tag-driven). Use SEMPRE que o usuário pedir para "publicar release", "distribuir em homebrew/snap/apt", "expandir o release pipeline", "adicionar canal de distribuição", "configurar brew tap", "snapcraft", "cargo-deb", "cloudsmith", "ppa", "instalador apt/brew/snap", "automatizar release", "publicar nova versão nos package managers", "tornar instalável via apt/brew/snap", ou qualquer trabalho no `.github/workflows/release.yml` relacionado a canais de distribuição. Também trigger em pedidos de "atualizar publish", "corrigir publicação do canal X", "reexecutar publish", "republish", "bump version nos package managers". NÃO use para builds Rust gerais, doctor, scaffolds — só para o eixo de release/distribuição.
---

# Sunscreen Publisher Harness

## Objetivo
Estender o `release.yml` existente (cargo-dist binários para GitHub Release) para também publicar em **Homebrew tap**, **Snap Store** e **APT via Cloudsmith** a cada tag `vX.Y.Z`. O caminho mais simples e eficiente vence — nada de PPA Launchpad, nada de build dentro do snap, nada de hospedar repo APT próprio se Cloudsmith free tier resolve.

## Time

| Agente | Responsabilidade |
|--------|------------------|
| `release-orchestrator` | Shape do workflow, ordem de jobs, secrets, smoke tests, doc agregada |
| `homebrew-publisher` | cargo-dist homebrew installer + tap repo |
| `snap-publisher` | snapcraft.yaml classic + snapcore/action-publish |
| `apt-publisher` | cargo-deb + Cloudsmith upload |

## Modo de Execução
**Hybrid**: orchestrator define o esqueleto do workflow (sequencial, Phase 1) → 3 publishers trabalham em **paralelo** como sub-agentes (Phase 2) → orchestrator consolida (Phase 3).

## Phase 0: Contexto
1. Ler `.github/workflows/release.yml` atual.
2. Verificar `_workspace/done_*-publisher.md` — se existe, é **rerun** (correção de canal específico). Caso contrário, **construção inicial**.
3. Confirmar com o usuário:
   - Owner do tap repo Homebrew (ex.: `Pantani/homebrew-sunscreen`)?
   - Snap name `sunscreen` está disponível?
   - Cloudsmith org/repo, ou usar fallback `gh-pages` APT?

## Phase 1: Skeleton (release-orchestrator)
Orchestrator edita `release.yml` adicionando 3 jobs stub `publish-homebrew`, `publish-snap`, `publish-apt` com `needs: [host]` e `continue-on-error: true`. Cria `docs/reference/distribution.md` com a tabela de secrets.

## Phase 2: Publishers (paralelo)
Spawn em paralelo via `Agent` com `run_in_background: true`:
- `homebrew-publisher` → preenche o job + edita `Cargo.toml` (`[workspace.metadata.dist] installers/tap`).
- `snap-publisher` → cria `snap/snapcraft.yaml` + preenche o job.
- `apt-publisher` → adiciona `[package.metadata.deb]` + preenche o job.

Cada um deposita `_workspace/done_*-publisher.md`.

## Phase 3: Consolidação (release-orchestrator)
1. Lê os 3 `done_*` files.
2. Resolve qualquer conflito de version string.
3. Adiciona step `release-summary` que agrega status dos 3 canais.
4. Atualiza `README.md` com badges + comandos install.
5. Adiciona entry no `CHANGELOG.md`.
6. Roda `cargo dist plan` localmente para validar config (sem push).

## Phase 4: Validação
Antes de pedir merge:
- [ ] `cargo dist plan` exit 0.
- [ ] `actionlint .github/workflows/release.yml` exit 0.
- [ ] Lista de secrets requeridos publicada em `docs/reference/distribution.md`.
- [ ] Smoke step de cada job documentado.
- [ ] Plano de teste com tag `v0.0.0-test.N` (dry-run via `workflow_dispatch`).

## Princípios Não Negociáveis
- **Falha de canal não bloqueia release** (`continue-on-error` + summary).
- **Mesma versão em todos canais** — derivada da tag.
- **Zero rebuild** nos canais que podem consumir o binário do release (snap, apt).
- **Idempotência** — rerun da mesma tag não duplica/quebra.
- **Secrets nunca em logs** — usar `${{ secrets.* }}` e mascaramento default.

## Re-execução
Quando o usuário pedir "republish do snap" ou "fix homebrew":
1. Phase 0 detecta `done_*` existente.
2. Orchestrator identifica o canal afetado.
3. Apenas o publisher daquele canal é invocado.
4. Outros canais são pulados.

## Cenários de Teste
- **Feliz**: tag `v0.2.0` → 3 canais publicam → `apt/brew/snap install sunscreen` retorna `sunscreen 0.2.0` em containers limpos.
- **Falha**: Snap Store 503 → workflow finaliza com summary mostrando `snap: failed`, `brew: ok`, `apt: ok` — release não é deletado.
