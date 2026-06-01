# ADR-0005 — Beginner Onboarding Surface for `sunscreen`

| Field | Value |
|---|---|
| **Status** | Proposed |
| **Date** | 2026-06-01 |
| **Authors** | Pantani |
| **Tags** | onboarding, ux, beginner, wizard, dx |
| **Supersedes** | — |
| **Superseded by** | — |
| **Related** | ADR-0001 § 1.4 e § 10.7 (sunscreen CLI), ADR-0002 (CLI Design Conventions), ADR-0004 (Incremental Scaffolding), `IMPLEMENTATION-KICKOFF.md` |

---

## Variation Log

| Date | Author | Version | Summary |
|------|--------|---------|---------|
| 2026-06-01 | Pantani | 1.0.0 | Initial ADR — formaliza a Phase 5.5 (Onboarding Layer) |

---

## TL;DR

`sunscreen` adiciona uma **camada de onboarding dedicada** (Phase 5.5) composta por sete comandos de alto nível — `init`, `examples`, `quickstart`, `wallet`, `deploy`, `learn` — e um contrato formal de **erros acionáveis** com campo `next_step`. Esta camada é uma fina envoltória interativa sobre o core já existente (`chain new`, scaffolders, doctor): `init` é um wizard `dialoguer` que termina chamando o mesmo loader/validator de `chain new`; `quickstart <recipe>` compõe `chain new` + scaffolders + frontend bootstrap em um único comando one-shot; `wallet` e `deploy` são wrappers amigáveis sobre `solana-keygen`/`solana airdrop`/`anchor deploy`; `examples` distribui projetos prontos via `rust-embed`; `learn` renderiza tutoriais markdown embarcados via `termimad`. Todos os comandos respeitam DD2 (não bloqueio do power user): TTY-detection desliga prompts e `--non-interactive` força equivalência flag-based. DoD: usuário sem conta Solana faz `sunscreen init` → `sunscreen quickstart nft` → vê NFT mintada em devnet em **< 10 min**.

---

## 1. Context

### 1.1 Problem framing

O plano original do ADR-0001 (Phases 0–8) assume um **dev Solana intermediário** — alguém que já entende `Pubkey`, sabe a diferença entre `Account` e `AccountInfo`, conhece o ciclo `anchor build → anchor deploy`, e está confortável editando `Anchor.toml`. As Phases 0–2 R3 (já entregues) refletem esse público: `chain new` exige flags explícitas (`--framework anchor --frontend next --clients ts,rs`), scaffolders esperam que o usuário saiba o que é uma "instruction", e `chain doctor` reporta toolchain status em jargão (`anchor-cli 0.30.x`, `solana-cli 2.0.x`).

A **visão de produto**, porém, é mais ambiciosa: `sunscreen` deve ser **a porta de entrada para devs que não sabem Rust nem Solana profundamente** — um desenvolvedor TypeScript com curiosidade sobre NFTs, um estudante que ouviu falar de SPL tokens, um indie que quer prototipar um DAO em uma tarde. Esse público:

- Não sabe que precisa de uma keypair antes de fazer `airdrop`.
- Não sabe a diferença entre `localnet`, `devnet`, `testnet`, `mainnet`.
- Não sabe o que é um PDA, e portanto não sabe *por que* `scaffold account` pergunta sobre seeds.
- Vai abandonar o CLI nos primeiros 5 minutos se a primeira tela for `error: missing required argument '--framework <FRAMEWORK>'`.

Este ADR formaliza uma **camada de superfície** que não existe no ADR-0001 e que será priorizada entre Phase 5 (recipes) e v1.0 (release).

### 1.2 Restrições

- **Não quebrar o expert.** Toda a surface de Phase 0–5 deve continuar funcionando byte-a-byte. Onboarding é *aditivo*.
- **Offline-first.** Examples e tutoriais embarcados (sem `git clone` mandatório no primeiro uso).
- **Sem path paralelo.** O wizard `init` **não** pode duplicar o loader/validator de `chain new`; precisa terminar invocando o mesmo código.
- **TTY-aware.** Detectar `isatty(stdin)` e degradar para flag-based quando rodando em pipe/CI.
- **Custo zero por default.** Nenhum comando deve gastar SOL sem confirmação explícita (mainnet em especial).
- **i18n preparado mas en-US first.** Strings centralizadas em `src/strings/en_US.rs`; PT-BR fica como skill futura.

