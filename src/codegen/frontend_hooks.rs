//! Frontend hook generation derived from Anchor IDLs.

use std::path::{Path, PathBuf};

use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use serde::Deserialize;

use super::idl::{export_idls, IdlExportOptions};
use super::{
    ensure_safe_relative_subpath, relative_path, sorted_json_files, write_if_changed, CodegenError,
    FileWrite,
};

/// Hook target selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HookTarget {
    /// Generate React Query hooks.
    React,
    /// Generate Solid Query hooks.
    Solid,
    /// Generate both React and Solid hooks.
    All,
}

/// Options for `generate frontend-hooks`.
#[derive(Debug, Clone)]
pub struct FrontendHooksOptions {
    /// Optional program name.
    pub program: Option<String>,
    /// Explicit frontend root relative to the workspace.
    pub frontend_path: Option<PathBuf>,
    /// Hook target set.
    pub target: HookTarget,
}

impl Default for FrontendHooksOptions {
    fn default() -> Self {
        Self {
            program: None,
            frontend_path: None,
            target: HookTarget::All,
        }
    }
}

/// Hook generation report.
#[derive(Debug, Clone)]
pub struct FrontendHooksReport {
    /// Files written or checked.
    pub files: Vec<FileWrite>,
}

#[derive(Debug, Deserialize)]
struct AnchorIdl {
    #[serde(default)]
    address: Option<String>,
    #[serde(default)]
    metadata: IdlMetadata,
    #[serde(default)]
    instructions: Vec<IdlInstruction>,
}

#[derive(Debug, Default, Deserialize)]
struct IdlMetadata {
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize)]
struct IdlInstruction {
    name: String,
    #[serde(default)]
    args: Vec<IdlArg>,
}

#[derive(Debug, Deserialize)]
struct IdlArg {
    name: String,
    #[serde(rename = "type")]
    ty: serde_json::Value,
}

#[derive(Debug)]
struct ProgramHooks {
    key: String,
    program_id: String,
    idl_json: serde_json::Value,
    instructions: Vec<InstructionHooks>,
}

#[derive(Debug)]
struct InstructionHooks {
    name: String,
    pascal: String,
    input_type: String,
}

/// Generate TypeScript IDL/core files plus TanStack React/Solid Query hooks.
pub fn generate_frontend_hooks(
    workspace_root: &Path,
    options: &FrontendHooksOptions,
) -> Result<FrontendHooksReport, CodegenError> {
    let idl_report = export_idls(
        workspace_root,
        &IdlExportOptions {
            program: options.program.clone(),
            ..IdlExportOptions::default()
        },
    )?;
    let frontend_root = resolve_frontend_root(workspace_root, options.frontend_path.as_deref())?;
    let generated = frontend_root.join("src/generated/sunscreen");
    let programs = load_program_hooks(workspace_root)?;

    let mut files = idl_report.files;
    files.push(write_if_changed(
        &generated.join("idl.ts"),
        &render_idl_ts(&programs)?,
    )?);
    files.push(write_if_changed(
        &generated.join("core.ts"),
        &render_core_ts(&programs),
    )?);
    if matches!(options.target, HookTarget::React | HookTarget::All) {
        files.push(write_if_changed(
            &generated.join("react.ts"),
            &render_react_ts(&programs),
        )?);
    }
    if matches!(options.target, HookTarget::Solid | HookTarget::All) {
        files.push(write_if_changed(
            &generated.join("solid.ts"),
            &render_solid_ts(&programs),
        )?);
    }
    files.push(write_if_changed(
        &generated.join("index.ts"),
        &render_index_ts(options.target),
    )?);

    Ok(FrontendHooksReport { files })
}

fn resolve_frontend_root(
    workspace_root: &Path,
    explicit: Option<&Path>,
) -> Result<PathBuf, CodegenError> {
    if let Some(path) = explicit {
        ensure_safe_relative_subpath("--frontend-path", path)?;
        return Ok(workspace_root.join(path));
    }

    let ws = crate::workspace::find_root(Some(workspace_root))?;
    if ws.config.workspace.frontend == crate::config::schema::Frontend::None {
        return Err(CodegenError::UserInput(
            "workspace has no frontend; pass --frontend-path to generate hooks explicitly".into(),
        ));
    }
    let rel = ws
        .config
        .workspace
        .frontend_path
        .as_deref()
        .unwrap_or("app");
    Ok(workspace_root.join(rel))
}

