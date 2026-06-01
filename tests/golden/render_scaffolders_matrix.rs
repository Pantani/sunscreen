//! Golden snapshot matrix for Phase 2 scaffolders.
//!
//! This file intentionally keeps many small snapshots instead of one huge
//! corpus so R5 coverage points at the exact scaffolder surface that changed.

use std::path::{Path, PathBuf};

use serde_json::json;
use sunscreen::templates::account::{AccountCtx, FieldSpec as AccountField};
use sunscreen::templates::event::{EventCtx, FieldSpec as EventField};
use sunscreen::templates::{
    render_account_file, render_account_mod_entry, render_account_mod_segment,
    render_dispatch_segment, render_error_variant, render_errors_file, render_event_entry,
    render_events_file, render_instruction, render_instructions_mod_segment, render_program,
    AccountKind, AccountSpec, ArgSpec, ErrorCtx, ErrorVariant, InstructionCtx, InstructionDispatch,
};

fn snap(name: &str, value: String) {
    insta::assert_snapshot!(name, value);
}

fn account_field(name: &str, ty: &str) -> AccountField {
    AccountField {
        name: name.into(),
        ty: ty.into(),
    }
}

fn event_field(name: &str, ty: &str) -> EventField {
    EventField {
        name: name.into(),
        ty: ty.into(),
    }
}

fn arg(name: &str, ty: &str) -> ArgSpec {
    ArgSpec {
        name: name.into(),
        ty: ty.into(),
    }
}

fn generic_account(name: &str, mutable: bool, signer: bool) -> AccountSpec {
    AccountSpec {
        name: name.into(),
        mutable,
        signer,
        seeds: None,
        kind: AccountKind::Generic,
    }
}

fn pda_account(name: &str, seeds: &[&str]) -> AccountSpec {
    AccountSpec {
        name: name.into(),
        mutable: true,
        signer: false,
        seeds: Some(seeds.iter().map(|seed| (*seed).to_string()).collect()),
        kind: AccountKind::Generic,
    }
}

fn program_account(name: &str, kind: AccountKind) -> AccountSpec {
    AccountSpec {
        name: name.into(),
        mutable: false,
        signer: false,
        seeds: None,
        kind,
    }
}

fn instruction_ctx(
    name: &str,
    args: Vec<ArgSpec>,
    accounts: Vec<AccountSpec>,
    emit: Option<&str>,
    emit_fields: Vec<&str>,
) -> InstructionCtx {
    InstructionCtx {
        program_name: "demo".into(),
        instruction_name: name.into(),
        args,
        accounts,
        emit: emit.map(str::to_string),
        emit_fields: emit_fields.into_iter().map(str::to_string).collect(),
    }
}

fn walk_sorted(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk_inner(root, &mut out);
    out.sort();
    out
}

fn walk_inner(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk_inner(&path, out);
        } else {
            out.push(path);
        }
    }
}

fn corpus(root: &Path) -> String {
    let mut out = String::new();
    for path in walk_sorted(root) {
        let rel = path.strip_prefix(root).unwrap();
        out.push_str("=== ");
        out.push_str(&rel.to_string_lossy().replace('\\', "/"));
        out.push_str(" ===\n");
        match std::fs::read_to_string(&path) {
            Ok(contents) => out.push_str(&contents),
            Err(_) => out.push_str("<binary>\n"),
        }
        if !out.ends_with('\n') {
            out.push('\n');
        }
    }
    out
}

