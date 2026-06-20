use globset::Glob;
use grep_regex::RegexMatcher;
use grep_searcher::Searcher;
use grep_searcher::sinks::UTF8;
use ignore::WalkBuilder;
use rmcp::ErrorData as McpError;
use rmcp::model::{CallToolResult, Content};
use serde_json::json;
use similar::TextDiff;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tracing::error;
use vac_foundation::file_backup_manager::FileBackupManager;
use vac_foundation::remote_connection::{RemoteConnection, RemoteFileSystemProvider};
use vac_foundation::utils::{
    LocalFileSystemProvider, generate_directory_tree, sanitize_text_output,
};

/// Options for viewing files/directories (used internally to reduce function arguments)
#[derive(Debug, Clone)]
pub(crate) struct ViewOptions<'a> {
    pub(crate) view_range: Option<[i32; 2]>,
    pub(crate) max_lines: usize,
    pub(crate) tree: Option<bool>,
    pub(crate) grep: Option<&'a str>,
    pub(crate) glob: Option<&'a str>,
}

/// View the contents of a local file or directory
pub(crate) async fn view_local_path(
    path: &str,
    opts: &ViewOptions<'_>,
) -> Result<CallToolResult, McpError> {
    let path_obj = Path::new(path);

    if !path_obj.exists() {
        return Ok(CallToolResult::error(vec![
            Content::text("FILE_NOT_FOUND"),
            Content::text(format!("File or directory not found: {}", path)),
        ]));
    }

    if path_obj.is_dir() {
        // Handle combined glob + grep: filter files by glob, then search content
        if let (Some(glob_pattern), Some(grep_pattern)) = (opts.glob, opts.grep) {
            return grep_local_directory_with_glob(
                path,
                grep_pattern,
                glob_pattern,
                opts.max_lines,
            )
            .await;
        }

        // Handle glob pattern filtering for directories (list files only)
        if let Some(glob_pattern) = opts.glob {
            return view_local_dir_with_glob(path, glob_pattern, opts.max_lines).await;
        }

        // Handle grep search in directory (all files)
        if let Some(grep_pattern) = opts.grep {
            return grep_local_directory(path, grep_pattern, opts.max_lines).await;
        }

        // Default directory tree view
        let depth = if opts.tree.unwrap_or(false) { 3 } else { 1 };
        let provider = LocalFileSystemProvider;
        let path_str = path_obj.to_string_lossy();

        match generate_directory_tree(&provider, &path_str, "", depth, 0).await {
            Ok(tree_content) => {
                let result = format!(
                    "Directory tree for \"{}\":\n{}\n{}",
                    path,
                    path_obj
                        .file_name()
                        .map(|name| name.to_string_lossy().to_string())
                        .unwrap_or_else(|| path.to_string()),
                    tree_content
                );
                Ok(CallToolResult::success(vec![Content::text(
                    sanitize_text_output(&result),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![
                Content::text("READ_ERROR"),
                Content::text(format!("Cannot read directory: {}", e)),
            ])),
        }
    } else {
        // Handle grep search in single file
        if let Some(grep_pattern) = opts.grep {
            return grep_local_file(path, grep_pattern, opts.max_lines);
        }

        // Read file contents
        match fs::read_to_string(path) {
            Ok(content) => {
                let result = match format_file_content(
                    &content,
                    path,
                    opts.view_range,
                    opts.max_lines,
                    "File",
                ) {
                    Ok(result) => result,
                    Err(e) => {
                        return Ok(CallToolResult::error(vec![
                            Content::text("FORMAT_ERROR"),
                            Content::text(format!("Failed to format file content: {}", e)),
                        ]));
                    }
                };

                Ok(CallToolResult::success(vec![Content::text(
                    sanitize_text_output(&result),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![
                Content::text("READ_ERROR"),
                Content::text(format!("Cannot read file: {}", e)),
            ])),
        }
    }
}

/// View directory contents filtered by glob pattern
async fn view_local_dir_with_glob(
    path: &str,
    glob_pattern: &str,
    max_lines: usize,
) -> Result<CallToolResult, McpError> {
    // Build the glob matcher
    let glob = match Glob::new(glob_pattern) {
        Ok(g) => g.compile_matcher(),
        Err(e) => {
            return Ok(CallToolResult::error(vec![
                Content::text("INVALID_GLOB"),
                Content::text(format!("Invalid glob pattern '{}': {}", glob_pattern, e)),
            ]));
        }
    };

    // Use ignore crate's WalkBuilder for gitignore-aware traversal
    let walker = WalkBuilder::new(path)
        .hidden(false) // Show hidden files
        .git_ignore(true) // Respect .gitignore
        .build();

    let mut matches: Vec<String> = Vec::new();
    let base_path = Path::new(path);

    for entry in walker.flatten() {
        let entry_path = entry.path();

        // Get relative path for glob matching
        let relative = match entry_path.strip_prefix(base_path) {
            Ok(r) => r.to_string_lossy().to_string(),
            Err(_) => continue,
        };

        // Skip the root directory itself
        if relative.is_empty() {
            continue;
        }

        // Check if the path matches the glob pattern
        if glob.is_match(&relative) || glob.is_match(entry_path.file_name().unwrap_or_default()) {
            let prefix = if entry_path.is_dir() {
                "📁 "
            } else {
                "📄 "
            };
            matches.push(format!("{}{}", prefix, relative));
        }
    }

    if matches.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "No files matching '{}' found in {}",
            glob_pattern, path
        ))]));
    }

    // Sort and truncate
    matches.sort();
    let total = matches.len();
    let truncated = matches.len() > max_lines;
    if truncated {
        matches.truncate(max_lines);
    }

    let mut result = format!(
        "Files matching '{}' in \"{}\" ({} matches):\n\n{}",
        glob_pattern,
        path,
        total,
        matches.join("\n")
    );

    if truncated {
        result.push_str(&format!("\n\n... and {} more files", total - max_lines));
    }

    Ok(CallToolResult::success(vec![Content::text(
        sanitize_text_output(&result),
    )]))
}

/// Grep search in a single local file
fn grep_local_file(
    path: &str,
    pattern: &str,
    max_lines: usize,
) -> Result<CallToolResult, McpError> {
    let matcher = match RegexMatcher::new(pattern) {
        Ok(m) => m,
        Err(e) => {
            return Ok(CallToolResult::error(vec![
                Content::text("INVALID_REGEX"),
                Content::text(format!("Invalid regex pattern '{}': {}", pattern, e)),
            ]));
        }
    };

    let mut matches: Vec<String> = Vec::new();
    let mut searcher = Searcher::new();

    let sink_result = searcher.search_path(
        &matcher,
        path,
        UTF8(|line_num, line| {
            if matches.len() < max_lines {
                matches.push(format!("{}:{}", line_num, line.trim_end()));
            }
            Ok(true)
        }),
    );

    if let Err(e) = sink_result {
        return Ok(CallToolResult::error(vec![
            Content::text("GREP_ERROR"),
            Content::text(format!("Error searching file: {}", e)),
        ]));
    }

    if matches.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "No matches for '{}' in {}",
            pattern, path
        ))]));
    }

    let total = matches.len();
    let result = format!(
        "Grep results for '{}' in \"{}\" ({} matches in 1 file):\n\n{}",
        pattern,
        path,
        total,
        matches.join("\n")
    );

    Ok(CallToolResult::success(vec![Content::text(
        sanitize_text_output(&result),
    )]))
}

