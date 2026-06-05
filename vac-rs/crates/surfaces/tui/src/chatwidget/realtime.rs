// Local-tool stub for removed realtime voice/WebRTC chat-widget surface.

use super::*;
use crate::app_event::{RealtimeAudioDeviceKind, RealtimeWebrtcEvent};
use crate::session_protocol::{
    ThreadRealtimeAudioChunk, ThreadRealtimeClosedNotification, ThreadRealtimeErrorNotification,
    ThreadRealtimeItemAddedNotification, ThreadRealtimeOutputAudioDeltaNotification,
    ThreadRealtimeStartedNotification,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum RealtimeConversationPhase {
    #[default]
    Inactive,
    Starting,
    Active,
    Stopping,
}

#[derive(Default)]
pub(super) struct RealtimeConversationUiState {
    pub(super) phase: RealtimeConversationPhase,
    pub(super) meter_placeholder_id: Option<String>,
}

impl RealtimeConversationUiState {
    pub(super) fn is_live(&self) -> bool {
        !matches!(self.phase, RealtimeConversationPhase::Inactive)
    }
    pub(super) fn is_active(&self) -> bool {
        matches!(self.phase, RealtimeConversationPhase::Active)
    }
}

impl ChatWidget {
    pub(super) fn stop_realtime_conversation_from_ui(&mut self) {
        self.realtime_conversation.phase = RealtimeConversationPhase::Stopping;
        if let Some(id) = self.realtime_conversation.meter_placeholder_id.take() {
            self.remove_recording_meter_placeholder(&id);
        }
        self.submit_op(AppCommand::realtime_conversation_close());
    }

    pub(crate) fn stop_realtime_conversation_for_deleted_meter(&mut self, id: &str) -> bool {
        if self.realtime_conversation.meter_placeholder_id.as_deref() == Some(id) {
            self.realtime_conversation.meter_placeholder_id = None;
            self.realtime_conversation.phase = RealtimeConversationPhase::Stopping;
            self.submit_op(AppCommand::realtime_conversation_close());
            return true;
        }
        false
    }

    pub(super) fn start_realtime_conversation(&mut self) {
        self.reset_realtime_conversation_state();
        self.add_info_message(
            "Realtime voice/WebRTC was removed from this local coding tool build.".to_string(),
            Some("Use text turns, shell tools, MCP tools, or @mentions instead.".to_string()),
        );
    }

    pub(super) fn request_realtime_conversation_close(&mut self, info_message: Option<String>) {
        self.reset_realtime_conversation_state();
        if let Some(message) = info_message { self.add_info_message(message, None); }
    }

    pub(super) fn reset_realtime_conversation_state(&mut self) {
        if let Some(id) = self.realtime_conversation.meter_placeholder_id.take() {
            self.remove_recording_meter_placeholder(&id);
        }
        self.realtime_conversation.phase = RealtimeConversationPhase::Inactive;
    }

    pub(super) fn on_realtime_conversation_started(&mut self, _notification: ThreadRealtimeStartedNotification) {
        self.start_realtime_conversation();
    }

    pub(super) fn on_realtime_output_audio_delta(&mut self, _notification: ThreadRealtimeOutputAudioDeltaNotification) {}
    pub(super) fn on_realtime_item_added(&mut self, _notification: ThreadRealtimeItemAddedNotification) {}

    pub(super) fn on_realtime_error(&mut self, notification: ThreadRealtimeErrorNotification) {
        self.realtime_conversation.phase = RealtimeConversationPhase::Stopping;
        if let Some(id) = self.realtime_conversation.meter_placeholder_id.take() {
            self.remove_recording_meter_placeholder(&id);
        }
        self.add_error_message(format!("Realtime voice error: {}", notification.message));
        self.submit_op(AppCommand::realtime_conversation_close());
    }

    pub(super) fn on_realtime_conversation_closed(&mut self, _notification: ThreadRealtimeClosedNotification) {
        self.reset_realtime_conversation_state();
    }

    pub(super) fn on_realtime_conversation_sdp(&mut self, _sdp: String) {}

    pub(crate) fn on_realtime_webrtc_offer_created(&mut self, _result: Result<crate::app_event::RealtimeWebrtcOffer, String>) {
        self.reset_realtime_conversation_state();
    }

    pub(crate) fn on_realtime_webrtc_event(&mut self, _event: RealtimeWebrtcEvent) {
        self.reset_realtime_conversation_state();
    }

    pub(crate) fn on_realtime_webrtc_local_audio_level(&mut self, _peak: u16) {}

    fn enqueue_realtime_audio_out(&mut self, _frame: &ThreadRealtimeAudioChunk) {}
    fn interrupt_realtime_audio_playback(&mut self) {}

    pub(crate) fn restart_realtime_audio_device(&mut self, _kind: RealtimeAudioDeviceKind) {
        self.add_info_message(
            "Realtime audio device selection was removed from this local coding tool build.".to_string(),
            None,
        );
    }
}