---

## 2. Decision Drivers

- **DD1 — Curva de aprendizado.** Tempo "hello world → NFT deployed em devnet" < 10 min para iniciante absoluto sem conta prévia.
- **DD2 — Não-bloqueio do power user.** Todo wizard tem equivalente flag-based; `--non-interactive` ou TTY-detection desliga prompts; nenhum comando novo aparece em `chain new` ou nos scaffolders existentes.
- **DD3 — Sem network mandatório.** Examples e `learn` embarcados via `rust-embed`; cluster ops (`wallet airdrop`, `deploy devnet`) são opt-in.
- **DD4 — Reuso da infraestrutura existente.** Wizard chama o mesmo `ChainNewArgs::from_resolved(...)` que `chain new` usa; `quickstart` compõe scaffolders pelo seu Rust API, não por shellout.
- **DD5 — i18n preparado, en-US first.** Strings em módulo dedicado; nenhum literal no fluxo de controle.
- **DD6 — Erros acionáveis.** Toda variante de `SunscreenError` carrega `next_step: Option<String>`; cobertura 100% verificada em CI.
- **DD7 — Discoverability.** Comandos top-level (não flags), nomes orientados a tarefa do usuário (`wallet new`, não `keypair generate`).

---

## 3. Considered Options

| # | Opção | Resumo |
|---|---|---|
| (A) | **Layer separado** *(escolhido)* | Novos comandos top-level (`init`, `examples`, `quickstart`, `wallet`, `deploy`, `learn`) + delegação ao core |
| (B) | Flags `--interactive` em comandos existentes | `chain new --interactive`, `scaffold instruction --interactive` |
| (C) | Sub-CLI separado | Distribuir `sunscreen-easy` como binário paralelo |
| (D) | Plugin externo opcional | Onboarding via `sunscreen plugin install onboarding` |

### 3.1 Opção (A) — Layer separado

**Prós:**
- Mantém ADR-0001 intacto: nenhuma flag nova em `chain new` ou scaffolders.
- Surface explícita e descobrível: `sunscreen --help` lista os comandos amigáveis ao lado dos expert.
- Fácil de evoluir: cada comando é uma `clap` subcommand com seu próprio módulo.
- Composição limpa: `quickstart nft` é literalmente `chain_new(...) + scaffold_instruction(...) + scaffold_account(...)` em sequência.

**Contras:**
- Aumenta o número de comandos top-level (de ~8 para ~14). Mitigação: grupo `Beginner` na `--help` (cf. ADR-0002 § 3.2).
- Risco de divergência entre o que o wizard pergunta e o que `chain new` aceita. Mitigação: validador único compartilhado.

### 3.2 Opção (B) — `--interactive` em comandos existentes

**Prós:** zero comandos novos; descoberta via `--help` do comando existente.

**Contras:**
- Polui surface: `chain new --interactive --framework anchor` é semanticamente confuso (por que pedir framework se é interativo?).
- Não cobre os casos novos (`wallet`, `deploy`, `learn`, `examples`) que não têm equivalente atual.
- Dificulta o "happy path" do iniciante: ele precisa adivinhar que `chain new` é o ponto de entrada.

**Rejeitado.**

### 3.3 Opção (C) — Sub-CLI separado (`sunscreen-easy`)

**Prós:** isola completamente UX iniciante da expert.

**Contras:** fragmenta a marca; duplica config loading; usuário precisa aprender *quando* trocar de binário; instalação dobra.

**Rejeitado.**

### 3.4 Opção (D) — Plugin externo opcional

**Prós:** mantém core enxuto; permite iteração rápida sem release do core.

**Contras:** onboarding precisa ser **default**, não opt-in — quem mais precisa do plugin é justamente quem não vai descobrir que precisa instalá-lo.

**Rejeitado.**

---

## 4. Decision

`sunscreen` adota a **Opção (A) — Layer separado** com sete novos comandos top-level e um contrato formal de erros acionáveis.

### 4.1 Comandos novos

