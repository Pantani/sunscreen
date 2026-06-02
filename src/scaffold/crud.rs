//! CRUD recipe plan.

use heck::{ToKebabCase, ToLowerCamelCase, ToPascalCase, ToSnakeCase};

use super::{generated_file, GeneratedFile, RecipeKind, RecipePlan, RecipeStep};

const DEFAULT_FIELDS: &str = "authority:Pubkey,title:String,body:String,published:bool";

/// Options for `sunscreen scaffold crud`.
#[derive(Debug, Clone)]
pub struct CrudRecipeOptions {
    pub resource: String,
    pub fields: String,
    pub include_update: bool,
    pub include_delete: bool,
    pub include_events: bool,
    pub include_frontend: bool,
    pub frontend_root: Option<String>,
}

/// Build a CRUD recipe expansion.
pub fn build(options: CrudRecipeOptions) -> RecipePlan {
    let resource_snake = options.resource.to_snake_case();
    let resource_pascal = resource_snake.to_pascal_case();
    let fields = if options.fields.trim().is_empty() {
        DEFAULT_FIELDS.to_string()
    } else {
        options.fields.clone()
    };

    let mut steps = vec![RecipeStep::Account {
        name: resource_pascal.clone(),
        fields: fields.clone(),
    }];

    if options.include_events {
        steps.extend([
            RecipeStep::Event {
                name: format!("{resource_pascal}Created"),
                fields: "resource:Pubkey".to_string(),
            },
            RecipeStep::Event {
                name: format!("{resource_pascal}Updated"),
                fields: "resource:Pubkey".to_string(),
            },
        ]);
        if options.include_delete {
            steps.push(RecipeStep::Event {
                name: format!("{resource_pascal}Deleted"),
                fields: "resource:Pubkey".to_string(),
            });
        }
    }

    steps.extend([
        RecipeStep::Error {
            name: format!("{resource_pascal}NotFound"),
            message: format!("{resource_pascal} was not found"),
        },
        RecipeStep::Error {
            name: format!("{resource_pascal}Unauthorized"),
            message: format!("caller cannot mutate this {}", resource_snake),
        },
    ]);

    steps.push(RecipeStep::Instruction {
        name: format!("create_{resource_snake}"),
        args: fields.clone(),
        accounts: crud_accounts(&resource_snake, true),
        emit: options
            .include_events
            .then(|| format!("{resource_pascal}Created")),
    });
    steps.push(RecipeStep::Instruction {
        name: format!("read_{resource_snake}"),
        args: String::new(),
        accounts: crud_accounts(&resource_snake, false),
        emit: None,
    });
    if options.include_update {
        steps.push(RecipeStep::Instruction {
            name: format!("update_{resource_snake}"),
            args: fields.clone(),
            accounts: crud_accounts(&resource_snake, true),
            emit: options
                .include_events
                .then(|| format!("{resource_pascal}Updated")),
        });
    }
    if options.include_delete {
        steps.push(RecipeStep::Instruction {
            name: format!("delete_{resource_snake}"),
            args: String::new(),
            accounts: crud_accounts(&resource_snake, true),
            emit: options
                .include_events
                .then(|| format!("{resource_pascal}Deleted")),
        });
    }

    let mut files = vec![crud_test_file(&resource_snake, &steps)];
    if options.include_frontend {
        if let Some(frontend_root) = options.frontend_root {
            files.push(crud_hook_file(
                &frontend_root,
                &resource_snake,
                options.include_update,
                options.include_delete,
            ));
        }
    }

    RecipePlan {
        kind: RecipeKind::Crud,
        resource: resource_snake,
        steps,
        files,
    }
}

fn crud_accounts(resource: &str, mutable_resource: bool) -> String {
    let resource_flags = if mutable_resource {
        "mut|seeds=b\"resource\";authority.key().as_ref()"
    } else {
        "seeds=b\"resource\";authority.key().as_ref()"
    };
    format!("{resource}:{resource_flags},authority:signer,system_program")
}

fn crud_test_file(resource: &str, steps: &[RecipeStep]) -> GeneratedFile {
    let kebab = resource.to_kebab_case();
    let method_assertions = steps
        .iter()
        .filter_map(|step| match step {
            RecipeStep::Instruction { name, .. } => Some(name.to_lower_camel_case()),
            _ => None,
        })
        .map(|method| format!("    expectMethod(\"{method}\");\n"))
        .collect::<String>();
    let contents = format!(
        "import * as anchor from \"@coral-xyz/anchor\";\n\n\
         describe(\"{kebab} recipe\", () => {{\n  \
         anchor.setProvider(anchor.AnchorProvider.env());\n\n  \
         const program = anchor.workspace[Object.keys(anchor.workspace)[0]];\n\n  \
         function expectMethod(name: string) {{\n    \
         if (!program?.methods?.[name]) throw new Error(`missing ${{name}} method`);\n  \
         }}\n\n  \
         it(\"exposes generated CRUD methods\", () => {{\n{method_assertions}  \
         }});\n\
         }});\n"
    );
    generated_file(format!("tests/__PROGRAM__/{resource}.test.ts"), contents)
}

fn crud_hook_file(
    frontend_root: &str,
    resource: &str,
    include_update: bool,
    include_delete: bool,
) -> GeneratedFile {
    let kebab = resource.to_kebab_case();
    let pascal = resource.to_pascal_case();
    let camel = resource.to_lower_camel_case();
    let mut contents = format!(
        "import {{ useMutation, useQuery }} from \"@tanstack/react-query\";\n\n\
         type RpcFn = (input?: unknown) => Promise<unknown>;\n\n\
         export function use{pascal}(address?: string) {{\n  \
         return useQuery({{\n    queryKey: [\"sunscreen\", \"{kebab}\", address],\n    \
         enabled: Boolean(address),\n    queryFn: async () => ({{ address }}),\n  }});\n\
         }}\n\n\
         export function useCreate{pascal}(rpc?: RpcFn) {{\n  \
         return useMutation({{\n    mutationKey: [\"sunscreen\", \"create\", \"{kebab}\"],\n    \
         mutationFn: async (input?: unknown) => (rpc ? rpc(input) : input),\n  }});\n\
         }}\n"
    );
    if include_update {
        contents.push_str(&format!(
            "\nexport function useUpdate{pascal}(rpc?: RpcFn) {{\n  \
             return useMutation({{\n    mutationKey: [\"sunscreen\", \"update\", \"{kebab}\"],\n    \
             mutationFn: async (input?: unknown) => (rpc ? rpc(input) : input),\n  }});\n\
             }}\n"
        ));
    }
    if include_delete {
        contents.push_str(&format!(
            "\nexport function useDelete{pascal}(rpc?: RpcFn) {{\n  \
             return useMutation({{\n    mutationKey: [\"sunscreen\", \"delete\", \"{kebab}\"],\n    \
             mutationFn: async (input?: unknown) => (rpc ? rpc(input) : input),\n  }});\n\
             }}\n"
        ));
    }
    contents.push_str(&format!(
        "\nexport const {camel}Recipe = {{ resource: \"{resource}\", hook: \"use{pascal}\" }} as const;\n"
    ));
    generated_file(format!("{frontend_root}/src/hooks/use-{kebab}.ts"), contents)
}
