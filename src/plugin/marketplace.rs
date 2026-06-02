//! Built-in marketplace index for reference plugins.
//!
//! This is an offline reference index. It documents the canonical plugin names
//! and transports without downloading remote artifacts.

use serde_json::json;

#[derive(Debug, Clone, Copy)]
pub struct MarketplaceEntry {
    pub name: &'static str,
    pub source: &'static str,
    pub version: &'static str,
    pub transport: &'static str,
    pub summary: &'static str,
}

pub const REFERENCE_PLUGINS: &[MarketplaceEntry] = &[
    MarketplaceEntry {
        name: "spl-token-2022",
        source: "sunscreen-apps/spl-token-2022",
        version: "v0.4.1",
        transport: "grpc",
        summary: "Token-2022 transfer-hook and confidential-transfer scaffolding",
    },
    MarketplaceEntry {
        name: "yellowstone-indexer",
        source: "sunscreen-apps/yellowstone-indexer",
        version: "v0.2.0",
        transport: "stdio-jsonrpc",
        summary: "Yellowstone/Vixen indexer scaffolding derived from Anchor IDLs",
    },
];

pub fn as_json() -> Vec<serde_json::Value> {
    REFERENCE_PLUGINS
        .iter()
        .map(|entry| {
            json!({
                "name": entry.name,
                "source": entry.source,
                "version": entry.version,
                "transport": entry.transport,
                "summary": entry.summary,
                "status": "reference",
            })
        })
        .collect()
}