#[test]
fn account_golden_matrix() {
    let cases = [
        (
            "account_empty_marker",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "Marker".into(),
                fields: vec![],
            },
        ),
        (
            "account_vault_totals",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "Vault".into(),
                fields: vec![
                    account_field("owner", "Pubkey"),
                    account_field("total", "u64"),
                ],
            },
        ),
        (
            "account_user_profile_casing",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "UserProfile".into(),
                fields: vec![
                    account_field("displayName", "String"),
                    account_field("ownerPubkey", "Pubkey"),
                ],
            },
        ),
        (
            "account_bytes_and_bool",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "Metadata".into(),
                fields: vec![
                    account_field("active", "bool"),
                    account_field("digest", "[u8; 32]"),
                ],
            },
        ),
        (
            "account_vec_payload",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "Blob".into(),
                fields: vec![account_field("data", "Vec<u8>")],
            },
        ),
        (
            "account_config_numbers",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "Config".into(),
                fields: vec![account_field("fee_bps", "u16"), account_field("bump", "u8")],
            },
        ),
        (
            "account_market_state",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "MarketState".into(),
                fields: vec![
                    account_field("asset", "Pubkey"),
                    account_field("quote_mint", "Pubkey"),
                    account_field("paused", "bool"),
                ],
            },
        ),
        (
            "account_order_book",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "OrderBook".into(),
                fields: vec![
                    account_field("next_order_id", "u128"),
                    account_field("depth", "u32"),
                ],
            },
        ),
        (
            "account_nft_record",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "NftRecord".into(),
                fields: vec![
                    account_field("mint", "Pubkey"),
                    account_field("uri", "String"),
                    account_field("royalty_bps", "u16"),
                ],
            },
        ),
        (
            "account_governance_vote",
            AccountCtx {
                program_name: "demo".into(),
                account_name: "GovernanceVote".into(),
                fields: vec![
                    account_field("proposal", "Pubkey"),
                    account_field("voter", "Pubkey"),
                    account_field("choice", "u8"),
                ],
            },
        ),
    ];
    for (name, ctx) in cases {
        snap(name, render_account_file(&ctx).unwrap());
    }

    snap(
        "account_mod_entry_vault",
        render_account_mod_entry("Vault").unwrap(),
    );
    snap(
        "account_mod_entry_user_profile",
        render_account_mod_entry("UserProfile").unwrap(),
    );
    snap(
        "account_mod_segment_sorted",
        render_account_mod_segment(&[
            "vault".into(),
            "treasury".into(),
            "UserProfile".into(),
            "vault".into(),
        ]),
    );
    snap(
        "account_mod_segment_single",
        render_account_mod_segment(&["Config".into()]),
    );
}

#[test]
fn event_golden_matrix() {
    let cases = [
        (
            "event_empty_ping",
            EventCtx {
                program_name: "demo".into(),
                event_name: "Ping".into(),
                fields: vec![],
            },
        ),
        (
            "event_deposited_amount",
            EventCtx {
                program_name: "demo".into(),
                event_name: "Deposited".into(),
                fields: vec![
                    event_field("authority", "Pubkey"),
                    event_field("amount", "u64"),
                ],
            },
        ),
        (
            "event_profile_updated_casing",
            EventCtx {
                program_name: "demo".into(),
                event_name: "ProfileUpdated".into(),
                fields: vec![
                    event_field("displayName", "String"),
                    event_field("active", "bool"),
                ],
            },
        ),
        (
            "event_market_opened",
            EventCtx {
                program_name: "demo".into(),
                event_name: "MarketOpened".into(),
                fields: vec![
                    event_field("asset", "Pubkey"),
                    event_field("quote_mint", "Pubkey"),
                ],
            },
        ),
        (
            "event_order_filled",
            EventCtx {
                program_name: "demo".into(),
                event_name: "OrderFilled".into(),
                fields: vec![
                    event_field("order_id", "u128"),
                    event_field("base_amount", "u64"),
                    event_field("quote_amount", "u64"),
                ],
            },
        ),
        (
            "event_nft_minted",
            EventCtx {
                program_name: "demo".into(),
                event_name: "NftMinted".into(),
                fields: vec![
                    event_field("mint", "Pubkey"),
                    event_field("owner", "Pubkey"),
                    event_field("uri", "String"),
                ],
            },
        ),
        (
            "event_configured",
            EventCtx {
                program_name: "demo".into(),
                event_name: "Configured".into(),
                fields: vec![
                    event_field("enabled", "bool"),
                    event_field("count", "u64"),
                    event_field("label", "String"),
                ],
            },
        ),
        (
            "event_vote_cast",
            EventCtx {
                program_name: "demo".into(),
                event_name: "VoteCast".into(),
                fields: vec![
                    event_field("proposal", "Pubkey"),
                    event_field("voter", "Pubkey"),
                    event_field("choice", "u8"),
                ],
            },
        ),
        (
            "event_closed_reason",
            EventCtx {
                program_name: "demo".into(),
                event_name: "Closed".into(),
                fields: vec![event_field("reason", "String")],
            },
        ),
        (
            "event_binary_digest",
            EventCtx {
                program_name: "demo".into(),
                event_name: "DigestRecorded".into(),
                fields: vec![event_field("digest", "[u8; 32]")],
            },
        ),
    ];
    for (name, ctx) in &cases {
        snap(name, render_event_entry(ctx).unwrap());
    }

    snap(
        "events_file_empty",
        render_events_file(&[cases[0].1.clone()]).unwrap(),
    );
    snap(
        "events_file_pair",
        render_events_file(&[cases[1].1.clone(), cases[6].1.clone()]).unwrap(),
    );
    snap(
        "events_file_three_entries",
        render_events_file(&[cases[3].1.clone(), cases[4].1.clone(), cases[5].1.clone()]).unwrap(),
    );
}