/// Grep search across a directory (recursive, respects .gitignore)
async fn grep_local_directory(
    path: &str,
    pattern: &str,
    max_lines: usize,
) -> Result<CallToolResult, McpError> {
    let matcher = match RegexMatcher::new(pattern) {
        Ok(m) => m,
        Err(e) => {
            return Ok(CallToolResult::error(vec![
                Content::text("INVALID_REGEX"),
                Content::text(format!("Invalid regex pattern '{}': {}", pattern, e)),
            ]));
        }
    };

    // Use ignore crate for gitignore-aware traversal
    let walker = WalkBuilder::new(path)
        .hidden(false)
        .git_ignore(true)
        .build();

    let mut all_matches: Vec<String> = Vec::new();
    let mut files_with_matches = 0;
    let base_path = Path::new(path);

    for entry in walker.flatten() {
        if all_matches.len() >= max_lines {
            break;
        }

        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }

        let relative = entry_path
            .strip_prefix(base_path)
            .map(|r| r.to_string_lossy().to_string())
            .unwrap_or_else(|_| entry_path.to_string_lossy().to_string());

        let mut file_matches: Vec<String> = Vec::new();
        let mut searcher = Searcher::new();

        let _ = searcher.search_path(
            &matcher,
            entry_path,
            UTF8(|line_num, line| {
                if all_matches.len() + file_matches.len() < max_lines {
                    file_matches.push(format!("{}:{}:{}", relative, line_num, line.trim_end()));
                }
                Ok(true)
            }),
        );

        if !file_matches.is_empty() {
            files_with_matches += 1;
            all_matches.extend(file_matches);
        }
    }

    if all_matches.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "No matches for '{}' in {}",
            pattern, path
        ))]));
    }

    let truncated = all_matches.len() >= max_lines;
    let result = format!(
        "Grep results for '{}' in \"{}\" ({} matches in {} files):\n\n{}{}",
        pattern,
        path,
        all_matches.len(),
        files_with_matches,
        all_matches.join("\n"),
        if truncated { "\n\n... (truncated)" } else { "" }
    );

    Ok(CallToolResult::success(vec![Content::text(
        sanitize_text_output(&result),
    )]))
}

/// Grep search across a directory filtered by glob pattern
async fn grep_local_directory_with_glob(
    path: &str,
    pattern: &str,
    glob_pattern: &str,
    max_lines: usize,
) -> Result<CallToolResult, McpError> {
    let matcher = match RegexMatcher::new(pattern) {
        Ok(m) => m,
        Err(e) => {
            return Ok(CallToolResult::error(vec![
                Content::text("INVALID_REGEX"),
                Content::text(format!("Invalid regex pattern '{}': {}", pattern, e)),
            ]));
        }
    };

    let glob = match Glob::new(glob_pattern) {
        Ok(g) => g.compile_matcher(),
        Err(e) => {
            return Ok(CallToolResult::error(vec![
                Content::text("INVALID_GLOB"),
                Content::text(format!("Invalid glob pattern '{}': {}", glob_pattern, e)),
            ]));
        }
    };

    // Use ignore crate for gitignore-aware traversal
    let walker = WalkBuilder::new(path)
        .hidden(false)
        .git_ignore(true)
        .build();

    let mut all_matches: Vec<String> = Vec::new();
    let mut files_with_matches = 0;
    let base_path = Path::new(path);

    for entry in walker.flatten() {
        if all_matches.len() >= max_lines {
            break;
        }

        let entry_path = entry.path();
        if !entry_path.is_file() {
            continue;
        }

        // Check if file matches glob pattern
        let relative = entry_path
            .strip_prefix(base_path)
            .map(|r| r.to_string_lossy().to_string())
            .unwrap_or_else(|_| entry_path.to_string_lossy().to_string());

        let matches_glob =
            glob.is_match(&relative) || glob.is_match(entry_path.file_name().unwrap_or_default());

        if !matches_glob {
            continue;
        }

        let mut file_matches: Vec<String> = Vec::new();
        let mut searcher = Searcher::new();

        let _ = searcher.search_path(
            &matcher,
            entry_path,
            UTF8(|line_num, line| {
                if all_matches.len() + file_matches.len() < max_lines {
                    file_matches.push(format!("{}:{}:{}", relative, line_num, line.trim_end()));
                }
                Ok(true)
            }),
        );

        if !file_matches.is_empty() {
            files_with_matches += 1;
            all_matches.extend(file_matches);
        }
    }

    if all_matches.is_empty() {
        return Ok(CallToolResult::success(vec![Content::text(format!(
            "No matches for '{}' in {} (filtered by glob '{}')",
            pattern, path, glob_pattern
        ))]));
    }

    let truncated = all_matches.len() >= max_lines;
    let result = format!(
        "Grep results for '{}' in \"{}\" (glob: '{}') ({} matches in {} files):\n\n{}{}",
        pattern,
        path,
        glob_pattern,
        all_matches.len(),
        files_with_matches,
        all_matches.join("\n"),
        if truncated { "\n\n... (truncated)" } else { "" }
    );

    Ok(CallToolResult::success(vec![Content::text(
        sanitize_text_output(&result),
    )]))
}

/// View the contents of a remote file or directory
pub(crate) async fn view_remote_path(
    conn: &Arc<RemoteConnection>,
    remote_path: &str,
    original_path: &str,
    opts: &ViewOptions<'_>,
) -> Result<CallToolResult, McpError> {
    if !conn.exists(remote_path).await {
        return Ok(CallToolResult::error(vec![
            Content::text("FILE_NOT_FOUND"),
            Content::text(format!(
                "Remote file or directory not found: {}",
                original_path
            )),
        ]));
    }

    if conn.is_directory(remote_path).await {
        // Handle combined glob + grep for remote directories
        if let (Some(glob_pattern), Some(grep_pattern)) = (opts.glob, opts.grep) {
            return grep_remote_directory_with_glob(
                conn,
                remote_path,
                original_path,
                grep_pattern,
                glob_pattern,
                opts.max_lines,
            )
            .await;
        }

        // Handle glob pattern filtering for remote directories
        if let Some(glob_pattern) = opts.glob {
            return view_remote_dir_with_glob(
                conn,
                remote_path,
                original_path,
                glob_pattern,
                opts.max_lines,
            )
            .await;
        }

        // Handle grep search in remote directory
        if let Some(grep_pattern) = opts.grep {
            return grep_remote_directory(
                conn,
                remote_path,
                original_path,
                grep_pattern,
                opts.max_lines,
            )
            .await;
        }

        // Default directory tree view
        let depth = if opts.tree.unwrap_or(false) { 3 } else { 1 };
        let provider = RemoteFileSystemProvider::new(conn.clone());

        match generate_directory_tree(&provider, remote_path, "", depth, 0).await {
            Ok(tree_content) => {
                let result = format!(
                    "Remote directory tree for \"{}\":\n{}\n{}",
                    original_path,
                    remote_path.split('/').next_back().unwrap_or(remote_path),
                    tree_content
                );
                Ok(CallToolResult::success(vec![Content::text(
                    sanitize_text_output(&result),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![
                Content::text("READ_ERROR"),
                Content::text(format!("Cannot read remote directory: {}", e)),
            ])),
        }
    } else {
        // Handle grep search in single remote file
        if let Some(grep_pattern) = opts.grep {
            return grep_remote_file(
                conn,
                remote_path,
                original_path,
                grep_pattern,
                opts.max_lines,
            )
            .await;
        }

        // Read remote file contents
        match conn.read_file_to_string(remote_path).await {
            Ok(content) => {
                let result = match format_file_content(
                    &content,
                    original_path,
                    opts.view_range,
                    opts.max_lines,
                    "Remote file",
                ) {
                    Ok(result) => result,
                    Err(e) => {
                        return Ok(CallToolResult::error(vec![
                            Content::text("FORMAT_ERROR"),
                            Content::text(format!("Failed to format remote file content: {}", e)),
                        ]));
                    }
                };

                Ok(CallToolResult::success(vec![Content::text(
                    sanitize_text_output(&result),
                )]))
            }
            Err(e) => Ok(CallToolResult::error(vec![
                Content::text("READ_ERROR"),
                Content::text(format!("Cannot read remote file: {}", e)),
            ])),
        }
    }
}

fn shell_single_quote_arg(value: &str) -> String {
    if value.is_empty() {
        return "''".to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

fn remote_head_limit(max_lines: usize) -> usize {
    max_lines.saturating_add(1)
}

fn build_remote_find_glob_command(
    remote_path: &str,
    glob_pattern: &str,
    max_lines: usize,
) -> String {
    format!(
        "find {} -name {} 2>/dev/null | head -n {}",
        shell_single_quote_arg(remote_path),
        shell_single_quote_arg(glob_pattern),
        remote_head_limit(max_lines)
    )
}

fn build_remote_grep_file_command(remote_path: &str, pattern: &str, max_lines: usize) -> String {
    format!(
        "grep -En -- {} {} 2>/dev/null | head -n {}",
        shell_single_quote_arg(pattern),
        shell_single_quote_arg(remote_path),
        remote_head_limit(max_lines)
    )
}

fn build_remote_grep_directory_command(
    remote_path: &str,
    pattern: &str,
    max_lines: usize,
) -> String {
    format!(
        "grep -rEn --include='*' -- {} {} 2>/dev/null | head -n {}",
        shell_single_quote_arg(pattern),
        shell_single_quote_arg(remote_path),
        remote_head_limit(max_lines)
    )
}

fn build_remote_grep_directory_with_glob_command(
    remote_path: &str,
    pattern: &str,
    glob_pattern: &str,
    max_lines: usize,
) -> String {
    format!(
        "find {} -name {} -type f -exec grep -EHn -- {} {{}} \\; 2>/dev/null | head -n {}",
        shell_single_quote_arg(remote_path),
        shell_single_quote_arg(glob_pattern),
        shell_single_quote_arg(pattern),
        remote_head_limit(max_lines)
    )
}

/// View remote directory contents filtered by glob pattern using find command
async fn view_remote_dir_with_glob(
    conn: &Arc<RemoteConnection>,
    remote_path: &str,
    original_path: &str,
    glob_pattern: &str,
    max_lines: usize,
) -> Result<CallToolResult, McpError> {
    let command = build_remote_find_glob_command(remote_path, glob_pattern, max_lines);

    match conn.execute_command(&command, None, None).await {
        Ok((output, exit_code)) => {
            if exit_code != 0 && output.trim().is_empty() {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "No files matching '{}' found in {}",
                    glob_pattern, original_path
                ))]));
            }

            let lines: Vec<&str> = output.lines().collect();
            let truncated = lines.len() > max_lines;
            let display_lines: Vec<&str> = lines.into_iter().take(max_lines).collect();

            if display_lines.is_empty() {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "No files matching '{}' found in {}",
                    glob_pattern, original_path
                ))]));
            }

            let mut result = format!(
                "Remote files matching '{}' in \"{}\":\n\n{}",
                glob_pattern,
                original_path,
                display_lines.join("\n")
            );

            if truncated {
                result.push_str("\n\n... (truncated)");
            }

            Ok(CallToolResult::success(vec![Content::text(
                sanitize_text_output(&result),
            )]))
        }
        Err(e) => Ok(CallToolResult::error(vec![
            Content::text("REMOTE_GLOB_ERROR"),
            Content::text(format!("Failed to search remote directory: {}", e)),
        ])),
    }
}

