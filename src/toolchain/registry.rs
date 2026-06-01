//! Static registry of known toolchain entries.
//!
//! Each [`ToolSpec`] is a declarative description of how to invoke a CLI
//! tool to discover its version. The list is intentionally small and
//! curated — adding a new tool means appending one entry here.

/// Declarative description of an external tool we know how to probe.
#[derive(Debug, Clone, Copy)]
pub struct ToolSpec {
    /// Logical name surfaced in reports / config keys.
    pub name: &'static str,
    /// Executable name to resolve via `which`.
    pub bin: &'static str,
    /// Argument that prints the version (typically `--version`).
    pub version_arg: &'static str,
    /// Regex with a single capture group around `MAJOR.MINOR.PATCH`.
    pub version_regex: &'static str,
    /// Whether the tool is required for sunscreen to function.
    pub required: bool,
    /// Default minimum version when no override is supplied.
    pub default_min: Option<&'static str>,
}

/// All tools known to the detector.
#[must_use]
pub fn known() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "anchor",
            bin: "anchor",
            version_arg: "--version",
            version_regex: r"anchor-cli (\d+\.\d+\.\d+)",
            required: true,
            default_min: Some("1.0.0"),
        },
        ToolSpec {
            name: "solana",
            bin: "solana",
            version_arg: "--version",
            version_regex: r"solana-cli (\d+\.\d+\.\d+)",
            required: true,
            default_min: Some("2.0.0"),
        },
        ToolSpec {
            name: "rustc",
            bin: "rustc",
            version_arg: "--version",
            version_regex: r"rustc (\d+\.\d+\.\d+)",
            required: true,
            default_min: Some("1.75.0"),
        },
        ToolSpec {
            name: "cargo",
            bin: "cargo",
            version_arg: "--version",
            version_regex: r"cargo (\d+\.\d+\.\d+)",
            required: true,
            default_min: Some("1.75.0"),
        },
        ToolSpec {
            name: "node",
            bin: "node",
            version_arg: "--version",
            version_regex: r"v(\d+\.\d+\.\d+)",
            required: true,
            default_min: Some("20.0.0"),
        },
        ToolSpec {
            name: "pnpm",
            bin: "pnpm",
            version_arg: "--version",
            version_regex: r"(\d+\.\d+\.\d+)",
            required: true,
            default_min: Some("9.0.0"),
        },
        ToolSpec {
            name: "codama",
            bin: "codama",
            version_arg: "--version",
            version_regex: r"(\d+\.\d+\.\d+)",
            required: false,
            default_min: None,
        },
        ToolSpec {
            name: "surfpool",
            bin: "surfpool",
            version_arg: "--version",
            version_regex: r"(\d+\.\d+\.\d+)",
            required: false,
            default_min: None,
        },
        ToolSpec {
            name: "rustfmt",
            bin: "rustfmt",
            version_arg: "--version",
            version_regex: r"rustfmt (\d+\.\d+\.\d+)",
            required: false,
            default_min: None,
        },
    ]
}