fn load_program_hooks(workspace_root: &Path) -> Result<Vec<ProgramHooks>, CodegenError> {
    let files = sorted_json_files(&workspace_root.join("clients/idl"))?;
    if files.is_empty() {
        return Err(CodegenError::UserInput(
            "no exported IDL files found under clients/idl".into(),
        ));
    }
    let mut programs = Vec::new();
    for path in files {
        let raw = std::fs::read_to_string(&path).map_err(|err| CodegenError::io(&path, err))?;
        let idl: AnchorIdl =
            serde_json::from_str(&raw).map_err(|err| CodegenError::json(&path, err))?;
        let idl_json: serde_json::Value =
            serde_json::from_str(&raw).map_err(|err| CodegenError::json(&path, err))?;
        let fallback = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("program")
            .to_string();
        let key = idl.metadata.name.unwrap_or(fallback).to_snake_case();
        let program_id = idl
            .address
            .unwrap_or_else(|| "11111111111111111111111111111111".to_string());
        let instructions = idl
            .instructions
            .into_iter()
            .map(|ix| InstructionHooks {
                pascal: ix.name.to_pascal_case(),
                input_type: render_input_type(&ix.args),
                name: ix.name.to_lower_camel_case(),
            })
            .collect();
        programs.push(ProgramHooks {
            key,
            program_id,
            idl_json,
            instructions,
        });
    }
    Ok(programs)
}

fn render_idl_ts(programs: &[ProgramHooks]) -> Result<String, CodegenError> {
    let mut out = String::from("// Generated by sunscreen. Do not edit.\n\n");
    out.push_str("export const SUNSCREEN_IDLS = {\n");
    for program in programs {
        let rendered = serde_json::to_string_pretty(&program.idl_json)
            .map_err(|err| CodegenError::json("clients/idl", err))?;
        out.push_str(&format!(
            "  {}: {},\n",
            json_string(&program.key),
            indent_json(&rendered, 2)
        ));
    }
    out.push_str("} as const;\n\n");
    out.push_str("export type SunscreenIdlName = keyof typeof SUNSCREEN_IDLS;\n");
    Ok(out)
}

fn render_core_ts(programs: &[ProgramHooks]) -> String {
    let default = programs
        .first()
        .map(|p| p.key.as_str())
        .unwrap_or("program");
    let mut out = String::from("// Generated by sunscreen. Do not edit.\n\n");
    out.push_str("import { SUNSCREEN_IDLS } from \"./idl\";\n\n");
    out.push_str("export { SUNSCREEN_IDLS };\n\n");
    out.push_str("export const DEFAULT_RPC_ENDPOINT = \"http://127.0.0.1:8899\";\n\n");
    out.push_str("export const SUNSCREEN_PROGRAMS = {\n");
    for program in programs {
        out.push_str(&format!(
            "  {}: {{ name: {}, programId: {}, idl: SUNSCREEN_IDLS[{}] }},\n",
            json_string(&program.key),
            json_string(&program.key),
            json_string(&program.program_id),
            json_string(&program.key)
        ));
    }
    out.push_str("} as const;\n\n");
    out.push_str("export type SunscreenProgramName = keyof typeof SUNSCREEN_PROGRAMS;\n");
    out.push_str(&format!(
        "export const DEFAULT_PROGRAM_NAME = {} as SunscreenProgramName;\n",
        json_string(default)
    ));
    out.push_str(
        "export const DEFAULT_PROGRAM_ID = SUNSCREEN_PROGRAMS[DEFAULT_PROGRAM_NAME].programId;\n\n",
    );
    out.push_str("export type JsonRpcResponse<T> = { jsonrpc: \"2.0\"; id: number | string; result?: T; error?: { code: number; message: string; data?: unknown } };\n");
    out.push_str("export type RpcTransport = { request<T>(method: string, params?: unknown[]): Promise<T> };\n\n");
    out.push_str(
        r#"export function createSurfpoolRpc(endpoint = DEFAULT_RPC_ENDPOINT): RpcTransport {
  return {
    async request<T>(method: string, params: unknown[] = []): Promise<T> {
      const response = await fetch(endpoint, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ jsonrpc: "2.0", id: 1, method, params }),
      });
      const payload = (await response.json()) as JsonRpcResponse<T>;
      if (payload.error) {
        throw new Error(payload.error.message);
      }
      return payload.result as T;
    },
  };
}

export function programIdFor(name: SunscreenProgramName = DEFAULT_PROGRAM_NAME): string {
  return SUNSCREEN_PROGRAMS[name].programId;
}

export async function getProgramAccounts(
  transport: RpcTransport,
  programId: string = DEFAULT_PROGRAM_ID,
): Promise<unknown> {
  return transport.request("getProgramAccounts", [programId, { encoding: "base64" }]);
}
"#,
    );
    out
}

