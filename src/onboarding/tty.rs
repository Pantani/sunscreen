//! TTY / non-interactive helpers for onboarding commands.

use std::io::IsTerminal;

/// True when stdin can safely prompt and the caller did not force CI mode.
#[must_use]
pub fn is_interactive(non_interactive: bool) -> bool {
    if non_interactive {
        return false;
    }
    if std::env::var_os("SUNSCREEN_NON_INTERACTIVE")
        .map(|value| !value.is_empty() && value != "0")
        .unwrap_or(false)
    {
        return false;
    }
    std::io::stdin().is_terminal()
}
