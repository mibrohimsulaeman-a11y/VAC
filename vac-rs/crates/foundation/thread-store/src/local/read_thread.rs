use chrono::DateTime;
use chrono::Utc;
use vac_protocol::protocol::AskForApproval;
use vac_protocol::protocol::SandboxPolicy;
use vac_protocol::protocol::SessionMetaLine;
use vac_protocol::protocol::SessionSource;
use vac_rollout::RolloutRecorder;
use vac_rollout::find_archived_thread_path_by_id_str;
use vac_rollout::find_thread_name_by_id;
use vac_rollout::find_thread_path_by_id_str;
use vac_rollout::read_session_meta_line;
use vac_rollout::read_thread_item_from_rollout;
use vac_state::StateRuntime;
use vac_state::ThreadMetadata;

use super::LocalThreadStore;
use super::helpers::distinct_thread_metadata_title;
use super::helpers::git_info_from_parts;
use super::helpers::rollout_path_is_archived;
use super::helpers::set_thread_name_from_title;
use super::helpers::stored_thread_from_rollout_item;
use super::live_writer;
use crate::ReadThreadParams;
use crate::StoredThread;
use crate::StoredThreadHistory;
use crate::ThreadStoreError;
use crate::ThreadStoreResult;

pub(super) async fn read_thread(
    store: &LocalThreadStore,
    params: ReadThreadParams,
) -> ThreadStoreResult<StoredThread> {
    let thread_id = params.thread_id;
    if let Some(metadata) = read_sqlite_metadata(store, thread_id).await
        && (params.include_archived
            || (metadata.archived_at.is_none()
                && !rollout_path_is_archived(
                    store.config.vac_home.as_path(),
                    metadata.rollout_path.as_path(),
                )))
        && (!params.include_history
            || sqlite_rollout_path_can_load_history_for_thread(
                store,
                &metadata.rollout_path,
                thread_id,
            )
            .await)
    {
        let mut thread = stored_thread_from_sqlite_metadata(store, metadata).await;
        if !params.include_history
            && let Some(rollout_path) = thread.rollout_path.clone()
            && let Ok(mut rollout_thread) = read_thread_from_rollout_path(store, rollout_path).await
            && rollout_thread.thread_id == thread_id
            && (params.include_archived || rollout_thread.archived_at.is_none())
            && !rollout_thread.preview.is_empty()
        {
            if thread.name.is_some() {
                rollout_thread.name = thread.name;
            }
            rollout_thread.git_info = thread.git_info;
            thread = rollout_thread;
        }
        attach_history_if_requested(&mut thread, params.include_history).await?;
        return Ok(thread);
    }

    let path = resolve_rollout_path(store, thread_id, params.include_archived)
        .await?
        .ok_or_else(|| ThreadStoreError::InvalidRequest {
            message: format!("no rollout found for thread id {thread_id}"),
        })?;

    let mut thread = read_thread_from_rollout_path(store, path).await?;
    attach_history_if_requested(&mut thread, params.include_history).await?;
    Ok(thread)
}

async fn sqlite_rollout_path_can_load_history_for_thread(
    store: &LocalThreadStore,
    path: &std::path::Path,
    thread_id: vac_protocol::ThreadId,
) -> bool {
    if !tokio::fs::try_exists(path).await.unwrap_or(false) {
        return false;
    }
    // SQLite metadata can outlive a moved/recreated rollout path. When history is
    // requested, verify the path still resolves to the requested thread before
    // trusting it as the source replay.
    read_thread_from_rollout_path(store, path.to_path_buf())
        .await
        .is_ok_and(|thread| thread.thread_id == thread_id)
}

pub(super) async fn read_thread_by_rollout_path(
    store: &LocalThreadStore,
    rollout_path: std::path::PathBuf,
    include_archived: bool,
    include_history: bool,
) -> ThreadStoreResult<StoredThread> {
    let path = resolve_requested_rollout_path(store, rollout_path)?;
    let mut thread = read_thread_from_rollout_path(store, path).await?;
    if !include_archived && thread.archived_at.is_some() {
        return Err(ThreadStoreError::InvalidRequest {
            message: format!("thread {} is archived", thread.thread_id),
        });
    }
    if let Some(metadata) = read_sqlite_metadata(store, thread.thread_id).await {
        let existing_git_info = thread.git_info.take();
        let (fallback_sha, fallback_branch, fallback_origin_url) = match existing_git_info {
            Some(info) => (
                info.commit_hash.map(|sha| sha.0),
                info.branch,
                info.repository_url,
            ),
            None => (None, None, None),
        };
        thread.git_info = git_info_from_parts(
            metadata.git_sha.or(fallback_sha),
            metadata.git_branch.or(fallback_branch),
            metadata.git_origin_url.or(fallback_origin_url),
        );
    }
    attach_history_if_requested(&mut thread, include_history).await?;
    Ok(thread)
}