#[test]
fn error_golden_matrix() {
    let variants = [
        ErrorVariant {
            name: "Unauthorized".into(),
            message: "caller is not authorized".into(),
        },
        ErrorVariant {
            name: "MathOverflow".into(),
            message: "calculation overflowed".into(),
        },
        ErrorVariant {
            name: "AlreadyClosed".into(),
            message: "vault is already closed".into(),
        },
        ErrorVariant {
            name: "AlreadyOpen".into(),
            message: "vault is already open".into(),
        },
        ErrorVariant {
            name: "InvalidMint".into(),
            message: "mint does not match".into(),
        },
        ErrorVariant {
            name: "MissingBump".into(),
            message: "required bump was not found".into(),
        },
        ErrorVariant {
            name: "BadQuote".into(),
            message: "quote contained \"bad\" content".into(),
        },
        ErrorVariant {
            name: "PathIssue".into(),
            message: r#"path contains \ separator"#.into(),
        },
    ];
    for variant in &variants {
        snap(
            &format!("error_variant_{}", variant.name),
            render_error_variant(variant).unwrap(),
        );
    }

    let files = [
        ("errors_file_single", vec![variants[0].clone()]),
        (
            "errors_file_two_variants",
            vec![variants[0].clone(), variants[1].clone()],
        ),
        (
            "errors_file_market",
            vec![
                variants[2].clone(),
                variants[3].clone(),
                variants[4].clone(),
            ],
        ),
        (
            "errors_file_escaped_messages",
            vec![variants[6].clone(), variants[7].clone()],
        ),
        ("errors_file_all_variants", variants.to_vec()),
    ];
    for (name, variants) in files {
        snap(
            name,
            render_errors_file(&ErrorCtx {
                program_name: "demo".into(),
                enum_name: "demo_error".into(),
                variants,
            })
            .unwrap(),
        );
    }
}