| Signature | Flags | Output | Exit codes |
|-----------|-------|--------|-----------|
| `sunscreen init [name]` | `--non-interactive`, `--from-preset <name>`, `--json` | Cria workspace; emite resumo das escolhas + próximo passo | 0 ok; 4 user_input (prompt abortado); 5 conflict (path existe) |
| `sunscreen examples list` | `--json`, `--tag <tag>` | Tabela: nome, descrição curta, tags, tempo estimado | 0 ok |
| `sunscreen examples describe <name>` | `--json` | README do exemplo renderizado via `termimad` | 0 ok; 6 not_found |
| `sunscreen examples use <name> [path]` | `--non-interactive`, `--json` | Copia exemplo embarcado para `path` (default: `./<name>`) | 0 ok; 5 conflict; 6 not_found |
| `sunscreen quickstart <recipe>` | `--name <n>`, `--cluster <localnet\|devnet>`, `--non-interactive`, `--json` | Compõe `chain new` + scaffolds + bootstrap frontend; abre `localhost:3000` se TTY | 0 ok; 4 user_input; 5 conflict; 7 toolchain |
| `sunscreen wallet new [name]` | `--out <path>`, `--no-bip39-passphrase`, `--json` | Cria keypair; reporta pubkey + path | 0 ok; 5 conflict |
| `sunscreen wallet list` | `--json` | Lista keypairs conhecidas + qual é default | 0 ok |
| `sunscreen wallet airdrop [amount]` | `--cluster <c>`, `--to <pubkey>`, `--json` | Solicita airdrop; reporta saldo final | 0 ok; 8 network; 9 rate_limited |
| `sunscreen wallet balance` | `--cluster <c>`, `--json` | Saldo da default keypair | 0 ok; 8 network |
| `sunscreen wallet set-default <name>` | `--json` | Atualiza `sunscreen.yml` | 0 ok; 6 not_found |
| `sunscreen deploy <target>` | `--program <name>`, `--verify`, `--yes-i-understand-cost` (mainnet), `--json` | Wrappa `anchor deploy`; mostra custo estimado antes (mainnet) | 0 ok; 4 user_input; 7 toolchain; 8 network |
| `sunscreen learn` | — | Lista tópicos disponíveis | 0 ok |
| `sunscreen learn <topic>` | `--json` (emite frontmatter) | Renderiza tutorial markdown via `termimad` | 0 ok; 6 not_found |

`<recipe>` ∈ `{token, nft, dao, blog}` (extensível em ADR futuro).
`<target>` ∈ `{localnet, devnet, mainnet}`.
`<topic>` MVP ∈ `{pda, cpi, token-2022, accounts-model, anchor-vs-native}`.

### 4.2 Contrato de erros acionáveis

```rust
pub struct SunscreenError {
    pub kind: ErrorKind,
    pub message: String,
    pub next_step: Option<String>, // <-- contrato novo: 100% das variantes
    pub source: Option<Box<dyn std::error::Error + Send + Sync>>,
}
```

- CI test (`tests/errors_contract.rs`) usa `strum::IntoEnumIterator` para garantir que cada `ErrorKind` tem ao menos um construtor com `next_step.is_some()`.
- `--json` serializa `next_step` como campo top-level (cf. ADR-0002 § 5).
- Renderização TTY: linha extra `→ try: <next_step>` em ciano.

### 4.3 Distribuição de assets

- **Examples**: embarcados via `rust-embed` em `assets/examples/<name>/**`. Tamanho-alvo do binário: < 15 MB total. Examples grandes (>2 MB) marcados com `remote=true` no manifest; `examples use <name>` baixa via `gix` (puro Rust, sem dep de `git` CLI).
- **Learn**: 100% embarcado em `assets/learn/<topic>.md`. Frontmatter YAML com `title`, `est_minutes`, `prereqs`.
- **Recipes** (`quickstart`): definidas em código (`src/onboarding/recipes/<name>.rs`) — não são templates, são *programas* que orquestram chamadas do core.

### 4.4 TTY detection & --non-interactive

- Helper único `src/onboarding/tty.rs::is_interactive() -> bool` consulta `IsTerminal::is_terminal(&io::stdin())` E ausência de `--non-interactive` E ausência de `SUNSCREEN_NON_INTERACTIVE=1`.
- Wizard prompts substituídos por erro `ErrorKind::UserInput` com `next_step` listando a flag equivalente quando `is_interactive() == false`.

---

## 5. Consequences

### 5.1 Positivas