fn resolve_requested_rollout_path(
    store: &LocalThreadStore,
    rollout_path: std::path::PathBuf,
) -> ThreadStoreResult<std::path::PathBuf> {
    let path = if rollout_path.is_relative() {
        store.config.vac_home.join(rollout_path)
    } else {
        rollout_path
    };
    std::fs::canonicalize(&path).map_err(|err| ThreadStoreError::InvalidRequest {
        message: format!("failed to resolve rollout path `{}`: {err}", path.display()),
    })
}

async fn attach_history_if_requested(
    thread: &mut StoredThread,
    include_history: bool,
) -> ThreadStoreResult<()> {
    if !include_history {
        return Ok(());
    }
    let thread_id = thread.thread_id;
    let Some(path) = thread.rollout_path.clone() else {
        return Err(ThreadStoreError::Internal {
            message: format!("failed to load thread history for thread {thread_id}"),
        });
    };
    let items = load_history_items(&path).await?;
    thread.history = Some(StoredThreadHistory { thread_id, items });
    Ok(())
}

async fn resolve_rollout_path(
    store: &LocalThreadStore,
    thread_id: vac_protocol::ThreadId,
    include_archived: bool,
) -> ThreadStoreResult<Option<std::path::PathBuf>> {
    if let Ok(path) = live_writer::rollout_path(store, thread_id).await
        && tokio::fs::try_exists(path.as_path()).await.map_err(|err| {
            ThreadStoreError::InvalidRequest {
                message: format!("failed to check rollout path for thread id {thread_id}: {err}"),
            }
        })?
        && (include_archived || !rollout_path_is_archived(store.config.vac_home.as_path(), &path))
    {
        return Ok(Some(path));
    }

    if include_archived {
        match find_thread_path_by_id_str(store.config.vac_home.as_path(), &thread_id.to_string())
            .await
            .map_err(|err| ThreadStoreError::InvalidRequest {
                message: format!("failed to locate thread id {thread_id}: {err}"),
            })? {
            Some(path) => Ok(Some(path)),
            None => find_archived_thread_path_by_id_str(
                store.config.vac_home.as_path(),
                &thread_id.to_string(),
            )
            .await
            .map_err(|err| ThreadStoreError::InvalidRequest {
                message: format!("failed to locate archived thread id {thread_id}: {err}"),
            }),
        }
    } else {
        find_thread_path_by_id_str(store.config.vac_home.as_path(), &thread_id.to_string())
            .await
            .map_err(|err| ThreadStoreError::InvalidRequest {
                message: format!("failed to locate thread id {thread_id}: {err}"),
            })
    }
}

async fn read_thread_from_rollout_path(
    store: &LocalThreadStore,
    path: std::path::PathBuf,
) -> ThreadStoreResult<StoredThread> {
    let Some(item) = read_thread_item_from_rollout(path.clone()).await else {
        return stored_thread_from_session_meta(store, path).await;
    };
    let archived = rollout_path_is_archived(store.config.vac_home.as_path(), path.as_path());
    let mut thread = stored_thread_from_rollout_item(
        item,
        archived,
        store.config.default_model_provider_id.as_str(),
    )
    .ok_or_else(|| ThreadStoreError::Internal {
        message: format!("failed to read thread id from {}", path.display()),
    })?;
    if let Ok(meta_line) = read_session_meta_line(path.as_path()).await {
        thread.branched_from_id = meta_line.meta.branched_from_id;
        if let Some(model_provider) = meta_line
            .meta
            .model_provider
            .filter(|provider| !provider.is_empty())
        {
            thread.model_provider = model_provider;
        }
    }
    if let Ok(Some(title)) =
        find_thread_name_by_id(store.config.vac_home.as_path(), &thread.thread_id).await
    {
        set_thread_name_from_title(&mut thread, title);
    }
    Ok(thread)
}

async fn load_history_items(
    path: &std::path::Path,
) -> ThreadStoreResult<Vec<vac_protocol::protocol::RolloutItem>> {
    let (items, _, _) = RolloutRecorder::load_rollout_items(path)
        .await
        .map_err(|err| ThreadStoreError::Internal {
            message: format!("failed to load thread history {}: {err}", path.display()),
        })?;
    Ok(items)
}

async fn read_sqlite_metadata(
    store: &LocalThreadStore,
    thread_id: vac_protocol::ThreadId,
) -> Option<ThreadMetadata> {
    let runtime = StateRuntime::init(
        store.config.sqlite_home.clone(),
        store.config.default_model_provider_id.clone(),
    )
    .await
    .ok()?;
    runtime.get_thread(thread_id).await.ok().flatten()
}

