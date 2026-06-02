//! Embedded starter examples.

use std::path::{Component, Path, PathBuf};

use rust_embed::RustEmbed;

use crate::cli::onboarding::{ExampleUseArgs, ExamplesCmd, ExamplesListArgs};
use crate::error::SunscreenError;
use crate::fsutil::{Transaction, TxError};

#[derive(RustEmbed)]
#[folder = "assets/examples/"]
struct ExampleAssets;

#[derive(Debug, Clone)]
pub struct Example {
    pub name: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub tags: &'static [&'static str],
    pub est_minutes: u8,
}

const EXAMPLES: &[Example] = &[
    Example {
        name: "token-faucet",
        title: "Token Faucet",
        description: "SPL token minting starter with a faucet-style UX.",
        tags: &["token", "spl", "beginner"],
        est_minutes: 8,
    },
    Example {
        name: "nft-collection",
        title: "NFT Collection",
        description: "Metaplex NFT collection starter for devnet demos.",
        tags: &["nft", "metaplex", "devnet"],
        est_minutes: 10,
    },
    Example {
        name: "escrow",
        title: "Escrow",
        description: "Two-party escrow skeleton focused on account flow.",
        tags: &["pda", "cpi", "intermediate"],
        est_minutes: 12,
    },
    Example {
        name: "voting-dao",
        title: "Voting DAO",
        description: "Proposal and voting starter for governance prototypes.",
        tags: &["dao", "governance", "crud"],
        est_minutes: 10,
    },
    Example {
        name: "blog-crud",
        title: "Blog CRUD",
        description: "CRUD dApp starter built from the Phase 5 blog recipe.",
        tags: &["blog", "crud", "frontend"],
        est_minutes: 7,
    },
];

pub fn run(cmd: &ExamplesCmd, json: bool) -> Result<i32, SunscreenError> {
    match cmd {
        ExamplesCmd::List(args) => run_list(args, json),
        ExamplesCmd::Describe(args) => run_describe(&args.name, json),
        ExamplesCmd::Use(args) => run_use(args, json),
    }
}

fn run_list(args: &ExamplesListArgs, json: bool) -> Result<i32, SunscreenError> {
    let examples = examples(args.tag.as_deref());
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "examples": examples.iter().map(|example| example_json(example)).collect::<Vec<_>>(),
            })
        );
    } else {
        for example in examples {
            println!(
                "{}\t{}\t{} min\t{}",
                example.name,
                example.title,
                example.est_minutes,
                example.tags.join(",")
            );
        }
    }
    Ok(0)
}

fn run_describe(name: &str, json: bool) -> Result<i32, SunscreenError> {
    let example = find_example(name)?;
    let readme = read_example_file(example.name, "README.md")?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "example": example_json(example),
                "readme": readme,
            })
        );
    } else {
        print!("{readme}");
    }
    Ok(0)
}

fn run_use(args: &ExampleUseArgs, json: bool) -> Result<i32, SunscreenError> {
    let example = find_example(&args.name)?;
    let dest = args
        .path
        .clone()
        .unwrap_or_else(|| PathBuf::from(example.name));
    preflight_dest(&dest, args.dry_run)?;

    let files = files_for_example(example.name)?;
    if args.dry_run {
        emit_use_result(json, example, &dest, &files, true, 0);
        return Ok(0);
    }

    let mut tx = Transaction::new(&dest).map_err(map_tx_err)?;
    for (rel, bytes) in &files {
        tx.stage(rel, bytes).map_err(map_tx_err)?;
    }
    let written = tx.commit().map_err(map_tx_err)?.len();
    emit_use_result(json, example, &dest, &files, false, written);
    Ok(0)
}

fn examples(tag: Option<&str>) -> Vec<&'static Example> {
    EXAMPLES
        .iter()
        .filter(|example| tag.map(|tag| example.tags.contains(&tag)).unwrap_or(true))
        .collect()
}

fn find_example(name: &str) -> Result<&'static Example, SunscreenError> {
    EXAMPLES
        .iter()
        .find(|example| example.name == name)
        .ok_or_else(|| {
            SunscreenError::UserInput(format!(
                "unknown example `{name}`; run `sunscreen examples list`"
            ))
        })
}