- Democratiza o CLI: iniciante chega ao primeiro NFT mintado em < 10 min.
- Aumenta adoção e reduz fricção em demos/workshops.
- Reduz carga de suporte: erros com `next_step` evitam metade das issues abertas hoje em CLIs similares.
- Reaproveita 100% do core existente — wizard é fina camada.
- `learn` cria pulmão de documentação procurável dentro do binário.

### 5.2 Negativas

- **+2 sprints** de trabalho (Bloco E do roadmap; ver § 6).
- **+5–8 MB no binário** por embed de examples + learn. Mitigação: `--features minimal` para builds em CI/produção sem onboarding.
- Aumenta superfície de testes: cada wizard precisa de teste interativo (via `expectrl` ou similar) + teste `--non-interactive`.
- Risco de divergência entre wizard e flags. Mitigação: validador único; teste property-based que aleatoriza inputs do wizard e compara com `chain new` equivalente.
- Mais comandos top-level na `--help`. Mitigação: agrupar via `clap` `help_heading`.

### 5.3 Neutrais

- Requer 3 novas deps: `dialoguer ^0.11`, `termimad ^0.31`, `indicatif ^0.17` — todas já planejadas para a TUI da Phase 6.
- Strings centralizadas em `src/strings/` viabilizam i18n futura sem refactor adicional.

### 5.4 Mitigações de risco

- Golden tests gravam transcripts completos de cada wizard (via `insta` + `expectrl`).
- Property test: para cada combinação possível de respostas do `init`, verificar que o workspace resultante é byte-idêntico ao gerado por `chain new` com flags equivalentes.
- `quickstart` tem teste E2E em `localnet` no CI (Surfpool inicia em background; teardown em `Drop`).

---

## 6. Implementation Plan (Phase 5.5)

Inserida entre Phase 5 (recipes) e Phase 8 (release). 2 sprints (~4 semanas).

| Sprint | Entrega | Tests |
|---|---|---|
| **S1** | `init` (wizard + validator share), `wallet *`, contrato `next_step` em 100% das variantes | unit + golden de transcripts |
| **S2** | `examples` (list/describe/use), `quickstart {token, nft, dao, blog}`, `deploy`, `learn` (5 tópicos MVP) | E2E em localnet; teste de embed integrity |

### 6.1 Componentes esperados

```text
src/
├── onboarding/
│   ├── mod.rs
│   ├── tty.rs              # is_interactive()
│   ├── wizard.rs           # init flow
│   ├── recipes/
│   │   ├── token.rs        # SPL fungível
│   │   ├── nft.rs          # Metaplex Token Metadata + Master Edition
│   │   ├── dao.rs          # voting program
│   │   └── blog.rs         # CRUD com PDAs
│   ├── wallet.rs           # solana-keygen wrapper
│   ├── deploy.rs           # anchor deploy wrapper + cost preview
│   ├── examples.rs         # rust-embed gallery
│   └── learn.rs            # termimad renderer
├── strings/
│   └── en_US.rs            # toda string user-facing
└── error.rs                # next_step field
assets/
├── examples/
│   ├── token-faucet/
│   ├── nft-collection/
│   ├── escrow/
│   ├── voting-dao/
│   └── blog-crud/
└── learn/
    ├── pda.md
    ├── cpi.md
    ├── token-2022.md
    ├── accounts-model.md
    └── anchor-vs-native.md
```

---

## 7. UX Examples

### 7.1 `sunscreen init` (transcript)

```text
$ sunscreen init
✻ Welcome to sunscreen — let's build a Solana app.

? Project name › my-app
? What are you building?
  ❯ A token (SPL fungible)
    An NFT collection (Metaplex)
    A DAO / voting program
    A blog / CRUD app
    Something else (blank workspace)
? Frontend framework?
  ❯ Next.js (recommended)
    Vite + React
    None (CLI only)
? Generate client SDKs?
  ❯ TypeScript + Rust
    TypeScript only
    None
? Cluster for development? › devnet

✓ Workspace created at ./my-app
✓ Codama IDL bootstrapped
✓ Frontend scaffolded (Next.js)

→ next: cd my-app && sunscreen quickstart nft
```

### 7.2 `sunscreen quickstart nft` (output)