fn render_react_ts(programs: &[ProgramHooks]) -> String {
    let mut out = String::from("// Generated by sunscreen. Do not edit.\n\n");
    out.push_str("import { useMutation, useQuery } from \"@tanstack/react-query\";\n");
    out.push_str("import { DEFAULT_PROGRAM_NAME, createSurfpoolRpc, getProgramAccounts, programIdFor, type RpcTransport, type SunscreenProgramName } from \"./core\";\n\n");
    out.push_str(&shared_hook_types(programs));
    out.push_str("export type ProgramAccountsQueryOptions = { transport?: RpcTransport; program?: SunscreenProgramName; programId?: string };\n\n");
    out.push_str(
        r#"export function useProgramAccountsQuery(options: ProgramAccountsQueryOptions = {}) {
  const transport = options.transport ?? createSurfpoolRpc();
  const program = options.program ?? DEFAULT_PROGRAM_NAME;
  const programId = options.programId ?? programIdFor(program);
  return useQuery({
    queryKey: ["sunscreen", program, "programAccounts", programId],
    queryFn: () => getProgramAccounts(transport, programId),
  });
}

"#,
    );
    for ix in all_instructions(programs) {
        out.push_str(&format!(
            "export function use{}Mutation<TResult = unknown>(executor: {}Executor<TResult>) {{\n  return useMutation<TResult, Error, {}Input>({{ mutationKey: [\"sunscreen\", {}], mutationFn: executor }});\n}}\n\n",
            ix.pascal,
            ix.pascal,
            ix.pascal,
            json_string(&ix.name)
        ));
    }
    out
}

fn render_solid_ts(programs: &[ProgramHooks]) -> String {
    let mut out = String::from("// Generated by sunscreen. Do not edit.\n\n");
    out.push_str("import { createMutation, createQuery } from \"@tanstack/solid-query\";\n");
    out.push_str("import { DEFAULT_PROGRAM_NAME, createSurfpoolRpc, getProgramAccounts, programIdFor, type RpcTransport, type SunscreenProgramName } from \"./core\";\n\n");
    out.push_str(&shared_hook_types(programs));
    out.push_str("export type ProgramAccountsQueryOptions = { transport?: RpcTransport; program?: SunscreenProgramName; programId?: string };\n\n");
    out.push_str(
        r#"export function createProgramAccountsQuery(options: ProgramAccountsQueryOptions = {}) {
  const transport = options.transport ?? createSurfpoolRpc();
  const program = options.program ?? DEFAULT_PROGRAM_NAME;
  const programId = options.programId ?? programIdFor(program);
  return createQuery(() => ({
    queryKey: ["sunscreen", program, "programAccounts", programId],
    queryFn: () => getProgramAccounts(transport, programId),
  }));
}

"#,
    );
    for ix in all_instructions(programs) {
        out.push_str(&format!(
            "export function create{}Mutation<TResult = unknown>(executor: {}Executor<TResult>) {{\n  return createMutation(() => ({{ mutationKey: [\"sunscreen\", {}], mutationFn: executor }}));\n}}\n\n",
            ix.pascal,
            ix.pascal,
            json_string(&ix.name)
        ));
    }
    out
}

fn render_index_ts(target: HookTarget) -> String {
    let mut out = String::from("// Generated by sunscreen. Do not edit.\n\n");
    out.push_str("export * from \"./core\";\nexport * from \"./idl\";\n");
    match target {
        HookTarget::React => out.push_str("export * from \"./react\";\n"),
        HookTarget::Solid => out.push_str("export * from \"./solid\";\n"),
        HookTarget::All => {
            out.push_str("export * as react from \"./react\";\n");
            out.push_str("export * as solid from \"./solid\";\n");
        }
    }
    out
}

fn shared_hook_types(programs: &[ProgramHooks]) -> String {
    let mut out = String::new();
    for ix in all_instructions(programs) {
        out.push_str(&format!(
            "export type {}Input = {};\n",
            ix.pascal, ix.input_type
        ));
        out.push_str(&format!(
            "export type {}Executor<TResult = unknown> = (input: {}Input) => Promise<TResult>;\n\n",
            ix.pascal, ix.pascal
        ));
    }
    out
}

fn all_instructions(programs: &[ProgramHooks]) -> Vec<&InstructionHooks> {
    let mut out = Vec::new();
    for program in programs {
        for ix in &program.instructions {
            out.push(ix);
        }
    }
    out
}

fn render_input_type(args: &[IdlArg]) -> String {
    if args.is_empty() {
        return "void".into();
    }
    let mut out = String::from("{ ");
    for arg in args {
        out.push_str(&format!(
            "{}: {}; ",
            arg.name.to_lower_camel_case(),
            ts_type(&arg.ty)
        ));
    }
    out.push('}');
    out
}

fn ts_type(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::String(s) => match s.as_str() {
            "bool" => "boolean",
            "string" => "string",
            "u8" | "u16" | "u32" | "i8" | "i16" | "i32" | "f32" | "f64" => "number",
            "u64" | "u128" | "i64" | "i128" => "number | bigint",
            "pubkey" | "Pubkey" | "publicKey" => "string",
            _ => "unknown",
        },
        serde_json::Value::Object(map) if map.contains_key("vec") => "unknown[]",
        serde_json::Value::Object(map) if map.contains_key("option") => "unknown | null",
        _ => "unknown",
    }
}

fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("string serialization")
}

fn indent_json(value: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    value
        .lines()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                line.to_string()
            } else {
                format!("{pad}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Render changed paths relative to the workspace root.
#[must_use]
pub fn changed_files(workspace_root: &Path, files: &[FileWrite]) -> Vec<String> {
    files
        .iter()
        .filter(|file| file.changed)
        .map(|file| relative_path(workspace_root, &file.path))
        .collect()
}