/// Grep search in a single remote file
async fn grep_remote_file(
    conn: &Arc<RemoteConnection>,
    remote_path: &str,
    original_path: &str,
    pattern: &str,
    max_lines: usize,
) -> Result<CallToolResult, McpError> {
    let command = build_remote_grep_file_command(remote_path, pattern, max_lines);

    match conn.execute_command(&command, None, None).await {
        Ok((output, _exit_code)) => {
            // grep returns exit code 1 for no matches, which is fine
            let lines: Vec<&str> = output.lines().collect();

            if lines.is_empty() {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "No matches for '{}' in {}",
                    pattern, original_path
                ))]));
            }

            let truncated = lines.len() > max_lines;
            let display_lines: Vec<&str> = lines.into_iter().take(max_lines).collect();

            let mut result = format!(
                "Grep results for '{}' in \"{}\" ({} matches in 1 file):\n\n{}",
                pattern,
                original_path,
                display_lines.len(),
                display_lines.join("\n")
            );

            if truncated {
                result.push_str("\n\n... (truncated)");
            }

            Ok(CallToolResult::success(vec![Content::text(
                sanitize_text_output(&result),
            )]))
        }
        Err(e) => Ok(CallToolResult::error(vec![
            Content::text("REMOTE_GREP_ERROR"),
            Content::text(format!("Failed to grep remote file: {}", e)),
        ])),
    }
}

/// Grep search across a remote directory using grep -rE
async fn grep_remote_directory(
    conn: &Arc<RemoteConnection>,
    remote_path: &str,
    original_path: &str,
    pattern: &str,
    max_lines: usize,
) -> Result<CallToolResult, McpError> {
    let command = build_remote_grep_directory_command(remote_path, pattern, max_lines);

    match conn.execute_command(&command, None, None).await {
        Ok((output, _exit_code)) => {
            let lines: Vec<&str> = output.lines().collect();

            if lines.is_empty() {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "No matches for '{}' in {}",
                    pattern, original_path
                ))]));
            }

            let truncated = lines.len() > max_lines;
            let display_lines: Vec<&str> = lines.into_iter().take(max_lines).collect();

            // Count unique files
            let files_with_matches: std::collections::HashSet<&str> = display_lines
                .iter()
                .filter_map(|line| line.split(':').next())
                .collect();

            let mut result = format!(
                "Grep results for '{}' in \"{}\" ({} matches in {} files):\n\n{}",
                pattern,
                original_path,
                display_lines.len(),
                files_with_matches.len(),
                display_lines.join("\n")
            );

            if truncated {
                result.push_str("\n\n... (truncated)");
            }

            Ok(CallToolResult::success(vec![Content::text(
                sanitize_text_output(&result),
            )]))
        }
        Err(e) => Ok(CallToolResult::error(vec![
            Content::text("REMOTE_GREP_ERROR"),
            Content::text(format!("Failed to grep remote directory: {}", e)),
        ])),
    }
}

/// Grep search across a remote directory filtered by glob pattern
async fn grep_remote_directory_with_glob(
    conn: &Arc<RemoteConnection>,
    remote_path: &str,
    original_path: &str,
    pattern: &str,
    glob_pattern: &str,
    max_lines: usize,
) -> Result<CallToolResult, McpError> {
    let command = build_remote_grep_directory_with_glob_command(
        remote_path,
        pattern,
        glob_pattern,
        max_lines,
    );

    match conn.execute_command(&command, None, None).await {
        Ok((output, _exit_code)) => {
            let lines: Vec<&str> = output.lines().collect();

            if lines.is_empty() {
                return Ok(CallToolResult::success(vec![Content::text(format!(
                    "No matches for '{}' in {} (filtered by glob '{}')",
                    pattern, original_path, glob_pattern
                ))]));
            }

            let truncated = lines.len() > max_lines;
            let display_lines: Vec<&str> = lines.into_iter().take(max_lines).collect();

            // Count unique files
            let files_with_matches: std::collections::HashSet<&str> = display_lines
                .iter()
                .filter_map(|line| line.split(':').next())
                .collect();

            let mut result = format!(
                "Grep results for '{}' in \"{}\" (glob: '{}') ({} matches in {} files):\n\n{}",
                pattern,
                original_path,
                glob_pattern,
                display_lines.len(),
                files_with_matches.len(),
                display_lines.join("\n")
            );

            if truncated {
                result.push_str("\n\n... (truncated)");
            }

            Ok(CallToolResult::success(vec![Content::text(
                sanitize_text_output(&result),
            )]))
        }
        Err(e) => Ok(CallToolResult::error(vec![
            Content::text("REMOTE_GREP_ERROR"),
            Content::text(format!("Failed to grep remote directory: {}", e)),
        ])),
    }
}