fn read_example_file(example: &str, rel: &str) -> Result<String, SunscreenError> {
    let path = format!("{example}/{rel}");
    let asset = ExampleAssets::get(&path)
        .ok_or_else(|| SunscreenError::Other(anyhow::anyhow!("embedded example missing {path}")))?;
    String::from_utf8(asset.data.into_owned())
        .map_err(|err| SunscreenError::Other(anyhow::anyhow!("decode {path}: {err}")))
}

fn files_for_example(example: &str) -> Result<Vec<(String, Vec<u8>)>, SunscreenError> {
    let prefix = format!("{example}/");
    let mut files = Vec::new();
    for asset in ExampleAssets::iter() {
        let Some(rel) = asset.strip_prefix(&prefix) else {
            continue;
        };
        ensure_safe_rel(rel)?;
        let file = ExampleAssets::get(asset.as_ref()).ok_or_else(|| {
            SunscreenError::Other(anyhow::anyhow!("embedded example missing {asset}"))
        })?;
        files.push((rel.to_string(), file.data.into_owned()));
    }
    files.sort_by(|a, b| a.0.cmp(&b.0));
    if files.is_empty() {
        return Err(SunscreenError::Other(anyhow::anyhow!(
            "embedded example `{example}` has no files"
        )));
    }
    Ok(files)
}

fn ensure_safe_rel(path: &str) -> Result<(), SunscreenError> {
    let path = Path::new(path);
    let unsafe_path = path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)));
    if unsafe_path {
        return Err(SunscreenError::Other(anyhow::anyhow!(
            "embedded example path escapes output: {}",
            path.display()
        )));
    }
    Ok(())
}

fn preflight_dest(path: &Path, dry_run: bool) -> Result<(), SunscreenError> {
    if dry_run || !path.exists() {
        return Ok(());
    }
    let mut entries = std::fs::read_dir(path).map_err(|err| {
        SunscreenError::Other(anyhow::anyhow!(
            "read output directory {}: {err}",
            path.display()
        ))
    })?;
    if entries.next().is_some() {
        return Err(SunscreenError::PathConflict(format!(
            "destination already exists and is not empty: {}",
            path.display()
        )));
    }
    Ok(())
}

fn emit_use_result(
    json: bool,
    example: &Example,
    dest: &Path,
    files: &[(String, Vec<u8>)],
    dry_run: bool,
    written: usize,
) {
    let paths = files.iter().map(|(path, _)| path).collect::<Vec<_>>();
    if json {
        println!(
            "{}",
            serde_json::json!({
                "ok": true,
                "command": "examples_use",
                "example": example.name,
                "path": dest.display().to_string(),
                "dry_run": dry_run,
                "files": paths,
                "written": written,
                "next_step": format!("cd {} && sunscreen chain serve --headless", dest.display()),
            })
        );
    } else if dry_run {
        println!(
            "dry-run: would copy example `{}` to {}",
            example.name,
            dest.display()
        );
        for path in paths {
            println!("  {path}");
        }
    } else {
        println!(
            "copied example `{}` to {} ({} files)",
            example.name,
            dest.display(),
            written
        );
    }
}

fn example_json(example: &Example) -> serde_json::Value {
    serde_json::json!({
        "name": example.name,
        "title": example.title,
        "description": example.description,
        "tags": example.tags,
        "est_minutes": example.est_minutes,
    })
}

fn map_tx_err(err: TxError) -> SunscreenError {
    match err {
        TxError::PathEscape(path) => SunscreenError::UserInput(format!("invalid path: {path}")),
        TxError::DestinationExists(path) => {
            SunscreenError::PathConflict(format!("destination already exists: {}", path.display()))
        }
        TxError::DuplicateStage(path) => {
            SunscreenError::Other(anyhow::anyhow!("embedded example duplicated path: {path}"))
        }
        TxError::Io(err) => SunscreenError::Other(anyhow::anyhow!(err)),
    }
}