```text
$ sunscreen quickstart nft --name pixel-cats
[1/6] chain new pixel-cats --framework anchor --frontend next --clients ts,rs   ✓ (1.2s)
[2/6] scaffold account collection --seeds "collection,authority"                 ✓ (0.4s)
[3/6] scaffold instruction mint_nft --accounts collection,mint,metadata         ✓ (0.6s)
[4/6] scaffold instruction update_metadata                                       ✓ (0.3s)
[5/6] wallet airdrop 2 --cluster devnet                                          ✓ (3.1s)
[6/6] anchor build && anchor deploy --provider.cluster devnet                    ✓ (47s)

✓ Program deployed: 7xKXt...mJqP
✓ Frontend running at http://localhost:3000
✓ Mint your first NFT: http://localhost:3000/mint

→ next: open http://localhost:3000/mint in your browser
```

### 7.3 Erro acionável com `next_step`

```text
$ sunscreen deploy devnet
error: no default wallet configured
  → try: sunscreen wallet new --out ~/.config/solana/id.json
```

JSON equivalent:

```json
{
  "ok": false,
  "kind": "user_input",
  "message": "no default wallet configured",
  "next_step": "sunscreen wallet new --out ~/.config/solana/id.json",
  "exit_code": 4
}
```

---

## 8. Open Questions

1. **Examples gallery: embed vs git clone on-demand?**
   - Tendência: **embed por default** (offline-first, DD3); flag `remote=true` no manifest para examples grandes (> 2 MB) que são baixados via `gix`.
2. **Wizard em PT-BR no MVP ou só en-US?**
   - Tendência: **en-US first** (Solana é global); strings centralizadas em `src/strings/en_US.rs` para viabilizar PT-BR via skill futura sem refactor.
3. **`sunscreen deploy mainnet` exige `--yes-i-understand-cost` ou só confirmação interativa?**
   - Tendência: **ambos** — confirmação interativa quando TTY, flag obrigatória quando `--non-interactive` (cobre CI accidents).
4. **`sunscreen learn` content gerenciado in-repo ou repo separado versionado?**
   - Tendência: **in-repo** no MVP (5 tópicos); migrar para `sunscreen-learn` repo + `learn update` quando passar de ~20 tópicos.
5. **`quickstart` deve abrir o browser automaticamente?**
   - Tendência: sim quando TTY (via `open` crate); silenciar quando `--non-interactive`.
6. **Wallet storage location.**
   - Reusar `~/.config/solana/id.json` (compat com `solana-cli`) ou usar `~/.config/sunscreen/wallets/`? Tendência: reusar o path canônico do Solana para interop.

---

## 9. Acceptance Criteria

- [ ] Sete novos comandos implementados conforme tabela § 4.1 com `--json` e `--non-interactive` onde aplicável.
- [ ] Contrato `next_step` cobre 100% das variantes de `ErrorKind` (verificado por test em CI).
- [ ] Wizard `init` produz workspace **byte-idêntico** ao `chain new` com flags equivalentes (property test).
- [ ] Cinco recipes de `quickstart` (`token`, `nft`, `dao`, `blog`, + um genérico) executam em localnet no CI.
- [ ] `sunscreen examples list` retorna ≥ 5 entries embarcadas; `examples use <name>` cria projeto utilizável.
- [ ] `sunscreen learn` renderiza ≥ 5 tópicos MVP sem warnings de `termimad`.
- [ ] DoD humano: usuário sem conta Solana faz `sunscreen init` → `sunscreen quickstart nft` → vê NFT mintada em devnet em **< 10 min** (medido em workshop interno).
- [ ] Tamanho do binário com onboarding: < 25 MB (release, stripped); sem onboarding (`--no-default-features`): < 12 MB.

---

## 10. References

- ADR-0001 § 1.4 (Personas alvo) e § 10.7 (Recipes & onboarding gaps)
- ADR-0002 (CLI Design Conventions — `--json`, exit codes, help grouping)
- ADR-0004 (Incremental Scaffolding — reuso do core pelos recipes)
- `IMPLEMENTATION-KICKOFF.md` (roadmap; Phase 5.5 a ser inserida)
- Ignite CLI `scaffold chain` wizard (referência de UX)
- `dialoguer` 0.11, `termimad` 0.31, `indicatif` 0.17, `rust-embed` 8.x, `gix` 0.66
