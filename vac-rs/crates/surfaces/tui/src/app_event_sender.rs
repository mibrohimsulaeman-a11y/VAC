// Convenience sender for app events and common outbound TUI commands.
//
// This wraps the raw channel so call sites can submit typed `AppCommand`s
// without duplicating event construction or session logging behavior.

use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crate::app_command::AppCommand;
use crate::session_protocol::CommandExecutionApprovalDecision;
use crate::session_protocol::FileChangeApprovalDecision;
use crate::session_protocol::McpServerElicitationAction;
use crate::session_protocol::RequestId as AppServerRequestId;
use crate::session_protocol::ToolRequestUserInputResponse;
use tokio::sync::mpsc::Sender;
#[cfg(test)]
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::error::TrySendError;
use vac_protocol::ThreadId;
use vac_protocol::protocol::ReviewTarget;
use vac_protocol::request_permissions::RequestPermissionsResponse;

use crate::app_event::AppEvent;
use crate::session_log;

const APP_EVENT_QUEUE_CAPACITY: usize = 2048;
static APP_EVENT_QUEUE_FULL_DROPS: AtomicU64 = AtomicU64::new(0);
static APP_EVENT_QUEUE_CLOSED_SENDS: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug)]
pub(crate) enum AppEventChannelSender {
    Bounded(Sender<AppEvent>),
    #[cfg(test)]
    Unbounded(UnboundedSender<AppEvent>),
}

impl From<Sender<AppEvent>> for AppEventChannelSender {
    fn from(sender: Sender<AppEvent>) -> Self {
        Self::Bounded(sender)
    }
}

#[cfg(test)]
impl From<UnboundedSender<AppEvent>> for AppEventChannelSender {
    fn from(sender: UnboundedSender<AppEvent>) -> Self {
        Self::Unbounded(sender)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct AppEventSender {
    app_event_tx: AppEventChannelSender,
}

impl AppEventSender {
    pub(crate) const QUEUE_CAPACITY: usize = APP_EVENT_QUEUE_CAPACITY;

    pub(crate) fn new<T>(app_event_tx: T) -> Self
    where
        T: Into<AppEventChannelSender>,
    {
        Self {
            app_event_tx: app_event_tx.into(),
        }
    }

    /// Send an event to the app event channel. If the bounded production queue
    /// is full or closed, we swallow the error and log it instead of allowing
    /// app-event producers to grow memory without backpressure.
    pub(crate) fn send(&self, event: AppEvent) {
        // Record inbound events for high-fidelity session replay.
        // Avoid double-logging Ops; those are logged at the point of submission.
        if !matches!(event, AppEvent::VACOp(_)) {
            session_log::log_inbound_app_event(&event);
        }
        match &self.app_event_tx {
            AppEventChannelSender::Bounded(sender) => {
                if let Err(err) = sender.try_send(event) {
                    match err {
                        TrySendError::Full(_) => {
                            let dropped_total =
                                APP_EVENT_QUEUE_FULL_DROPS.fetch_add(1, Ordering::Relaxed) + 1;
                            tracing::warn!(
                                capacity = Self::QUEUE_CAPACITY,
                                dropped_total,
                                "dropping app event because the bounded TUI event queue is full"
                            );
                        }
                        TrySendError::Closed(_) => {
                            let closed_total =
                                APP_EVENT_QUEUE_CLOSED_SENDS.fetch_add(1, Ordering::Relaxed) + 1;
                            tracing::error!(
                                closed_total,
                                "failed to send event: bounded TUI event queue closed"
                            );
                        }
                    }
                }
            }
            #[cfg(test)]
            AppEventChannelSender::Unbounded(sender) => {
                if let Err(err) = sender.send(event) {
                    tracing::error!("failed to send event: {err}");
                }
            }
        }
    }

    pub(crate) fn interrupt(&self) {
        self.send(AppEvent::VACOp(AppCommand::interrupt()));
    }

    pub(crate) fn compact(&self) {
        self.send(AppEvent::VACOp(AppCommand::compact()));
    }

    pub(crate) fn set_thread_name(&self, name: String) {
        self.send(AppEvent::VACOp(AppCommand::set_thread_name(name)));
    }

    pub(crate) fn review(&self, target: ReviewTarget) {
        self.send(AppEvent::VACOp(AppCommand::review(target)));
    }

    pub(crate) fn list_skills(&self, cwds: Vec<PathBuf>, force_reload: bool) {
        self.send(AppEvent::VACOp(AppCommand::list_skills(cwds, force_reload)));
    }

    pub(crate) fn user_input_answer(&self, id: String, response: ToolRequestUserInputResponse) {
        self.send(AppEvent::VACOp(AppCommand::user_input_answer(id, response)));
    }

    pub(crate) fn exec_approval(
        &self,
        thread_id: ThreadId,
        id: String,
        decision: CommandExecutionApprovalDecision,
    ) {
        self.send(AppEvent::SubmitThreadOp {
            thread_id,
            op: AppCommand::exec_approval(id, /*turn_id*/ None, decision),
        });
    }

    pub(crate) fn request_permissions_response(
        &self,
        thread_id: ThreadId,
        id: String,
        response: RequestPermissionsResponse,
    ) {
        self.send(AppEvent::SubmitThreadOp {
            thread_id,
            op: AppCommand::request_permissions_response(id, response),
        });
    }

    pub(crate) fn patch_approval(
        &self,
        thread_id: ThreadId,
        id: String,
        decision: FileChangeApprovalDecision,
    ) {
        self.send(AppEvent::SubmitThreadOp {
            thread_id,
            op: AppCommand::patch_approval(id, decision),
        });
    }

    pub(crate) fn resolve_elicitation(
        &self,
        thread_id: ThreadId,
        server_name: String,
        request_id: AppServerRequestId,
        decision: McpServerElicitationAction,
        content: Option<serde_json::Value>,
        meta: Option<serde_json::Value>,
    ) {
        self.send(AppEvent::SubmitThreadOp {
            thread_id,
            op: AppCommand::resolve_elicitation(server_name, request_id, decision, content, meta),
        });
    }
}
