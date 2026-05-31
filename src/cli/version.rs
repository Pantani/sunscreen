//! `sunscreen version` subcommand.

/// Print the sunscreen version string.
pub fn run() {
    println!("sunscreen {}", env!("CARGO_PKG_VERSION"));
}
