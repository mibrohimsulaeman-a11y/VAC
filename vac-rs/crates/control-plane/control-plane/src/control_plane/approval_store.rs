use super::approval_lifecycle::ApprovalDecision;
use super::approval_lifecycle::ApprovalRequest;
use super::approval_lifecycle::ApprovalRequestId;
use super::approval_lifecycle::ApprovalStore;
use super::approval_lifecycle::ApprovalStoreError;
use super::approval_lifecycle::InMemoryApprovalStore;
use chrono::DateTime;
use chrono::Utc;
use sqlx::Row;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::sqlite::SqlitePoolOptions;
use std::collections::BTreeMap;
use std::fs;
use std::future::Future;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::str::FromStr;
use std::thread;
use std::time::Duration;
use tokio::runtime::Builder;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileApprovalStore {
    path: PathBuf,
    inner: InMemoryApprovalStore,
}

impl FileApprovalStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, ApprovalStoreError> {
        let path = path.into();
        let requests = read_requests(&path)?;
        Ok(Self {
            path,
            inner: InMemoryApprovalStore::from_requests(requests),
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn pending_count(&self) -> usize {
        self.inner.pending_count()
    }

    pub fn request(
        &mut self,
        request: ApprovalRequest,
    ) -> Result<ApprovalRequestId, ApprovalStoreError> {
        let id = self.inner.request(request);
        self.persist()?;
        Ok(id)
    }

    pub fn resolve(
        &mut self,
        id: &ApprovalRequestId,
        decision: ApprovalDecision,
    ) -> Result<ApprovalRequest, ApprovalStoreError> {
        let resolved = self.inner.resolve(id, decision)?;
        self.persist()?;
        Ok(resolved)
    }

    pub fn get(&self, id: &ApprovalRequestId) -> Option<&ApprovalRequest> {
        self.inner.get(id)
    }

    pub fn expire_due(
        &mut self,
        now: DateTime<Utc>,
        reason: impl Into<String>,
    ) -> Result<Vec<ApprovalRequest>, ApprovalStoreError> {
        let expired = self.inner.expire_due(now, reason);
        if !expired.is_empty() {
            self.persist()?;
        }
        Ok(expired)
    }

    pub fn spawn_ttl_sweeper(
        path: impl Into<PathBuf>,
        interval: Duration,
        reason: impl Into<String>,
    ) -> thread::JoinHandle<()> {
        let path = path.into();
        let reason = reason.into();
        thread::spawn(move || {
            loop {
                thread::sleep(interval);
                match Self::open(path.clone()) {
                    Ok(mut store) => {
                        if let Err(err) = store.expire_due(Utc::now(), reason.clone()) {
                            tracing::warn!(error = %err, path = %path.display(), "approval ttl sweeper failed");
                        }
                    }
                    Err(err) => {
                        tracing::warn!(error = %err, path = %path.display(), "approval ttl sweeper failed to open store");
                    }
                }
            }
        })
    }

    fn persist(&self) -> Result<(), ApprovalStoreError> {
        write_requests(&self.path, self.inner.requests())
    }
}

fn read_requests(
    path: &Path,
) -> Result<BTreeMap<ApprovalRequestId, ApprovalRequest>, ApprovalStoreError> {
    if let Some(requests) = read_legacy_json_requests(path)? {
        if path.exists() {
            fs::remove_file(path).map_err(|err| persist_error(path, err))?;
        }
        write_requests(path, &requests)?;
        return Ok(requests);
    }

    run_sqlite(path, async {
        let pool = open_pool(path).await?;
        ensure_schema(&pool, path).await?;
        let rows = sqlx::query("SELECT id, payload_json FROM approval_requests ORDER BY id")
            .fetch_all(&pool)
            .await
            .map_err(|err| load_sql_error(path, err))?;

        let mut requests = BTreeMap::new();
        for row in rows {
            let id: String = row.get("id");
            let payload_json: String = row.get("payload_json");
            let request: ApprovalRequest =
                serde_json::from_str(&payload_json).map_err(|err| ApprovalStoreError::Load {
                    path: path.display().to_string(),
                    message: format!("failed to decode approval request `{id}`: {err}"),
                })?;
            requests.insert(request.id.clone(), request);
        }
        Ok(requests)
    })
}

fn write_requests(
    path: &Path,
    requests: &BTreeMap<ApprovalRequestId, ApprovalRequest>,
) -> Result<(), ApprovalStoreError> {
    run_sqlite(path, async {
        let pool = open_pool(path).await?;
        ensure_schema(&pool, path).await?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|err| persist_sql_error(path, err))?;
        sqlx::query("DELETE FROM approval_requests")
            .execute(&mut *tx)
            .await
            .map_err(|err| persist_sql_error(path, err))?;
        for request in requests.values() {
            let payload_json =
                serde_json::to_string(request).map_err(|err| ApprovalStoreError::Persist {
                    path: path.display().to_string(),
                    message: err.to_string(),
                })?;
            sqlx::query(
                "INSERT INTO approval_requests \
                 (id, workflow_id, step_id, capability_id, status, requested_at, expires_at, decided_at, payload_json) \
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(request.id.as_str())
            .bind(&request.workflow_id)
            .bind(&request.step_id)
            .bind(&request.capability_id)
            .bind(request.status.as_str())
            .bind(request.requested_at.to_rfc3339())
            .bind(request.expires_at.map(|ts| ts.to_rfc3339()))
            .bind(request.decided_at.map(|ts| ts.to_rfc3339()))
            .bind(payload_json)
            .execute(&mut *tx)
            .await
            .map_err(|err| persist_sql_error(path, err))?;
        }
        tx.commit()
            .await
            .map_err(|err| persist_sql_error(path, err))?;
        Ok(())
    })
}

fn read_legacy_json_requests(
    path: &Path,
) -> Result<Option<BTreeMap<ApprovalRequestId, ApprovalRequest>>, ApprovalStoreError> {
    if !path.exists() {
        return Ok(None);
    }
    let raw = match fs::read(path) {
        Ok(raw) => raw,
        Err(err) => return Err(load_error(path, err)),
    };
    if raw.iter().all(u8::is_ascii_whitespace) {
        return Ok(Some(BTreeMap::new()));
    }
    let first = raw.iter().copied().find(|byte| !byte.is_ascii_whitespace());
    if !matches!(first, Some(b'{') | Some(b'[')) {
        return Ok(None);
    }
    let raw = String::from_utf8(raw).map_err(|err| ApprovalStoreError::Load {
        path: path.display().to_string(),
        message: err.to_string(),
    })?;
    serde_json::from_str(&raw)
        .map(Some)
        .map_err(|err| ApprovalStoreError::Load {
            path: path.display().to_string(),
            message: err.to_string(),
        })
}

fn run_sqlite<T, F>(path: &Path, future: F) -> Result<T, ApprovalStoreError>
where
    F: Future<Output = Result<T, ApprovalStoreError>>,
{
    let runtime = Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|err| ApprovalStoreError::Load {
            path: path.display().to_string(),
            message: err.to_string(),
        })?;
    runtime.block_on(future)
}

async fn open_pool(path: &Path) -> Result<SqlitePool, ApprovalStoreError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| persist_error(parent, err))?;
    }
    let options = SqliteConnectOptions::from_str(&format!("sqlite://{}", path.display()))
        .map_err(|err| ApprovalStoreError::Load {
            path: path.display().to_string(),
            message: err.to_string(),
        })?
        .create_if_missing(true);
    SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .map_err(|err| load_sql_error(path, err))
}