async fn stored_thread_from_sqlite_metadata(
    store: &LocalThreadStore,
    metadata: ThreadMetadata,
) -> StoredThread {
    let name = match distinct_thread_metadata_title(&metadata) {
        Some(title) => Some(title),
        None => find_thread_name_by_id(store.config.vac_home.as_path(), &metadata.id)
            .await
            .ok()
            .flatten(),
    };
    let branched_from_id = read_session_meta_line(metadata.rollout_path.as_path())
        .await
        .ok()
        .and_then(|meta_line| meta_line.meta.branched_from_id);
    StoredThread {
        thread_id: metadata.id,
        rollout_path: Some(metadata.rollout_path),
        branched_from_id,
        preview: metadata.first_user_message.clone().unwrap_or_default(),
        name,
        model_provider: if metadata.model_provider.is_empty() {
            store.config.default_model_provider_id.clone()
        } else {
            metadata.model_provider
        },
        model: metadata.model,
        reasoning_effort: metadata.reasoning_effort,
        created_at: metadata.created_at,
        updated_at: metadata.updated_at,
        archived_at: metadata.archived_at,
        cwd: metadata.cwd,
        cli_version: metadata.cli_version,
        source: parse_session_source(&metadata.source),
        agent_nickname: metadata.agent_nickname,
        agent_role: metadata.agent_role,
        agent_path: metadata.agent_path,
        git_info: git_info_from_parts(
            metadata.git_sha,
            metadata.git_branch,
            metadata.git_origin_url,
        ),
        approval_mode: parse_or_default(&metadata.approval_mode, AskForApproval::OnRequest),
        sandbox_policy: parse_or_default(
            &metadata.sandbox_policy,
            SandboxPolicy::new_read_only_policy(),
        ),
        token_usage: None,
        first_user_message: metadata.first_user_message,
        history: None,
    }
}

async fn stored_thread_from_session_meta(
    store: &LocalThreadStore,
    path: std::path::PathBuf,
) -> ThreadStoreResult<StoredThread> {
    let meta_line = read_session_meta_line(path.as_path())
        .await
        .map_err(|err| ThreadStoreError::Internal {
            message: format!("failed to read thread {}: {err}", path.display()),
        })?;
    let archived = rollout_path_is_archived(store.config.vac_home.as_path(), path.as_path());
    Ok(stored_thread_from_meta_line(
        store, meta_line, path, archived,
    ))
}

fn stored_thread_from_meta_line(
    store: &LocalThreadStore,
    meta_line: SessionMetaLine,
    path: std::path::PathBuf,
    archived: bool,
) -> StoredThread {
    let created_at = parse_rfc3339_non_optional(&meta_line.meta.timestamp).unwrap_or_else(Utc::now);
    let updated_at = std::fs::metadata(path.as_path())
        .ok()
        .and_then(|meta| meta.modified().ok())
        .map(DateTime::<Utc>::from)
        .unwrap_or(created_at);
    StoredThread {
        thread_id: meta_line.meta.id,
        rollout_path: Some(path),
        branched_from_id: meta_line.meta.branched_from_id,
        preview: String::new(),
        name: None,
        model_provider: meta_line
            .meta
            .model_provider
            .filter(|provider| !provider.is_empty())
            .unwrap_or_else(|| store.config.default_model_provider_id.clone()),
        model: None,
        reasoning_effort: None,
        created_at,
        updated_at,
        archived_at: archived.then_some(updated_at),
        cwd: meta_line.meta.cwd,
        cli_version: meta_line.meta.cli_version,
        source: meta_line.meta.source,
        agent_nickname: meta_line.meta.agent_nickname,
        agent_role: meta_line.meta.agent_role,
        agent_path: meta_line.meta.agent_path,
        git_info: meta_line.git,
        approval_mode: AskForApproval::OnRequest,
        sandbox_policy: SandboxPolicy::new_read_only_policy(),
        token_usage: None,
        first_user_message: None,
        history: None,
    }
}

fn parse_session_source(source: &str) -> SessionSource {
    serde_json::from_str(source)
        .or_else(|_| serde_json::from_value(serde_json::Value::String(source.to_string())))
        .unwrap_or(SessionSource::Unknown)
}

fn parse_or_default<T>(value: &str, default: T) -> T
where
    T: serde::de::DeserializeOwned,
{
    serde_json::from_str(value)
        .or_else(|_| serde_json::from_value(serde_json::Value::String(value.to_string())))
        .unwrap_or(default)
}

fn parse_rfc3339_non_optional(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

#[cfg(test)]
mod tests;
