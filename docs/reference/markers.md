# Marker Format Reference

> **Status:** canonical spec for marker syntax used by `sunscreen` scaffolders.
> **Related:** ADR-0001 § 7.1 (Rust Code Mutation Strategy), § 8.4 (Sample Generated File), ADR-0004 (Incremental Scaffolding).

`sunscreen` modifica arquivos Rust existentes (`lib.rs`, `instructions/mod.rs`, `errors.rs`, etc.) por meio de **regiões delimitadas por comentários estruturados** — chamadas de *markers*. Esta página é a fonte de verdade do formato. Qualquer divergência entre uma implementação e este documento é um bug.

---

## 1. Filosofia

- Markers são **comentários de linha** (`//`), nunca block comments. Isso garante sobrevivência ao `rustfmt` (item invariante, ver § 5).
- Markers nunca aparecem **dentro** de uma expressão, `match`, `if` ou bloco `{}` arbitrário; sempre em escopo de item (top-level, dentro de `mod {}`, dentro do bloco `#[program]`).
- Existem **dois tipos** de região:
  - `auto-generated` — território do `sunscreen`. Será **sobrescrito** a cada `sunscreen scaffold`.
  - `user-region` — território do humano. `sunscreen` **nunca toca** depois da criação inicial.
- Markers funcionam em pares (`begin` / `end`) e são casados por **parsing linha-a-linha**: `sunscreen` detecta linhas que contenham `// === sunscreen:`, ignora indentação inicial, e tokeniza atributos `key=value` por whitespace. Sem regex.

---

## 2. Sintaxe Formal

### 2.1 Região auto-gerenciada

```rust
// === sunscreen:auto-generated:begin segment=<name> version=<n> [generator=<g>] ===
// DO NOT EDIT THIS REGION. Manual changes will be overwritten by `sunscreen scaffold`.
//
// <conteúdo gerenciado>
//
// === sunscreen:auto-generated:end segment=<name> ===
```

### 2.2 Região do usuário

```rust
// === sunscreen:user-region:begin segment=<name> ===
// You can freely edit anything inside this region.
//
// <conteúdo do usuário — sunscreen nunca sobrescreve>
//
// === sunscreen:user-region:end segment=<name> ===
```

### 2.3 Gramática

```text
MARKER       := AG_BEGIN | AG_END | UR_BEGIN | UR_END

AG_BEGIN := "// === sunscreen:auto-generated:begin"
            " segment=" NAME " version=" INT
            ( " generator=" IDENT )?
            " ==="                              // version= obrigatório; user-region não versiona

AG_END   := "// === sunscreen:auto-generated:end segment=" NAME " ==="

UR_BEGIN := "// === sunscreen:user-region:begin segment=" NAME " ==="
UR_END   := "// === sunscreen:user-region:end segment=" NAME " ==="

NAME  := [a-z][a-z0-9_-]*
INT   := [1-9][0-9]*
IDENT := [a-z][a-z0-9_-]*
```

Regras adicionais:

- `===` no início da linha (após `//`) é obrigatório. O `===` de fechamento é convencional (preservado na geração) mas o scanner atual tolera sua ausência — futuras versões poderão torná-lo estrito.
- O scanner ignora **indentação inicial** (trim_start) e tokeniza atributos por whitespace; ordem de atributos extras é tolerada. A geração produz sempre a forma canônica (`segment=` antes de `version=`).
- `version` só aparece em `auto-generated`. `user-region` não versiona (sunscreen nunca migra conteúdo do usuário).
- `generator` é diagnóstico (qual scaffolder produziu o segmento).

---

## 3. Tipos de Marker

| Kind | `sunscreen` escreve | `sunscreen` lê | Usuário edita | Sobrevive a re-scaffold |
|---|---|---|---|---|
| `auto-generated` | sim, a cada scaffold | sim | **não** (será sobrescrito) | conteúdo é regenerado |
| `user-region` | só na criação inicial | sim (para preservar offsets) | **sim, livremente** | sim, preservado byte-a-byte |

> Resumo mental: `auto-generated` = "sunscreen escreve, humano lê"; `user-region` = "humano escreve, sunscreen evita".

---

## 4. Segmentos Conhecidos

| Segment | Kind padrão | Local | Conteúdo |
|---|---|---|---|
| `instructions` | `auto-generated` | `programs/<prog>/src/instructions/mod.rs` | `pub mod <ix>;` por instrução + re-exports |
| `dispatch` | `auto-generated` | `programs/<prog>/src/lib.rs` dentro de `#[program] pub mod <prog> { … }` | `pub fn <ix>(ctx: Context<…>, …) -> Result<()> { instructions::<ix>::handler(ctx, …) }` |
| `file` | `auto-generated` | `programs/<prog>/src/instructions/<ix>.rs` | imports, `#[derive(Accounts)] struct <Ix>`, structs auxiliares |
| `handler` | `user-region` | mesmo arquivo de `file` | corpo de `pub fn handler(...) -> Result<()> { … }` |
| `accounts` *(R2)* | `auto-generated` | `programs/<prog>/src/state/mod.rs` | `pub mod <acc>;` |
| `state` *(R2)* | `auto-generated` | `programs/<prog>/src/state/<acc>.rs` | `#[account] pub struct <Acc> { … }` |
| `events` *(R3)* | `auto-generated` | `programs/<prog>/src/events.rs` | declarações `#[event]` |
| `errors` *(R4)* | `auto-generated` | `programs/<prog>/src/errors.rs` | variantes do enum `#[error_code]` |

Segments futuros são adicionados a esta tabela e introduzem `version=1`; bumps subsequentes (`version=2`, …) acionam migradores automáticos.