async fn ensure_schema(pool: &SqlitePool, path: &Path) -> Result<(), ApprovalStoreError> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS approval_requests (
            id TEXT PRIMARY KEY NOT NULL,
            workflow_id TEXT NOT NULL,
            step_id TEXT NOT NULL,
            capability_id TEXT NOT NULL,
            status TEXT NOT NULL,
            requested_at TEXT NOT NULL,
            expires_at TEXT,
            decided_at TEXT,
            payload_json TEXT NOT NULL
        )",
    )
    .execute(pool)
    .await
    .map_err(|err| persist_sql_error(path, err))?;
    sqlx::query(
        "CREATE INDEX IF NOT EXISTS approval_requests_status_idx
         ON approval_requests(status)",
    )
    .execute(pool)
    .await
    .map_err(|err| persist_sql_error(path, err))?;
    Ok(())
}

fn load_error(path: &Path, err: io::Error) -> ApprovalStoreError {
    ApprovalStoreError::Load {
        path: path.display().to_string(),
        message: err.to_string(),
    }
}

fn persist_error(path: &Path, err: io::Error) -> ApprovalStoreError {
    ApprovalStoreError::Persist {
        path: path.display().to_string(),
        message: err.to_string(),
    }
}

fn load_sql_error(path: &Path, err: sqlx::Error) -> ApprovalStoreError {
    ApprovalStoreError::Load {
        path: path.display().to_string(),
        message: err.to_string(),
    }
}

