use std::io::{self, BufRead};
use std::path::{Path, PathBuf};

use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct FileAstResult {
    path: String,
    ok: bool,
    index: Option<vac_index::rust_ast::RustAstIndex>,
    error: Option<vac_index::rust_ast::RustAstError>,
}

fn root_from_args() -> Result<PathBuf, String> {
    let mut args = std::env::args().skip(1);
    let mut root = std::env::current_dir().map_err(|err| err.to_string())?;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--root requires a workspace path".to_string())?;
                root = PathBuf::from(value);
            }
            "--help" | "-h" => {
                println!(
                    "Usage: vac-index-rust-ast [--root <workspace>] < newline-separated relative Rust paths"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    Ok(root)
}

fn parse_file(root: &Path, rel: &str) -> FileAstResult {
    let path = root.join(rel);
    let source = match std::fs::read_to_string(&path) {
        Ok(source) => source,
        Err(err) => {
            return FileAstResult {
                path: rel.to_string(),
                ok: false,
                index: None,
                error: Some(vac_index::rust_ast::RustAstError {
                    path: rel.to_string(),
                    parser_mode: vac_index::rust_ast::rust_ast_parser_mode().to_string(),
                    message: format!("failed to read Rust source: {err}"),
                }),
            };
        }
    };

    match vac_index::rust_ast::extract_rust_ast_index(rel, &source) {
        Ok(index) => FileAstResult {
            path: rel.to_string(),
            ok: true,
            index: Some(index),
            error: None,
        },
        Err(error) => FileAstResult {
            path: rel.to_string(),
            ok: false,
            index: None,
            error: Some(error),
        },
    }
}

fn main() -> Result<(), String> {
    let root = root_from_args()?;
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let rel = line.map_err(|err| err.to_string())?;
        let rel = rel.trim();
        if rel.is_empty() {
            continue;
        }
        let result = parse_file(&root, rel);
        let encoded = serde_json::to_string(&result).map_err(|err| err.to_string())?;
        println!("{encoded}");
    }
    Ok(())
}
