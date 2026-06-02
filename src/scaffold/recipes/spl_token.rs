//! SPL token recipe plan.

use heck::{ToKebabCase, ToPascalCase, ToSnakeCase};

use crate::scaffold::{generated_file, RecipeKind, RecipePlan, RecipeStep};

/// Options for `sunscreen scaffold spl-token`.
#[derive(Debug, Clone)]
pub struct SplTokenRecipeOptions {
    pub name: String,
}

/// Build the SPL token recipe expansion.
pub fn build(options: SplTokenRecipeOptions) -> RecipePlan {
    let snake = options.name.to_snake_case();
    let pascal = snake.to_pascal_case();
    let mut steps = vec![
        RecipeStep::Account {
            name: pascal.clone(),
            fields: "authority:Pubkey,mint:Pubkey,total_supply:u64".to_string(),
        },
        RecipeStep::Event {
            name: format!("{pascal}Initialized"),
            fields: "mint:Pubkey".to_string(),
        },
        RecipeStep::Event {
            name: format!("{pascal}Minted"),
            fields: "mint:Pubkey,amount:u64".to_string(),
        },
        RecipeStep::Event {
            name: format!("{pascal}Transferred"),
            fields: "mint:Pubkey,amount:u64".to_string(),
        },
        RecipeStep::Error {
            name: "InvalidMint".to_string(),
            message: "mint does not match this token recipe".to_string(),
        },
        RecipeStep::Error {
            name: format!("{pascal}Unauthorized"),
            message: format!("caller cannot mutate {}", snake),
        },
    ];

    steps.extend([
        RecipeStep::Instruction {
            name: format!("initialize_{snake}"),
            args: String::new(),
            accounts: token_accounts(&snake, true),
            emit: Some(format!("{pascal}Initialized")),
        },
        RecipeStep::Instruction {
            name: format!("mint_{snake}"),
            args: "amount:u64".to_string(),
            accounts: token_accounts(&snake, true),
            emit: Some(format!("{pascal}Minted")),
        },
        RecipeStep::Instruction {
            name: format!("transfer_{snake}"),
            args: "amount:u64".to_string(),
            accounts: token_accounts(&snake, true),
            emit: Some(format!("{pascal}Transferred")),
        },
    ]);

    RecipePlan {
        kind: RecipeKind::SplToken,
        resource: snake.clone(),
        steps,
        files: vec![recipe_test_file("spl-token", &snake)],
    }
}

fn token_accounts(resource: &str, mutable_resource: bool) -> String {
    let flags = if mutable_resource {
        "mut|seeds=b\"token\";authority.key().as_ref()"
    } else {
        "seeds=b\"token\";authority.key().as_ref()"
    };
    format!("{resource}:{flags},authority:signer,mint:mut,token_program,associated_token_program")
}

fn recipe_test_file(recipe: &str, resource: &str) -> crate::scaffold::GeneratedFile {
    let kebab = resource.to_kebab_case();
    generated_file(
        format!("tests/__PROGRAM__/{kebab}-{recipe}.test.ts"),
        format!(
            "import * as anchor from \"@coral-xyz/anchor\";\n\n\
             describe(\"{kebab} {recipe} recipe\", () => {{\n  \
             anchor.setProvider(anchor.AnchorProvider.env());\n\n  \
             it(\"loads recipe scaffold\", () => {{\n    \
             const program = anchor.workspace[Object.keys(anchor.workspace)[0]];\n    \
             if (!program) throw new Error(\"program not loaded\");\n  \
             }});\n\
             }});\n"
        ),
    )
}
