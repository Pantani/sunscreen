//! Beginner onboarding command surface.

use crate::error::SunscreenError;
pub use crate::onboarding::args::{
    DeployArgs, ExampleNameArgs, ExampleUseArgs, ExamplesCmd, ExamplesListArgs, InitArgs,
    LearnArgs, QuickstartArgs, QuickstartRecipeArg, WalletAirdropArgs, WalletBalanceArgs,
    WalletCmd, WalletNewArgs, WalletSetDefaultArgs,
};
use crate::onboarding::recipes::RecipeApplier;

struct CliRecipeApplier;

impl RecipeApplier for CliRecipeApplier {
    fn apply_spl_token(
        &self,
        name: &str,
        program: &str,
        workspace_root: &std::path::Path,
    ) -> Result<(), SunscreenError> {
        crate::cli::scaffold::run_spl_token_quiet(
            &crate::cli::scaffold::BuiltinRecipeArgs {
                name: name.to_string(),
                program: program.to_string(),
                dry_run: false,
            },
            workspace_root,
        )
        .map(|_| ())
    }

    fn apply_metaplex_nft(
        &self,
        name: &str,
        program: &str,
        workspace_root: &std::path::Path,
    ) -> Result<(), SunscreenError> {
        crate::cli::scaffold::run_metaplex_nft_quiet(
            &crate::cli::scaffold::BuiltinRecipeArgs {
                name: name.to_string(),
                program: program.to_string(),
                dry_run: false,
            },
            workspace_root,
        )
        .map(|_| ())
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_crud(
        &self,
        name: &str,
        program: &str,
        fields: &str,
        no_update: bool,
        no_delete: bool,
        no_events: bool,
        no_frontend: bool,
        workspace_root: &std::path::Path,
    ) -> Result<(), SunscreenError> {
        crate::cli::scaffold::run_crud_quiet(
            &crate::cli::scaffold::CrudArgs {
                name: name.to_string(),
                program: program.to_string(),
                fields: fields.to_string(),
                no_update,
                no_delete,
                no_events,
                no_frontend,
                dry_run: false,
            },
            workspace_root,
        )
        .map(|_| ())
    }
}

pub fn run_init(args: &InitArgs, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::wizard::run(args, json, &CliRecipeApplier)
}

pub fn run_examples(cmd: &ExamplesCmd, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::examples::run(cmd, json, &CliRecipeApplier)
}

pub fn run_quickstart(args: &QuickstartArgs, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::recipes::run(args, json, &CliRecipeApplier)
}

pub fn run_wallet(cmd: &WalletCmd, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::wallet::run(cmd, json)
}

pub fn run_deploy(args: &DeployArgs, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::deploy::run(args, json)
}

pub fn run_learn(args: &LearnArgs, json: bool) -> Result<i32, SunscreenError> {
    crate::onboarding::learn::run(args, json)
}
