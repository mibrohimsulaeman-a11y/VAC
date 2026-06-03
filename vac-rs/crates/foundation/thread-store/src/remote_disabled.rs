//! Local-only fail-closed replacement for the removed remote thread-store RPC client.

use std::any::Any;

use async_trait::async_trait;
use vac_protocol::ThreadId;

use crate::AppendThreadItemsParams;
use crate::ArchiveThreadParams;
use crate::CreateThreadParams;
use crate::ListThreadsParams;
use crate::LoadThreadHistoryParams;
use crate::ReadThreadByRolloutPathParams;
use crate::ReadThreadParams;
use crate::ResumeThreadParams;
use crate::StoredThread;
use crate::StoredThreadHistory;
use crate::ThreadPage;
use crate::ThreadStore;
use crate::ThreadStoreError;
use crate::ThreadStoreResult;
use crate::UpdateThreadMetadataParams;

#[derive(Debug, Clone)]
pub struct RemoteThreadStore;

impl RemoteThreadStore {
    pub fn unavailable() -> Self {
        Self
    }

    fn unavailable_error() -> ThreadStoreError {
        ThreadStoreError::InvalidRequest {
            message: "remote thread-store is unavailable in the default local build".to_string(),
        }
    }
}

#[async_trait]
impl ThreadStore for RemoteThreadStore {
    fn as_any(&self) -> &dyn Any {
        self
    }

    async fn create_thread(&self, _params: CreateThreadParams) -> ThreadStoreResult<()> {
        Err(Self::unavailable_error())
    }

    async fn resume_thread(&self, _params: ResumeThreadParams) -> ThreadStoreResult<()> {
        Err(Self::unavailable_error())
    }

    async fn append_items(&self, _params: AppendThreadItemsParams) -> ThreadStoreResult<()> {
        Err(Self::unavailable_error())
    }

    async fn persist_thread(&self, _thread_id: ThreadId) -> ThreadStoreResult<()> {
        Err(Self::unavailable_error())
    }

    async fn flush_thread(&self, _thread_id: ThreadId) -> ThreadStoreResult<()> {
        Err(Self::unavailable_error())
    }

    async fn shutdown_thread(&self, _thread_id: ThreadId) -> ThreadStoreResult<()> {
        Err(Self::unavailable_error())
    }

    async fn discard_thread(&self, _thread_id: ThreadId) -> ThreadStoreResult<()> {
        Err(Self::unavailable_error())
    }

    async fn load_history(
        &self,
        _params: LoadThreadHistoryParams,
    ) -> ThreadStoreResult<StoredThreadHistory> {
        Err(Self::unavailable_error())
    }

    async fn read_thread(&self, _params: ReadThreadParams) -> ThreadStoreResult<StoredThread> {
        Err(Self::unavailable_error())
    }

    async fn read_thread_by_rollout_path(
        &self,
        _params: ReadThreadByRolloutPathParams,
    ) -> ThreadStoreResult<StoredThread> {
        Err(Self::unavailable_error())
    }

    async fn list_threads(&self, _params: ListThreadsParams) -> ThreadStoreResult<ThreadPage> {
        Err(Self::unavailable_error())
    }

    async fn update_thread_metadata(
        &self,
        _params: UpdateThreadMetadataParams,
    ) -> ThreadStoreResult<StoredThread> {
        Err(Self::unavailable_error())
    }

    async fn archive_thread(&self, _params: ArchiveThreadParams) -> ThreadStoreResult<()> {
        Err(Self::unavailable_error())
    }

    async fn unarchive_thread(
        &self,
        _params: ArchiveThreadParams,
    ) -> ThreadStoreResult<StoredThread> {
        Err(Self::unavailable_error())
    }
}