#[test]
fn instruction_golden_matrix() {
    let cases = [
        (
            "instruction_no_accounts_no_args",
            instruction_ctx("heartbeat", vec![], vec![], None, vec![]),
        ),
        (
            "instruction_signer_only",
            instruction_ctx(
                "authorize",
                vec![],
                vec![generic_account("authority", false, true)],
                None,
                vec![],
            ),
        ),
        (
            "instruction_mut_pda",
            instruction_ctx(
                "initialize_vault",
                vec![arg("amount", "u64")],
                vec![
                    pda_account("vault", &["b\"vault\"", "authority.key().as_ref()"]),
                    generic_account("authority", true, true),
                    program_account("system_program", AccountKind::SystemProgram),
                ],
                None,
                vec![],
            ),
        ),
        (
            "instruction_token_programs",
            instruction_ctx(
                "transfer_tokens",
                vec![],
                vec![
                    program_account("token_program", AccountKind::TokenProgram),
                    program_account("ata_program", AccountKind::AssociatedTokenProgram),
                    generic_account("payer", false, true),
                ],
                None,
                vec![],
            ),
        ),
        (
            "instruction_varied_args",
            instruction_ctx(
                "configure",
                vec![
                    arg("enabled", "bool"),
                    arg("count", "u64"),
                    arg("label", "String"),
                    arg("owner", "Pubkey"),
                ],
                vec![generic_account("admin", true, true)],
                None,
                vec![],
            ),
        ),
        (
            "instruction_emit_empty_event",
            instruction_ctx(
                "ping",
                vec![arg("nonce", "u64")],
                vec![generic_account("user", false, true)],
                Some("Pinged"),
                vec![],
            ),
        ),
        (
            "instruction_emit_fielded_event",
            instruction_ctx(
                "deposit",
                vec![arg("amount", "u64")],
                vec![generic_account("payer", false, true)],
                Some("Deposited"),
                vec!["amount"],
            ),
        ),
        (
            "instruction_emit_many_fields",
            instruction_ctx(
                "configure_emit",
                vec![arg("enabled", "bool"), arg("label", "String")],
                vec![generic_account("admin", false, true)],
                Some("Configured"),
                vec!["enabled", "count", "label", "owner"],
            ),
        ),
        (
            "instruction_cased_names",
            instruction_ctx(
                "updateProfile",
                vec![arg("displayName", "String")],
                vec![generic_account("userProfile", true, false)],
                None,
                vec![],
            ),
        ),
        (
            "instruction_seed_and_token_mix",
            instruction_ctx(
                "mix_accounts",
                vec![],
                vec![
                    pda_account("vault", &["b\"vault\"", "payer.key().as_ref()"]),
                    generic_account("payer", false, true),
                    program_account("token_program", AccountKind::TokenProgram),
                    program_account("system_program", AccountKind::SystemProgram),
                ],
                None,
                vec![],
            ),
        ),
        (
            "instruction_large_numeric_args",
            instruction_ctx(
                "place_order",
                vec![arg("order_id", "u128"), arg("quantity", "u64")],
                vec![generic_account("trader", true, true)],
                None,
                vec![],
            ),
        ),
        (
            "instruction_close_vault",
            instruction_ctx(
                "close_vault",
                vec![],
                vec![
                    generic_account("authority", false, true),
                    pda_account("vault", &["b\"vault\"", "authority.key().as_ref()"]),
                ],
                Some("Closed"),
                vec!["reason"],
            ),
        ),
        (
            "instruction_nft_mint",
            instruction_ctx(
                "mint_nft",
                vec![arg("uri", "String")],
                vec![
                    generic_account("payer", true, true),
                    generic_account("mint", true, false),
                    program_account("system_program", AccountKind::SystemProgram),
                ],
                Some("NftMinted"),
                vec!["mint", "owner", "uri"],
            ),
        ),
        (
            "instruction_vote_cast",
            instruction_ctx(
                "cast_vote",
                vec![arg("choice", "u8")],
                vec![generic_account("voter", false, true)],
                Some("VoteCast"),
                vec!["proposal", "voter", "choice"],
            ),
        ),
        (
            "instruction_blob_write",
            instruction_ctx(
                "write_blob",
                vec![arg("data", "Vec<u8>")],
                vec![generic_account("writer", false, true)],
                None,
                vec![],
            ),
        ),
        (
            "instruction_digest_record",
            instruction_ctx(
                "record_digest",
                vec![arg("digest", "[u8; 32]")],
                vec![generic_account("authority", false, true)],
                Some("DigestRecorded"),
                vec!["digest"],
            ),
        ),
    ];
    for (name, ctx) in cases {
        snap(name, render_instruction(&ctx).unwrap());
    }

    snap(
        "instructions_mod_segment_sorted",
        render_instructions_mod_segment(&[
            "deposit".into(),
            "initialize".into(),
            "deposit".into(),
            "close_vault".into(),
        ]),
    );
    snap(
        "instructions_mod_segment_single",
        render_instructions_mod_segment(&["ping".into()]),
    );
    snap(
        "dispatch_segment_with_args",
        render_dispatch_segment(
            "demo",
            &[
                InstructionDispatch {
                    name: "initialize".into(),
                    args: vec![arg("seed", "u64")],
                },
                InstructionDispatch {
                    name: "configure".into(),
                    args: vec![arg("enabled", "bool"), arg("label", "String")],
                },
            ],
        ),
    );
    snap(
        "dispatch_segment_no_args",
        render_dispatch_segment(
            "demo",
            &[InstructionDispatch {
                name: "ping".into(),
                args: vec![],
            }],
        ),
    );
}

#[test]
fn program_golden_matrix() {
    let cases = [
        (
            "program_rewards_default_id",
            json!({
                "project_name": "Rewards App",
                "program_name": "rewards",
                "anchor_version": "0.30.1",
                "rust_edition": "2021",
            }),
        ),
        (
            "program_treasury_custom_id",
            json!({
                "project_name": "Treasury App",
                "program_name": "treasury",
                "program_id": "11111111111111111111111111111111",
                "anchor_version": "0.30.1",
                "rust_edition": "2021",
            }),
        ),
        (
            "program_cased_name",
            json!({
                "project_name": "My Cased Dapp",
                "program_name": "GovernanceCore",
                "anchor_version": "0.30.1",
                "rust_edition": "2021",
            }),
        ),
        (
            "program_market_maker",
            json!({
                "project_name": "Market Maker",
                "program_name": "market_maker",
                "anchor_version": "0.30.1",
                "rust_edition": "2021",
            }),
        ),
        (
            "program_nft_factory",
            json!({
                "project_name": "NFT Factory",
                "program_name": "nft_factory",
                "anchor_version": "0.30.1",
                "rust_edition": "2021",
            }),
        ),
        (
            "program_edition_2024",
            json!({
                "project_name": "Future App",
                "program_name": "future_program",
                "anchor_version": "0.30.1",
                "rust_edition": "2024",
            }),
        ),
        (
            "program_anchor_version_override",
            json!({
                "project_name": "Anchor Override",
                "program_name": "anchor_override",
                "anchor_version": "0.31.0",
                "rust_edition": "2021",
            }),
        ),
    ];

    for (name, ctx) in cases {
        let tmp = tempfile::tempdir().unwrap();
        render_program(&ctx, tmp.path()).unwrap();
        snap(name, corpus(tmp.path()));
    }
}
