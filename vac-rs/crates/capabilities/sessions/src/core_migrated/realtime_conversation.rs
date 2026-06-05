use std::sync::Arc;

use tokio::sync::RwLock;
use vac_protocol::error::Result as VACResult;
use vac_protocol::error::VACErr;
use vac_protocol::protocol::ConversationAudioParams;
use vac_protocol::protocol::ConversationStartParams;
use vac_protocol::protocol::ConversationTextParams;
use vac_protocol::protocol::ErrorEvent;
use vac_protocol::protocol::Event;
use vac_protocol::protocol::EventMsg;
use vac_protocol::protocol::VACErrorInfo;

use crate::session::session::Session;

pub(crate) const REALTIME_USER_TEXT_PREFIX: &str = "[USER] ";

#[derive(Debug, Default)]
pub(crate) struct RealtimeConversationManager {
    active: RwLock<bool>,
    active_handoff: RwLock<Option<String>>,
}

impl RealtimeConversationManager {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) async fn running_state(&self) -> Option<()> {
        self.active.read().await.then_some(())
    }

    pub(crate) async fn is_running_v2(&self) -> bool {
        false
    }

    pub(crate) async fn text_in(&self, _text: String) -> anyhow::Result<()> {
        anyhow::bail!("realtime conversation transport is not available")
    }

    pub(crate) async fn handoff_out(&self, _output_text: String) -> anyhow::Result<()> {
        Ok(())
    }

    pub(crate) async fn handoff_complete(&self) -> anyhow::Result<()> {
        anyhow::bail!("realtime conversation transport is not available")
    }

    pub(crate) async fn clear_active_handoff(&self) {
        *self.active_handoff.write().await = None;
    }

    pub(crate) async fn active_handoff_id(&self) -> Option<String> {
        self.active_handoff.read().await.clone()
    }

    pub(crate) async fn shutdown(&self) -> anyhow::Result<()> {
        *self.active.write().await = false;
        self.clear_active_handoff().await;
        Ok(())
    }
}

pub(crate) async fn handle_start(
    sess: &Arc<Session>,
    sub_id: String,
    _params: ConversationStartParams,
) -> VACResult<()> {
    {
        let mut active = sess.conversation.active.write().await;
        *active = false;
    }
    emit_unavailable(sess, sub_id).await;
    Err(VACErr::InvalidRequest(
        "realtime conversation transport is not available".to_string(),
    ))
}

pub(crate) async fn handle_audio(
    sess: &Arc<Session>,
    sub_id: String,
    _params: ConversationAudioParams,
) {
    emit_unavailable(sess, sub_id).await;
}

pub(crate) async fn handle_text(
    sess: &Arc<Session>,
    sub_id: String,
    _params: ConversationTextParams,
) {
    emit_unavailable(sess, sub_id).await;
}

pub(crate) async fn handle_close(sess: &Arc<Session>, sub_id: String) {
    {
        let mut active = sess.conversation.active.write().await;
        *active = false;
    }
    emit_unavailable(sess, sub_id).await;
}

pub(crate) fn prefix_realtime_v2_text(text: String, prefix: &str) -> String {
    if text.is_empty() || text.starts_with(prefix) {
        text
    } else {
        format!("{prefix}{text}")
    }
}

async fn emit_unavailable(sess: &Session, sub_id: String) {
    sess.send_event_raw(Event {
        id: sub_id,
        msg: EventMsg::Error(ErrorEvent {
            message: "realtime conversation transport is not available".to_string(),
            vac_error_info: Some(VACErrorInfo::Other),
        }),
    })
    .await;
}
