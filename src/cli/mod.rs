//! Command-line interface surface for sunscreen.

pub mod chain;
pub mod doctor;
pub mod generate;
#[cfg(feature = "onboarding")]
pub mod onboarding;
pub mod root;
pub mod scaffold;
pub mod version;

pub use root::execute;