---

## 5. Invariantes

1. **Sobrevivem a `rustfmt`.** Como são line comments fora de qualquer expressão, `rustfmt --edition=2024` preserva-os. Esta propriedade é coberta por golden test em CI (cf. ADR-0001 § 9.5.1).
2. **Nunca dentro de `match`, `if`, `for`, `while`, `loop`, ou bloco `{ … }` arbitrário.** Markers ficam apenas em escopo de item.
3. **Line-grained.** Markers ocupam linhas inteiras; nada de marker inline com código.
4. **Pareados e ordenados.** Para cada `begin segment=X` há exatamente um `end segment=X` posterior no mesmo arquivo. Sem aninhamento.
5. **Determinísticos.** Mesma invocação de scaffold com mesmos args ⇒ mesmo conteúdo entre os markers, byte-a-byte.

---

## 6. Erros Comuns e Recovery

| Sintoma | Causa | Recovery |
|---|---|---|
| `error: marker pair mismatch: begin segment=dispatch without matching end` | usuário apagou linha do `end` | `sunscreen chain doctor --fix-markers` (R2) reconstrói a partir do IDL + heurística |
| `error: duplicate begin segment=instructions in src/instructions/mod.rs` | merge conflict não resolvido | resolver conflito; manter apenas um par |
| `error: marker drift: version=1 expected, found version=2` | downgrade do CLI | atualizar `sunscreen` ou rodar migração reversa |
| `error: marker inside expression` | usuário moveu marker para dentro de `match` | mover de volta para escopo de item |
| `warning: user-region with version=` | violação da spec | sunscreen ignora o `version` e prossegue |

`sunscreen chain doctor` (R2 desta fase) validará markers em todo o workspace e oferecerá `--fix-markers` para casos recuperáveis.

---

## 7. Versionamento

- Toda região `auto-generated` carrega `version=<n>`.
- Bump de `version` indica **mudança incompatível** no formato do conteúdo gerado dentro daquele segment.
- Quando `sunscreen` encontra `version=N` mas o scaffolder atual emite `version=N+1`, ele executa o **migrator** correspondente antes de reescrever a região. *(Suporte a migradores automáticos planejado para R2.)*
- `version=1` é o ponto de partida para todos os segments listados em § 4.

---

## 8. Exemplo Completo

Workspace gerado por `sunscreen chain new escrow` seguido de:

```bash
sunscreen scaffold instruction deposit \
  --program escrow \
  --args "amount:u64" \
  --accounts "vault:mut|seeds=b\"vault\",depositor:signer|mut,system_program:system"
```

### 8.1 `programs/escrow/src/instructions/mod.rs`

```rust
// === sunscreen:auto-generated:begin segment=instructions version=1 ===
// DO NOT EDIT. Use `sunscreen scaffold instruction` to extend.
pub mod initialize;
pub mod deposit;

pub use initialize::*;
pub use deposit::*;
// === sunscreen:auto-generated:end segment=instructions ===
```

### 8.2 `programs/escrow/src/lib.rs`

```rust
use anchor_lang::prelude::*;

pub mod instructions;
pub mod state;

declare_id!("Esc11111111111111111111111111111111111111");

#[program]
pub mod escrow {
    use super::*;

    // === sunscreen:auto-generated:begin segment=dispatch version=1 ===
    pub fn initialize(ctx: Context<Initialize>, fee_bps: u16) -> Result<()> {
        instructions::initialize::handler(ctx, fee_bps)
    }

    pub fn deposit(ctx: Context<Deposit>, amount: u64) -> Result<()> {
        instructions::deposit::handler(ctx, amount)
    }
    // === sunscreen:auto-generated:end segment=dispatch ===
}
```

### 8.3 `programs/escrow/src/instructions/deposit.rs`

```rust
// === sunscreen:auto-generated:begin segment=file version=1 generator=instruction ===
// This file is initial scaffolding. The handler body below is a user-region.
// Re-running `sunscreen scaffold instruction deposit` with the same args is a no-op.

use anchor_lang::prelude::*;
use crate::state::*;

#[derive(Accounts)]
pub struct Deposit<'info> {
    #[account(mut, seeds = [b"vault"], bump)]
    pub vault: Account<'info, Vault>,
    #[account(mut)]
    pub depositor: Signer<'info>,
    pub system_program: Program<'info, System>,
}
// === sunscreen:auto-generated:end segment=file ===

// === sunscreen:user-region:begin segment=handler ===
// You can freely edit anything inside this region.
pub fn handler(ctx: Context<Deposit>, amount: u64) -> Result<()> {
    let vault = &mut ctx.accounts.vault;
    vault.total = vault.total.checked_add(amount).ok_or(ProgramError::ArithmeticOverflow)?;
    Ok(())
}
// === sunscreen:user-region:end segment=handler ===
```

O ponto-chave deste exemplo: rodar novamente `sunscreen scaffold instruction deposit` com os mesmos args **não toca** no corpo de `handler` — apenas valida que o segmento `file` continua coerente com os args fornecidos. Mudar os args (ex.: adicionar `--accounts ",fee_receiver:mut"`) regenera o segmento `file`, deixando `handler` intacto.

---

## 9. Conformidade

Implementações de scaffolders **devem**:

1. Emitir markers exatamente conforme § 2.
2. Validar pareamento antes de aplicar qualquer patch (fail-fast).
3. Tratar regiões `user-region` como read-only após criação.
4. Versionar todos os segments `auto-generated` com `version=` numérico.
5. Falhar com mensagem acionável apontando para `sunscreen chain doctor --fix-markers` quando encontrar corrupção.
