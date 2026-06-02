//! Metaplex NFT recipe plan.

use heck::{ToKebabCase, ToPascalCase, ToSnakeCase};

use crate::scaffold::{generated_file, RecipeKind, RecipePlan, RecipeStep};

/// Options for `sunscreen scaffold metaplex-nft`.
#[derive(Debug, Clone)]
pub struct MetaplexNftRecipeOptions {
    pub name: String,
}

/// Build the Metaplex NFT recipe expansion.
pub fn build(options: MetaplexNftRecipeOptions) -> RecipePlan {
    let snake = options.name.to_snake_case();
    let pascal = snake.to_pascal_case();
    let mut steps = vec![
        RecipeStep::Account {
            name: pascal.clone(),
            fields: "authority:Pubkey,collection_mint:Pubkey,items:u64".to_string(),
        },
        RecipeStep::Event {
            name: format!("{pascal}Created"),
            fields: "collection:Pubkey".to_string(),
        },
        RecipeStep::Event {
            name: format!("{pascal}Minted"),
            fields: "collection:Pubkey,mint:Pubkey".to_string(),
        },
        RecipeStep::Event {
            name: format!("{pascal}Verified"),
            fields: "collection:Pubkey,mint:Pubkey".to_string(),
        },
        RecipeStep::Error {
            name: "InvalidMetadata".to_string(),
            message: "metadata account does not match this NFT recipe".to_string(),
        },
        RecipeStep::Error {
            name: format!("{pascal}Unauthorized"),
            message: format!("caller cannot mutate {}", snake),
        },
    ];

    steps.extend([
        RecipeStep::Instruction {
            name: format!("create_{snake}"),
            args: "name:String,symbol:String,uri:String".to_string(),
            accounts: nft_accounts(&snake, true),
            emit: Some(format!("{pascal}Created")),
        },
        RecipeStep::Instruction {
            name: format!("mint_{snake}"),
            args: "uri:String".to_string(),
            accounts: nft_accounts(&snake, true),
            emit: Some(format!("{pascal}Minted")),
        },
        RecipeStep::Instruction {
            name: format!("verify_{snake}"),
            args: String::new(),
            accounts: nft_accounts(&snake, true),
            emit: Some(format!("{pascal}Verified")),
        },
    ]);

    RecipePlan {
        kind: RecipeKind::MetaplexNft,
        resource: snake.clone(),
        steps,
        files: vec![recipe_test_file("metaplex-nft", &snake)],
    }
}

fn nft_accounts(resource: &str, mutable_resource: bool) -> String {
    let flags = if mutable_resource {
        "mut|seeds=b\"nft\";authority.key().as_ref()"
    } else {
        "seeds=b\"nft\";authority.key().as_ref()"
    };
    format!(
        "{resource}:{flags},authority:signer,collection_mint:mut,metadata:mut,master_edition:mut,token_program,associated_token_program,system_program"
    )
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