fn persist_sql_error(path: &Path, err: sqlx::Error) -> ApprovalStoreError {
    ApprovalStoreError::Persist {
        path: path.display().to_string(),
        message: err.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::super::approval_lifecycle::ApprovalAction;
    use super::super::approval_lifecycle::ApprovalRisk;
    use super::*;

    fn ts(seconds: i64) -> DateTime<Utc> {
        DateTime::from_timestamp(seconds, 0).expect("valid fixture timestamp")
    }

    fn request(id: ApprovalRequestId) -> ApprovalRequest {
        ApprovalRequest::pending(
            id,
            "maintenance.build-check",
            "validate",
            "vac.build",
            ApprovalAction::ProcessExecute,
            ApprovalRisk::Execute,
            vec!["policy requires approval for process_execute".to_string()],
            ts(1),
            Some(ts(61)),
        )
    }

    #[test]
    fn approval_file_store_persists_request_across_reopen() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("approvals.sqlite");
        let id = ApprovalRequestId::for_workflow_step_attempt(
            "maintenance.build-check",
            "run-a",
            "attempt-1",
            "validate",
        );

        let mut store = FileApprovalStore::open(&path).expect("open empty store");
        store.request(request(id.clone())).expect("persist request");
        assert_eq!(store.pending_count(), 1);

        let reopened = FileApprovalStore::open(&path).expect("reopen persisted store");
        let stored = reopened.get(&id).expect("persisted approval request");
        assert_eq!(stored.id, id);
        assert_eq!(stored.status.as_str(), "pending");
        assert_eq!(reopened.pending_count(), 1);
    }

    #[test]
    fn approval_file_store_resumes_pending_request_and_persisted_approval_after_reopen() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("approvals.sqlite");
        let id = ApprovalRequestId::for_workflow_step_attempt(
            "maintenance.build-check",
            "run-resume",
            "attempt-1",
            "validate",
        );

        {
            let mut store = FileApprovalStore::open(&path).expect("open empty store");
            let persisted_id = store.request(request(id.clone())).expect("persist request");
            assert_eq!(persisted_id, id);
            assert_eq!(store.pending_count(), 1);
        }

        {
            let mut resumed = FileApprovalStore::open(&path).expect("reopen pending store");
            let pending = resumed.get(&id).expect("pending request after restart");
            assert_eq!(pending.status.as_str(), "pending");
            assert_eq!(pending.decided_by, None);
            assert_eq!(pending.decided_at, None);
            assert_eq!(pending.decision_reason, None);
            assert_eq!(resumed.pending_count(), 1);

            let approved = resumed
                .resolve(
                    &id,
                    ApprovalDecision::approved("operator", "reviewed after restart", ts(10)),
                )
                .expect("persist approval decision");
            assert_eq!(approved.status.as_str(), "approved");
            assert_eq!(approved.decided_by.as_deref(), Some("operator"));
            assert_eq!(
                approved.decision_reason.as_deref(),
                Some("reviewed after restart")
            );
            assert_eq!(resumed.pending_count(), 0);
        }

        let resumed = FileApprovalStore::open(&path).expect("reopen approved store");
        let approved = resumed.get(&id).expect("approved request after restart");
        assert_eq!(approved.status.as_str(), "approved");
        assert_eq!(approved.decided_by.as_deref(), Some("operator"));
        assert_eq!(approved.decided_at, Some(ts(10)));
        assert_eq!(
            approved.decision_reason.as_deref(),
            Some("reviewed after restart")
        );
        assert_eq!(resumed.pending_count(), 0);
    }

    #[test]
    fn approval_file_store_exposes_background_ttl_sweeper_entrypoint() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("approvals.sqlite");
        let id = ApprovalRequestId::for_workflow_step_attempt(
            "maintenance.build-check",
            "run-sweeper",
            "attempt-1",
            "validate",
        );

        let mut store = FileApprovalStore::open(&path).expect("open empty store");
        store.request(request(id)).expect("persist request");
        let handle = FileApprovalStore::spawn_ttl_sweeper(
            path,
            Duration::from_millis(10),
            "approval request expired",
        );

        assert_eq!(handle.thread().name(), None);
    }

    #[test]
    fn approval_file_store_persists_resolution_and_ttl_sweep() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("approvals.sqlite");
        let id = ApprovalRequestId::for_workflow_step_attempt(
            "maintenance.build-check",
            "run-b",
            "attempt-2",
            "validate",
        );

        let mut store = FileApprovalStore::open(&path).expect("open empty store");
        store.request(request(id.clone())).expect("persist request");
        let expired = store.expire_due(ts(61), "ttl elapsed").expect("expire due");
        assert_eq!(expired.len(), 1);

        let reopened = FileApprovalStore::open(&path).expect("reopen persisted store");
        let stored = reopened.get(&id).expect("persisted approval request");
        assert_eq!(stored.status.as_str(), "expired");
        assert_eq!(stored.decision_reason.as_deref(), Some("ttl elapsed"));
        assert_eq!(reopened.pending_count(), 0);
    }

    #[test]
    fn approval_file_store_migrates_legacy_json_payload() {
        let tempdir = tempfile::tempdir().expect("tempdir");
        let path = tempdir.path().join("approvals.db");
        let id = ApprovalRequestId::for_workflow_step_attempt(
            "maintenance.build-check",
            "run-json",
            "attempt-1",
            "validate",
        );
        let mut requests = BTreeMap::new();
        requests.insert(id.clone(), request(id.clone()));
        fs::write(
            &path,
            serde_json::to_string_pretty(&requests).expect("json"),
        )
        .expect("write legacy json");

        let store = FileApprovalStore::open(&path).expect("migrate legacy json store");
        assert!(store.get(&id).is_some());
        let reopened = FileApprovalStore::open(&path).expect("reopen migrated sqlite store");
        assert!(reopened.get(&id).is_some());
    }
}