/// Format file content with line numbers and truncation - shared logic
fn format_file_content(
    content: &str,
    path: &str,
    view_range: Option<[i32; 2]>,
    max_lines: usize,
    prefix: &str,
) -> Result<String, McpError> {
    let result = if let Some([start, end]) = view_range {
        let lines: Vec<&str> = content.lines().collect();
        let start_idx = if start <= 0 { 0 } else { (start - 1) as usize };
        let end_idx = if end == -1 {
            lines.len()
        } else {
            std::cmp::min(end as usize, lines.len())
        };

        if start_idx >= lines.len() {
            return Err(McpError::internal_error(
                "Invalid range",
                Some(json!({
                    "error": format!("Start line {} is beyond file length {}", start, lines.len())
                })),
            ));
        }

        let selected_lines = &lines[start_idx..end_idx];
        if selected_lines.len() <= max_lines {
            format!(
                "{}: {} (lines {}-{})\n{}",
                prefix,
                path,
                start_idx + 1,
                end_idx,
                selected_lines
                    .iter()
                    .enumerate()
                    .map(|(i, line)| format!("{:3}: {}", start_idx + i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        } else {
            let selected_lines = selected_lines.iter().take(max_lines).collect::<Vec<_>>();
            format!(
                "{}: {} (showing lines {}-{}, only the first {} lines of your view range)\n{}\n...",
                prefix,
                path,
                start_idx + 1,
                start_idx + 1 + max_lines,
                max_lines,
                selected_lines
                    .iter()
                    .enumerate()
                    .map(|(i, line)| format!("{:4}: {}", start_idx + i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    } else {
        let lines: Vec<&str> = content.lines().collect();
        if lines.len() <= max_lines {
            format!(
                "{}: {} ({} lines)\n{}",
                prefix,
                path,
                lines.len(),
                lines
                    .iter()
                    .enumerate()
                    .map(|(i, line)| format!("{:3}: {}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        } else {
            let selected_lines = lines.iter().take(max_lines).collect::<Vec<_>>();
            format!(
                "{}: {} (showing {} / {} lines)\n{}\n...",
                prefix,
                path,
                max_lines,
                lines.len(),
                selected_lines
                    .iter()
                    .enumerate()
                    .map(|(i, line)| format!("{:3}: {}", i + 1, line))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        }
    };

    Ok(sanitize_text_output(&result))
}

fn create_unified_diff(original: &str, modified: &str, from_file: &str, to_file: &str) -> String {
    let text_diff = TextDiff::from_lines(original, modified);
    format!(
        "{}",
        text_diff
            .unified_diff()
            .context_radius(0)
            .header(from_file, to_file)
    )
}

/// Replace a specific string in a remote file
pub(crate) async fn str_replace_remote(
    conn: &Arc<RemoteConnection>,
    remote_path: &str,
    original_path: &str,
    old_str: &str,
    new_str: &str,
    replace_all: Option<bool>,
) -> Result<CallToolResult, McpError> {
    if old_str == new_str {
        return Ok(CallToolResult::error(vec![
            Content::text("OLD_STR_NEW_STR_IDENTICAL"),
            Content::text("The old string and new string are identical - no replacement needed"),
        ]));
    }

    let content = match conn.read_file_to_string(remote_path).await {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read remote file for str_replace: {}", e);
            return Ok(CallToolResult::error(vec![
                Content::text("REMOTE_FILE_READ_ERROR"),
                Content::text(format!("Failed to read remote file: {}", e)),
            ]));
        }
    };

    if !content.contains(old_str) {
        return Ok(CallToolResult::error(vec![
            Content::text("STRING_NOT_FOUND"),
            Content::text("The string old_str was not found in the file"),
        ]));
    }

    let new_content = if replace_all.unwrap_or(false) {
        content.replace(old_str, new_str)
    } else {
        content.replacen(old_str, new_str, 1)
    };

    let replaced_count = if replace_all.unwrap_or(false) {
        content.matches(old_str).count()
    } else if content.contains(old_str) {
        1
    } else {
        0
    };

    if let Err(e) = conn.write_file(remote_path, new_content.as_bytes()).await {
        error!("Failed to write remote file for str_replace: {}", e);
        return Ok(CallToolResult::error(vec![
            Content::text("REMOTE_FILE_WRITE_ERROR"),
            Content::text(format!("Failed to write remote file: {}", e)),
        ]));
    }

    let unified_diff = create_unified_diff(&content, &new_content, original_path, original_path);

    let output = format!(
        "Successfully replaced {} occurrences of text (remote)\n\n```diff\n{}\n```",
        replaced_count, unified_diff
    );

    Ok(CallToolResult::success(vec![Content::text(&output)]))
}

/// Replace a specific string in a local file
pub(crate) async fn str_replace_local(
    path: &str,
    old_str: &str,
    new_str: &str,
    replace_all: Option<bool>,
) -> Result<CallToolResult, McpError> {
    if old_str == new_str {
        return Ok(CallToolResult::error(vec![
            Content::text("OLD_STR_NEW_STR_IDENTICAL"),
            Content::text("The old string and new string are identical - no replacement needed"),
        ]));
    }

    let original_content = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            error!("Failed to read local file for str_replace: {}", e);
            return Ok(CallToolResult::error(vec![
                Content::text("FILE_READ_ERROR"),
                Content::text(format!("Failed to read local file: {}", e)),
            ]));
        }
    };

    // Try exact match first, then fall back to Unicode-normalized matching.
    // LLMs commonly normalize curly quotes to straight quotes, en-dashes to
    // hyphens, etc. The fallback finds the original substring in the file by
    // normalizing both sides to ASCII and using char-position mapping.
    let (new_content, replaced_count) = if original_content.contains(old_str) {
        // Exact match — fast path.  Use `replacen` for single or
        // `replace` for all, and derive the count from the result to
        // avoid scanning the string twice.
        if replace_all.unwrap_or(false) {
            let result = original_content.replace(old_str, new_str);
            // Derive count from the length difference.
            let old_len = old_str.len();
            let new_len = new_str.len();
            let count = if old_len == new_len {
                // Length-neutral replacement — count via matches (unavoidable).
                original_content.matches(old_str).count()
            } else {
                let orig = original_content.len();
                let after = result.len();
                // diff = count * (new_len - old_len), signed arithmetic
                let diff = after as isize - orig as isize;
                let step = new_len as isize - old_len as isize;
                (diff / step) as usize
            };
            (result, count)
        } else {
            (original_content.replacen(old_str, new_str, 1), 1)
        }
    } else if let Some(result) = unicode_normalized_replace(
        &original_content,
        old_str,
        new_str,
        replace_all.unwrap_or(false),
    ) {
        // Unicode-normalized fallback matched
        result
    } else {
        return Ok(CallToolResult::error(vec![
            Content::text("STRING_NOT_FOUND"),
            Content::text("The string old_str was not found in the file"),
        ]));
    };

    let unified_diff = create_unified_diff(&original_content, &new_content, path, path);

    if let Err(e) = fs::write(path, &new_content) {
        error!("Failed to write local file for str_replace: {}", e);
        return Ok(CallToolResult::error(vec![
            Content::text("FILE_WRITE_ERROR"),
            Content::text(format!("Failed to write local file: {}", e)),
        ]));
    }

    let output = format!(
        "Successfully replaced {} occurrences of text (local)\n\n```diff\n{}\n```",
        replaced_count, unified_diff
    );

    Ok(CallToolResult::success(vec![Content::text(&output)]))
}

/// Create a remote file with the specified content
pub(crate) async fn create_remote(
    conn: &Arc<RemoteConnection>,
    remote_path: &str,
    original_path: &str,
    file_text: &str,
) -> Result<CallToolResult, McpError> {
    if conn.exists(remote_path).await {
        return Ok(CallToolResult::error(vec![
            Content::text("FILE_EXISTS"),
            Content::text(format!("Remote file already exists: {}", original_path)),
        ]));
    }

    // Create parent directories if needed
    if let Some(parent) = std::path::Path::new(remote_path).parent() {
        let parent_str = parent.to_string_lossy().to_string();
        if !parent_str.is_empty()
            && !conn.exists(&parent_str).await
            && let Err(e) = conn.create_directories(&parent_str).await
        {
            error!(
                "Failed to create remote parent directories '{}': {}",
                parent_str, e
            );
            return Ok(CallToolResult::error(vec![
                Content::text("CREATE_DIR_ERROR"),
                Content::text(format!(
                    "Failed to create remote parent directories '{}': {}",
                    parent_str, e
                )),
            ]));
        }
    }

    // Create the file using the correct SFTP method
    if let Err(e) = conn.create_file(remote_path, file_text.as_bytes()).await {
        error!("Failed to create remote file '{}': {}", remote_path, e);
        return Ok(CallToolResult::error(vec![
            Content::text("CREATE_ERROR"),
            Content::text(format!(
                "Failed to create remote file '{}': {}",
                remote_path, e
            )),
        ]));
    }

    let lines = file_text.lines().count();
    Ok(CallToolResult::success(vec![Content::text(format!(
        "Successfully created remote file {} with {} lines",
        original_path, lines
    ))]))
}

/// Create a local file with the specified content
pub(crate) fn create_local(path: &str, file_text: &str) -> Result<CallToolResult, McpError> {
    let path_obj = Path::new(&path);

    if path_obj.exists() {
        return Ok(CallToolResult::error(vec![
            Content::text("FILE_EXISTS"),
            Content::text(format!("File already exists: {}", path)),
        ]));
    }

    // Create parent directories if they don't exist
    if let Some(parent) = path_obj.parent()
        && !parent.exists()
        && let Err(e) = fs::create_dir_all(parent)
    {
        return Ok(CallToolResult::error(vec![
            Content::text("CREATE_DIR_ERROR"),
            Content::text(format!("Cannot create parent directories: {}", e)),
        ]));
    }

    match fs::write(path, file_text) {
        Ok(_) => {
            let lines = fs::read_to_string(path)
                .map(|content| content.lines().count())
                .unwrap_or(0);
            Ok(CallToolResult::success(vec![Content::text(format!(
                "Successfully created file {} with {} lines",
                path, lines
            ))]))
        }
        Err(e) => Ok(CallToolResult::error(vec![
            Content::text("WRITE_ERROR"),
            Content::text(format!("Cannot create file: {}", e)),
        ])),
    }
}

/// Remove a remote file or directory using native SFTP operations where possible
pub(crate) async fn remove_remote_path(
    conn: &Arc<RemoteConnection>,
    remote_path: &str,
    original_path: &str,
    recursive: bool,
) -> Result<CallToolResult, McpError> {
    if !conn.exists(remote_path).await {
        return Ok(CallToolResult::error(vec![
            Content::text("PATH_NOT_FOUND"),
            Content::text(format!("Path does not exist: {}", original_path)),
        ]));
    }

    let is_directory = conn.is_directory(remote_path).await;

    let ssh_prefix = match conn.get_ssh_prefix() {
        Ok(prefix) => prefix,
        Err(e) => {
            return Ok(CallToolResult::error(vec![
                Content::text("CONNECTION_ERROR"),
                Content::text(format!("Failed to get SSH connection info: {}", e)),
            ]));
        }
    };

    let canonical_original_path = match conn.canonicalize(remote_path).await {
        Ok(abs_path) => abs_path,
        Err(_) => remote_path.to_string(),
    };
    let ssh_prefixed_original_path = format!("{}{}", ssh_prefix, canonical_original_path);

    // Move the entire path (file or directory) to backup location - this IS the removal
    let backup_path = match FileBackupManager::move_remote_path_to_backup(conn, remote_path).await {
        Ok(backup_path) => backup_path,
        Err(e) => {
            return Ok(CallToolResult::error(vec![
                Content::text("BACKUP_ERROR"),
                Content::text(format!("Failed to move remote path to backup: {}", e)),
            ]));
        }
    };

    let ssh_prefixed_backup_path = format!("{}{}", ssh_prefix, backup_path);

    let mut backup_mapping = std::collections::HashMap::new();
    backup_mapping.insert(ssh_prefixed_original_path, ssh_prefixed_backup_path);

    let item_type = if is_directory { "directory" } else { "file" };
    let recursive_note = if is_directory && recursive {
        " (recursively)"
    } else {
        ""
    };

    let backup_xml = FileBackupManager::format_backup_xml(&backup_mapping, "remote");
    let output = format!(
        "Successfully removed {} '{}'{}\n\n{}",
        item_type, original_path, recursive_note, backup_xml
    );

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Remove a local file or directory
pub(crate) async fn remove_local_path(
    path: &str,
    recursive: bool,
) -> Result<CallToolResult, McpError> {
    let local_path = Path::new(path);

    if !local_path.exists() {
        return Ok(CallToolResult::error(vec![
            Content::text("PATH_NOT_FOUND"),
            Content::text(format!("Path does not exist: {}", path)),
        ]));
    }

    let is_directory = local_path.is_dir();

    let absolute_original_path = match local_path.canonicalize() {
        Ok(abs_path) => abs_path.to_string_lossy().to_string(),
        Err(_) => path.to_string(),
    };

    // Move the entire path (file or directory) to backup location - this IS the removal
    let backup_path = match FileBackupManager::move_local_path_to_backup(path) {
        Ok(backup_path) => backup_path,
        Err(e) => {
            return Ok(CallToolResult::error(vec![
                Content::text("BACKUP_ERROR"),
                Content::text(format!("Failed to move local path to backup: {}", e)),
            ]));
        }
    };

    let mut backup_mapping = std::collections::HashMap::new();
    backup_mapping.insert(absolute_original_path, backup_path);

    let item_type = if is_directory { "directory" } else { "file" };
    let recursive_note = if is_directory && recursive {
        " (recursively)"
    } else {
        ""
    };

    let backup_xml = FileBackupManager::format_backup_xml(&backup_mapping, "local");
    let output = format!(
        "Successfully removed {} '{}'{}\n\n{}",
        item_type, path, recursive_note, backup_xml
    );

    Ok(CallToolResult::success(vec![Content::text(output)]))
}

/// Normalize a single character: map common Unicode "fancy" characters to their
/// ASCII equivalents.  Most mappings are 1-to-1, but some are 1-to-many (e.g.
/// `…` → `...`).  Returns `None` when the character requires no normalisation.
fn normalize_unicode_char(c: char) -> Option<&'static str> {
    match c {
        // Quotation marks
        '\u{2018}' | '\u{2019}' | '\u{201A}' | '\u{2039}' | '\u{203A}' => Some("'"), // ' ' ‚ ‹ ›  → '
        '\u{FF07}' => Some("'"), // fullwidth apostrophe → '
        '\u{201C}' | '\u{201D}' | '\u{201E}' | '\u{00AB}' | '\u{00BB}' => Some("\""), // " " „ « »  → "

        // Dashes
        '\u{2010}' | '\u{2011}' | '\u{2012}' | '\u{2013}' | '\u{2014}' | '\u{2015}' => Some("-"), // ‐ ‑ ‒ – — ―  → -

        // Spaces
        '\u{00A0}' | '\u{2002}' | '\u{2003}' | '\u{2009}' | '\u{200A}' | '\u{202F}' => Some(" "), // NBSP, en/em/thin/hair/nnbsp → space

        // Dots / ellipsis  (1-to-many: one char → three chars)
        '\u{2026}' => Some("..."), // … → ...

        // Other common normalizations
        '\u{2022}' => Some("*"), // bullet → *
        '\u{00B7}' => Some("."), // middle dot → .

        _ => None,
    }
}

/// Normalize a string by mapping each character through [`normalize_unicode_char`].
///
/// Because some mappings are 1-to-many (e.g. `…` → `...`), the returned string
/// may have a **different** character count than the input.  Use
/// [`normalize_with_byte_mapping`] when you need to map positions back.
fn normalize_unicode_to_ascii(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match normalize_unicode_char(c) {
            Some(replacement) => out.push_str(replacement),
            None => out.push(c),
        }
    }
    out
}

/// Result of normalizing a string with byte-position tracking.
struct NormalizedWithMapping {
    /// The normalized string.
    text: String,
    /// For each char-boundary byte offset in `text`, the corresponding byte
    /// offset in the **original** string.
    ///
    /// Indexed by the char-boundary byte offset in `text`. There is one entry
    /// per character plus a sentinel at the end equal to the original string's
    /// byte length. This lets us translate match byte ranges from
    /// [`str::find`] directly to original byte ranges without an intermediate
    /// char-index conversion.
    norm_byte_to_orig_byte: Vec<usize>,
    /// All char-boundary byte offsets in `text` (sorted).
    /// Used to locate the *end* of a match: given the match start byte +
    /// pattern byte length, we round to the next char boundary via binary
    /// search.  Only needed when the normalized text contains multi-byte
    /// characters (e.g. non-mapped Unicode like `é`).
    char_boundaries: Vec<usize>,
}

/// Normalize `s` while building a byte-level mapping from positions in the
/// normalized output back to positions in `s`.
fn normalize_with_byte_mapping(s: &str) -> NormalizedWithMapping {
    let mut text = String::with_capacity(s.len());
    let mut norm_byte_to_orig_byte: Vec<usize> = Vec::with_capacity(s.len() + 1);
    let mut char_boundaries: Vec<usize> = Vec::with_capacity(s.len() + 1);

    for (byte_idx, c) in s.char_indices() {
        match normalize_unicode_char(c) {
            Some(replacement) => {
                for rc in replacement.chars() {
                    char_boundaries.push(text.len());
                    norm_byte_to_orig_byte.push(byte_idx);
                    text.push(rc);
                }
            }
            None => {
                char_boundaries.push(text.len());
                norm_byte_to_orig_byte.push(byte_idx);
                text.push(c);
            }
        }
    }

    // Sentinel: one past the last character.
    char_boundaries.push(text.len());
    norm_byte_to_orig_byte.push(s.len());

    NormalizedWithMapping {
        text,
        norm_byte_to_orig_byte,
        char_boundaries,
    }
}

impl NormalizedWithMapping {
    /// Convert a byte offset in the normalized `text` to the corresponding
    /// byte offset in the original string.  Returns `None` if `norm_byte` does
    /// not fall on a character boundary (should never happen for offsets
    /// returned by [`str::find`]).
    fn orig_byte_at(&self, norm_byte: usize) -> Option<usize> {
        let idx = self.char_boundaries.binary_search(&norm_byte).ok()?;
        Some(self.norm_byte_to_orig_byte[idx])
    }
}

/// Attempt to find `old_str` in `content` using Unicode-normalized matching,
/// then perform the replacement on the *original* content preserving its
/// encoding.
///
/// Returns `Some((new_content, replaced_count))` on success, `None` if the
/// normalized old_str is still not found.
///
/// Supports 1-to-many normalizations (e.g. `…` → `...`) by building a byte-
/// position mapping from the normalized characters back to original byte
/// ranges.
///
/// Uses Rust's built-in [`str::find`] (Two-Way algorithm) for O(n + m)
/// substring search instead of a naive O(n × m) character-by-character scan.
#[allow(clippy::string_slice)] // all indices from find() on normalized text + orig_byte_at() which maps to char_indices() boundaries
fn unicode_normalized_replace(
    content: &str,
    old_str: &str,
    new_str: &str,
    replace_all: bool,
) -> Option<(String, usize)> {
    let norm_old = normalize_unicode_to_ascii(old_str);

    if norm_old.is_empty() {
        return None;
    }

    // Cheap pre-check: normalize content without building the mapping.
    // If the pattern doesn't appear in the normalized content at all,
    // skip the heavier mapping allocation.
    let norm_content_quick = normalize_unicode_to_ascii(content);
    if !norm_content_quick.contains(&norm_old) {
        return None;
    }
    drop(norm_content_quick);

    // Pattern is present — build the full mapping.
    let norm_content = normalize_with_byte_mapping(content);

    // Use Rust's optimized string search (Two-Way algorithm, O(n + m)).
    // Matches are collected as (orig_byte_start, orig_byte_end) pairs.
    let norm_old_byte_len = norm_old.len();
    let mut match_orig_ranges: Vec<(usize, usize)> = Vec::new();

    if replace_all {
        let mut search_byte = 0usize;
        while search_byte + norm_old_byte_len <= norm_content.text.len() {
            if let Some(rel) = norm_content.text[search_byte..].find(&norm_old) {
                let match_start = search_byte + rel;
                let match_end = match_start + norm_old_byte_len;

                if let (Some(orig_start), Some(orig_end)) = (
                    norm_content.orig_byte_at(match_start),
                    norm_content.orig_byte_at(match_end),
                ) {
                    match_orig_ranges.push((orig_start, orig_end));
                }
                search_byte = match_end;
            } else {
                break;
            }
        }
    } else if let Some(match_start) = norm_content.text.find(&norm_old) {
        let match_end = match_start + norm_old_byte_len;

        if let (Some(orig_start), Some(orig_end)) = (
            norm_content.orig_byte_at(match_start),
            norm_content.orig_byte_at(match_end),
        ) {
            match_orig_ranges.push((orig_start, orig_end));
        }
    }

    if match_orig_ranges.is_empty() {
        return None;
    }

    let replaced_count = match_orig_ranges.len();

    // Build the result by splicing in new_str at each matched byte range.
    let mut result = String::with_capacity(content.len());
    let mut prev_byte_end = 0usize;

    for &(orig_start, orig_end) in &match_orig_ranges {
        result.push_str(&content[prev_byte_end..orig_start]);
        result.push_str(new_str);
        prev_byte_end = orig_end;
    }
    result.push_str(&content[prev_byte_end..]);

    Some((result, replaced_count))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shell_single_quote_arg_quotes_empty_and_plain_values() {
        assert_eq!(shell_single_quote_arg(""), "''");
        assert_eq!(
            shell_single_quote_arg("/tmp/path with spaces"),
            "'/tmp/path with spaces'"
        );
    }

    #[test]
    fn shell_single_quote_arg_escapes_embedded_single_quote() {
        assert_eq!(shell_single_quote_arg("a'b"), "'a'\\''b'");
    }

    #[test]
    fn remote_read_commands_quote_dynamic_path_and_pattern_args() {
        let path = "/tmp/a b;touch /tmp/pwn$(x)'z";
        let pattern = "-needle$(touch nope)'|.*";
        let glob = "*.rs';rm -rf /";

        let file_command = build_remote_grep_file_command(path, pattern, 5);
        assert_eq!(
            file_command,
            "grep -En -- '-needle$(touch nope)'\\''|.*' '/tmp/a b;touch /tmp/pwn$(x)'\\''z' 2>/dev/null | head -n 6"
        );

        let dir_command = build_remote_grep_directory_command(path, pattern, 5);
        assert_eq!(
            dir_command,
            "grep -rEn --include='*' -- '-needle$(touch nope)'\\''|.*' '/tmp/a b;touch /tmp/pwn$(x)'\\''z' 2>/dev/null | head -n 6"
        );

        let glob_command = build_remote_grep_directory_with_glob_command(path, pattern, glob, 5);
        assert_eq!(
            glob_command,
            "find '/tmp/a b;touch /tmp/pwn$(x)'\\''z' -name '*.rs'\\'';rm -rf /' -type f -exec grep -EHn -- '-needle$(touch nope)'\\''|.*' {} \\; 2>/dev/null | head -n 6"
        );
    }

    // ---------------------------------------------------------------
    // normalize_unicode_char / normalize_unicode_to_ascii
    // ---------------------------------------------------------------

    #[test]
    fn test_normalize_straight_quotes_unchanged() {
        assert_eq!(normalize_unicode_to_ascii("it's fine"), "it's fine");
    }

    #[test]
    fn test_normalize_curly_single_quotes() {
        // U+2018 LEFT SINGLE QUOTATION MARK
        assert_eq!(normalize_unicode_to_ascii("it\u{2018}s"), "it's");
        // U+2019 RIGHT SINGLE QUOTATION MARK
        assert_eq!(normalize_unicode_to_ascii("shouldn\u{2019}t"), "shouldn't");
    }

    #[test]
    fn test_normalize_curly_double_quotes() {
        // U+201C / U+201D
        assert_eq!(
            normalize_unicode_to_ascii("\u{201C}hello\u{201D}"),
            "\"hello\""
        );
    }

    #[test]
    fn test_normalize_en_dash() {
        assert_eq!(normalize_unicode_to_ascii("a\u{2013}b"), "a-b");
    }

    #[test]
    fn test_normalize_em_dash() {
        assert_eq!(normalize_unicode_to_ascii("a\u{2014}b"), "a-b");
    }

    #[test]
    fn test_normalize_figure_dash() {
        assert_eq!(normalize_unicode_to_ascii("a\u{2012}b"), "a-b");
    }

    #[test]
    fn test_normalize_hyphen_unicode() {
        // U+2010 HYPHEN
        assert_eq!(normalize_unicode_to_ascii("a\u{2010}b"), "a-b");
    }

    #[test]
    fn test_normalize_nbsp() {
        assert_eq!(
            normalize_unicode_to_ascii("hello\u{00A0}world"),
            "hello world"
        );
    }

    #[test]
    fn test_normalize_preserves_char_count_for_1to1() {
        // Only 1-to-1 mappings in this input — char count preserved
        let input = "\u{201C}shouldn\u{2019}t\u{201D} \u{2013} done";
        let normalized = normalize_unicode_to_ascii(input);
        assert_eq!(input.chars().count(), normalized.chars().count());
    }

    #[test]
    fn test_normalize_ellipsis_expands() {
        // Ellipsis is 1-to-3 mapping
        assert_eq!(normalize_unicode_to_ascii("wait\u{2026}"), "wait...");
        // Char count grows: 5 input chars → 7 output chars
        assert_eq!("wait\u{2026}".chars().count(), 5);
        assert_eq!(
            normalize_unicode_to_ascii("wait\u{2026}").chars().count(),
            7
        );
    }

    #[test]
    fn test_normalize_pure_ascii_passthrough() {
        let ascii = "The quick brown fox jumps over the lazy dog. 0123456789 !@#$%^&*()";
        assert_eq!(normalize_unicode_to_ascii(ascii), ascii);
    }

    #[test]
    fn test_normalize_bullet() {
        assert_eq!(normalize_unicode_to_ascii("\u{2022} item"), "* item");
    }

    #[test]
    fn test_normalize_non_breaking_hyphen() {
        assert_eq!(
            normalize_unicode_to_ascii("non\u{2011}breaking"),
            "non-breaking"
        );
    }

    #[test]
    fn test_normalize_guillemets() {
        assert_eq!(
            normalize_unicode_to_ascii("\u{00AB}quoted\u{00BB}"),
            "\"quoted\""
        );
    }

    #[test]
    fn test_normalize_fullwidth_apostrophe() {
        assert_eq!(normalize_unicode_to_ascii("it\u{FF07}s"), "it's");
    }

    #[test]
    fn test_normalize_mixed_unicode_and_ascii() {
        let input = "It\u{2019}s a \u{201C}test\u{201D} \u{2013} really";
        let expected = "It's a \"test\" - really";
        assert_eq!(normalize_unicode_to_ascii(input), expected);
    }

    // ---------------------------------------------------------------
    // normalize_with_byte_mapping
    // ---------------------------------------------------------------

    #[test]
    fn test_byte_mapping_ascii() {
        let m = normalize_with_byte_mapping("abc");
        assert_eq!(m.text, "abc");
        assert_eq!(m.char_boundaries, vec![0, 1, 2, 3]); // includes sentinel
        assert_eq!(m.norm_byte_to_orig_byte, vec![0, 1, 2, 3]); // includes sentinel
    }

    #[test]
    fn test_byte_mapping_ellipsis() {
        // … is 3 bytes in UTF-8, maps to 3 normalized chars "..."
        let m = normalize_with_byte_mapping("x\u{2026}y");
        assert_eq!(m.text, "x...y");
        // char boundaries: x=0, .=1, .=2, .=3, y=4, sentinel=5
        assert_eq!(m.char_boundaries, vec![0, 1, 2, 3, 4, 5]);
        // 'x' at orig 0, all three '.' at orig 1 (start of …), 'y' at orig 4, sentinel=5
        assert_eq!(m.norm_byte_to_orig_byte, vec![0, 1, 1, 1, 4, 5]);
    }

    #[test]
    fn test_byte_mapping_curly_quote() {
        // U+2019 is 3 bytes in UTF-8
        let m = normalize_with_byte_mapping("a\u{2019}b");
        assert_eq!(m.text, "a'b");
        assert_eq!(m.char_boundaries, vec![0, 1, 2, 3]);
        assert_eq!(m.norm_byte_to_orig_byte, vec![0, 1, 4, 5]);
    }

    #[test]
    fn test_byte_mapping_multibyte_passthrough() {
        // 'é' (2 bytes) is NOT in the normalization map — preserved as-is
        let m = normalize_with_byte_mapping("café");
        assert_eq!(m.text, "café");
        // c=0, a=1, f=2, é=3 (norm byte), sentinel=5 (norm byte, since é is 2 bytes)
        assert_eq!(m.char_boundaries, vec![0, 1, 2, 3, 5]);
        // c→0, a→1, f→2, é→3, sentinel→5 (orig len)
        assert_eq!(m.norm_byte_to_orig_byte, vec![0, 1, 2, 3, 5]);
    }

    // ---------------------------------------------------------------
    // unicode_normalized_replace — exact match still works
    // ---------------------------------------------------------------

    #[test]
    fn test_normalized_replace_exact_ascii() {
        let content = "hello world";
        let result = unicode_normalized_replace(content, "world", "rust", false);
        assert_eq!(result, Some(("hello rust".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_no_match() {
        let content = "hello world";
        assert_eq!(
            unicode_normalized_replace(content, "xyz", "abc", false),
            None
        );
    }

    // ---------------------------------------------------------------
    // unicode_normalized_replace — curly quote fallback
    // ---------------------------------------------------------------

    #[test]
    fn test_normalized_replace_curly_apostrophe() {
        // File has curly quote, LLM sends straight quote
        let content = "Infrastructure shouldn\u{2019}t be this hard.";
        let old_str = "Infrastructure shouldn't be this hard.";
        let new_str = "Infra is easy.";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("Infra is easy.".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_preserves_surrounding_content() {
        let content = "before shouldn\u{2019}t after";
        let old_str = "shouldn't";
        let new_str = "REPLACED";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("before REPLACED after".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_curly_double_quotes() {
        let content = "She said \u{201C}hello\u{201D} loudly";
        let old_str = "\"hello\"";
        let new_str = "\"hi\"";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("She said \"hi\" loudly".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_en_dash() {
        let content = "pages 10\u{2013}20 of the book";
        let old_str = "10-20";
        let new_str = "10-30";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("pages 10-30 of the book".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_em_dash() {
        let content = "word\u{2014}another word";
        let old_str = "word-another";
        let new_str = "one-two";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("one-two word".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_nbsp_to_space() {
        let content = "hello\u{00A0}world";
        let old_str = "hello world";
        let new_str = "hi there";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("hi there".to_string(), 1)));
    }

    // ---------------------------------------------------------------
    // unicode_normalized_replace — ellipsis (1-to-many mapping)
    // ---------------------------------------------------------------

    #[test]
    fn test_normalized_replace_ellipsis_in_content() {
        // File has … (1 char), LLM sends ... (3 chars)
        let content = "wait\u{2026} what?";
        let old_str = "wait... what?";
        let new_str = "oh!";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("oh!".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_ellipsis_in_old_str() {
        // File has ... (3 chars), LLM sends … (1 char, normalizes to ...)
        let content = "wait... what?";
        let old_str = "wait\u{2026} what?";
        let new_str = "oh!";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("oh!".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_ellipsis_preserves_surroundings() {
        let content = "before\u{2026}after";
        let old_str = "...";
        let new_str = "---";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("before---after".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_ellipsis_replace_all() {
        let content = "one\u{2026}two\u{2026}three";
        let old_str = "...";
        let new_str = " ";
        let result = unicode_normalized_replace(content, old_str, new_str, true);
        assert_eq!(result, Some(("one two three".to_string(), 2)));
    }

    #[test]
    fn test_normalized_replace_ellipsis_with_other_unicode() {
        // Mix of ellipsis and curly quotes
        let content = "She said \u{201C}wait\u{2026}\u{201D}";
        let old_str = "\"wait...\"";
        let new_str = "\"go!\"";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("She said \"go!\"".to_string(), 1)));
    }

    // ---------------------------------------------------------------
    // unicode_normalized_replace — additional Unicode chars
    // ---------------------------------------------------------------

    #[test]
    fn test_normalized_replace_figure_dash() {
        let content = "pages 10\u{2012}20";
        let old_str = "10-20";
        let new_str = "10-30";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("pages 10-30".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_unicode_hyphen() {
        let content = "non\u{2010}breaking";
        let old_str = "non-breaking";
        let new_str = "unbreakable";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("unbreakable".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_fullwidth_apostrophe() {
        let content = "it\u{FF07}s fine";
        let old_str = "it's fine";
        let new_str = "all good";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("all good".to_string(), 1)));
    }

    // ---------------------------------------------------------------
    // unicode_normalized_replace — replace_all
    // ---------------------------------------------------------------

    #[test]
    fn test_normalized_replace_all_multiple() {
        let content = "shouldn\u{2019}t and shouldn\u{2019}t again";
        let old_str = "shouldn't";
        let new_str = "should not";
        let result = unicode_normalized_replace(content, old_str, new_str, true);
        assert_eq!(
            result,
            Some(("should not and should not again".to_string(), 2))
        );
    }

    #[test]
    fn test_normalized_replace_all_false_stops_at_first() {
        let content = "shouldn\u{2019}t and shouldn\u{2019}t again";
        let old_str = "shouldn't";
        let new_str = "should not";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(
            result,
            Some(("should not and shouldn\u{2019}t again".to_string(), 1))
        );
    }

    // ---------------------------------------------------------------
    // unicode_normalized_replace — multiple different Unicode chars
    // ---------------------------------------------------------------

    #[test]
    fn test_normalized_replace_mixed_unicode() {
        // File has: curly quotes + en-dash
        let content = "\u{201C}10\u{2013}20\u{201D}";
        let old_str = "\"10-20\"";
        let new_str = "range";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("range".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_readme_scenario() {
        // The exact scenario from the bug: file has U+2019 in "shouldn't"
        // and U+2013 en-dashes elsewhere. LLM normalizes to ASCII.
        let content = concat!(
            "Infrastructure shouldn\u{2019}t be this hard.\n",
            "- `--disable-secret-redaction` \u{2013} **not recommended**\n",
            "- `--privacy-mode` \u{2013} redacts additional data\n",
        );

        // LLM sends old_str with the first line only (ASCII apostrophe)
        let old_str = "Infrastructure shouldn't be this hard.";
        let new_str = "Infrastructure is easy.";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 1);
        assert!(new_content.starts_with("Infrastructure is easy.\n"));
        // Rest of the file (with en-dashes) should be untouched
        assert!(new_content.contains("\u{2013}"));
    }

    // ---------------------------------------------------------------
    // unicode_normalized_replace — edge cases
    // ---------------------------------------------------------------

    #[test]
    fn test_normalized_replace_empty_old_str() {
        assert_eq!(unicode_normalized_replace("content", "", "x", false), None);
    }

    #[test]
    fn test_normalized_replace_empty_content() {
        assert_eq!(unicode_normalized_replace("", "hello", "x", false), None);
    }

    #[test]
    fn test_normalized_replace_entire_content() {
        let content = "shouldn\u{2019}t";
        let old_str = "shouldn't";
        let new_str = "should not";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("should not".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_at_start() {
        let content = "\u{201C}hello\u{201D} world";
        let old_str = "\"hello\"";
        let new_str = "\"hi\"";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("\"hi\" world".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_at_end() {
        let content = "world \u{201C}hello\u{201D}";
        let old_str = "\"hello\"";
        let new_str = "\"hi\"";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("world \"hi\"".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_no_false_positive_on_ascii() {
        // When both content and old_str are pure ASCII and don't match,
        // the normalized path should also return None.
        assert_eq!(
            unicode_normalized_replace("hello world", "goodbye", "x", false),
            None
        );
    }

    #[test]
    fn test_normalized_replace_preserves_other_unicode() {
        // Unicode that is NOT in the normalization map should be preserved
        let content = "café shouldn\u{2019}t break";
        let old_str = "shouldn't";
        let new_str = "should not";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("café should not break".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_adjacent_unicode_chars() {
        // Multiple unicode chars right next to each other
        let content = "\u{201C}\u{2019}\u{2013}\u{201D}";
        let old_str = "\"'-\"";
        let new_str = "X";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("X".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_only_unicode_differs() {
        // Content and old_str are identical except for one Unicode char
        let content = "a\u{00A0}b"; // non-breaking space
        let old_str = "a b"; // regular space
        let new_str = "a_b";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert_eq!(result, Some(("a_b".to_string(), 1)));
    }

    #[test]
    fn test_normalized_replace_large_multiline() {
        // Simulates a realistic str_replace with a large multi-line old_str
        let content = concat!(
            "# Title\n\n",
            "Some text before.\n\n",
            "Infrastructure shouldn\u{2019}t be this hard. VAC lets developers secure, deploy, and run infra.\n\n",
            "## Features\n\n",
            "- Feature 1 \u{2013} description\n",
            "- Feature 2 \u{2013} description\n",
            "\nMore text after.\n",
        );

        let old_str = concat!(
            "Infrastructure shouldn't be this hard. VAC lets developers secure, deploy, and run infra.\n\n",
            "## Features\n\n",
            "- Feature 1 - description\n",
            "- Feature 2 - description\n",
        );

        let new_str = "## Simplified\n\nJust works.\n";

        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 1);
        assert!(new_content.contains("# Title"));
        assert!(new_content.contains("Some text before."));
        assert!(new_content.contains("## Simplified\n\nJust works.\n"));
        assert!(new_content.contains("More text after."));
        // Original unicode chars that were NOT in old_str are preserved
        assert!(!new_content.contains("\u{2019}"));
        assert!(!new_content.contains("\u{2013}"));
    }

    #[test]
    fn test_normalized_replace_all_non_overlapping() {
        let content = "a\u{2013}b c\u{2013}d";
        let old_str = "-";
        // This should match the normalized dashes
        let new_str = "=";
        let result = unicode_normalized_replace(content, old_str, new_str, true);
        assert_eq!(result, Some(("a=b c=d".to_string(), 2)));
    }

    // ---------------------------------------------------------------
    // unicode_normalized_replace — ellipsis in multi-line realistic scenario
    // ---------------------------------------------------------------

    #[test]
    fn test_normalized_replace_ellipsis_multiline() {
        let content = concat!("Loading\u{2026}\n", "Please wait\u{2026}\n", "Done!\n",);
        let old_str = concat!("Loading...\n", "Please wait...\n",);
        let new_str = "Loaded!\n";
        let result = unicode_normalized_replace(content, old_str, new_str, false);
        assert!(result.is_some());
        let (new_content, count) = result.unwrap();
        assert_eq!(count, 1);
        assert_eq!(new_content, "Loaded!\nDone!\n");
    }

    #[test]
    fn test_normalized_replace_multiple_ellipsis_replace_all() {
        let content = "a\u{2026}b\u{2026}c";
        let old_str = "...";
        let new_str = "***";
        let result = unicode_normalized_replace(content, old_str, new_str, true);
        assert_eq!(result, Some(("a***b***c".to_string(), 2)));
    }
}
