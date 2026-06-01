//! Golden snapshot tests for the `event` scaffold renderer.
//!
//! Regenerate snapshots with `INSTA_UPDATE=auto cargo test`.
//!
//! Matrix covered:
//! - single event, single field
//! - event with no fields
//! - event with many fields / mixed types
//! - full `events.rs` file with one entry
//! - full `events.rs` file with multiple entries (declaration-order preserved)
//! - empty `events.rs` (zero entries) — markers still present
//! - idempotency: render twice → identical bytes

use sunscreen::templates::event::FieldSpec;
use sunscreen::templates::{render_event_entry, render_events_file, EventCtx};

fn ev_ping() -> EventCtx {
    EventCtx {
        program_name: "demo".into(),
        event_name: "ping".into(),
        fields: vec![],
    }
}

fn ev_deposited() -> EventCtx {
    EventCtx {
        program_name: "demo".into(),
        event_name: "deposited".into(),
        fields: vec![FieldSpec {
            name: "amount".into(),
            ty: "u64".into(),
        }],
    }
}

fn ev_complex() -> EventCtx {
    EventCtx {
        program_name: "demo".into(),
        event_name: "trade_settled".into(),
        fields: vec![
            FieldSpec {
                name: "buyer".into(),
                ty: "Pubkey".into(),
            },
            FieldSpec {
                name: "seller".into(),
                ty: "Pubkey".into(),
            },
            FieldSpec {
                name: "price".into(),
                ty: "u64".into(),
            },
            FieldSpec {
                name: "memo".into(),
                ty: "String".into(),
            },
        ],
    }
}

#[test]
fn event_entry_no_fields() {
    let out = render_event_entry(&ev_ping()).expect("render");
    insta::assert_snapshot!("event_entry_no_fields", out);
}

#[test]
fn event_entry_single_field() {
    let out = render_event_entry(&ev_deposited()).expect("render");
    insta::assert_snapshot!("event_entry_single_field", out);
}

#[test]
fn event_entry_many_fields() {
    let out = render_event_entry(&ev_complex()).expect("render");
    insta::assert_snapshot!("event_entry_many_fields", out);
}

#[test]
fn events_file_single_entry() {
    let out = render_events_file(&[ev_deposited()]).expect("render");
    insta::assert_snapshot!("events_file_single_entry", out);
}

#[test]
fn events_file_multiple_entries_order_preserved() {
    // Note: ordering follows the slice — NOT alphabetic.
    let out = render_events_file(&[ev_deposited(), ev_ping(), ev_complex()]).expect("render");
    insta::assert_snapshot!("events_file_multiple_entries_order_preserved", out);
}

#[test]
fn events_file_empty() {
    let out = render_events_file(&[]).expect("render");
    insta::assert_snapshot!("events_file_empty", out);
}

#[test]
fn events_file_idempotent() {
    let a = render_events_file(&[ev_deposited(), ev_ping()]).expect("render");
    let b = render_events_file(&[ev_deposited(), ev_ping()]).expect("render");
    assert_eq!(a, b);
}
