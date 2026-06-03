use super::App;
use crate::app_event::AppEvent;
use crate::app_event::ThreadGoalSetMode;
use crate::bottom_pane::SelectionAction;
use crate::bottom_pane::SelectionItem;
use crate::bottom_pane::SelectionViewParams;
use crate::bottom_pane::popup_consts::standard_popup_hint_line;
use crate::goal_display::goal_status_label;
use crate::goal_display::goal_usage_summary;
use crate::session_protocol::ThreadGoalStatus;
use crate::session_protocol::thread_goal_from_app_server;
use vac_protocol::ThreadId;

impl App {
    pub(super) async fn open_thread_goal_menu<S>(&mut self, app_server: &mut S, thread_id: ThreadId)
    where
        S: crate::local_runtime_session::LocalRuntimeSession,
    {
        let result = app_server.thread_goal_get(thread_id).await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        let response = match result {
            Ok(response) => response,
            Err(err) => {
                self.chat_widget
                    .add_error_message(format!("Failed to read thread goal: {err}"));
                return;
            }
        };

        let Some(goal) = response.goal.map(thread_goal_from_app_server) else {
            self.chat_widget.add_info_message(
                "Usage: /goal <objective>".to_string(),
                Some("No goal is currently set.".to_string()),
            );
            return;
        };

        self.chat_widget.show_goal_summary(goal);
    }

    pub(super) async fn set_thread_goal_objective<S>(
        &mut self,
        app_server: &mut S,
        thread_id: ThreadId,
        objective: String,
        mode: ThreadGoalSetMode,
    ) where
        S: crate::local_runtime_session::LocalRuntimeSession,
    {
        if mode == ThreadGoalSetMode::ConfirmIfExists {
            let result = app_server.thread_goal_get(thread_id).await;
            if self.current_displayed_thread_id() != Some(thread_id) {
                return;
            }

            match result {
                Ok(response) if response.goal.is_some() => {
                    self.show_replace_thread_goal_confirmation(thread_id, objective);
                    return;
                }
                Ok(_) => {}
                Err(err) => {
                    self.chat_widget
                        .add_error_message(format!("Failed to read thread goal: {err}"));
                    return;
                }
            }
        }

        let result = app_server
            .thread_goal_set(
                thread_id,
                Some(objective),
                Some(ThreadGoalStatus::Active),
                /*token_budget*/ None,
            )
            .await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        match result {
            Ok(response) => {
                let goal = thread_goal_from_app_server(response.goal);
                self.chat_widget.add_info_message(
                    format!("Goal {}", goal_status_label(goal.status)),
                    Some(goal_usage_summary(&goal)),
                );
            }
            Err(err) => self
                .chat_widget
                .add_error_message(format!("Failed to set thread goal: {err}")),
        }
    }

    pub(super) async fn set_thread_goal_status<S>(
        &mut self,
        app_server: &mut S,
        thread_id: ThreadId,
        status: ThreadGoalStatus,
    ) where
        S: crate::local_runtime_session::LocalRuntimeSession,
    {
        let result = app_server
            .thread_goal_set(
                thread_id,
                /*objective*/ None,
                Some(status),
                /*token_budget*/ None,
            )
            .await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        match result {
            Ok(response) => {
                let goal = thread_goal_from_app_server(response.goal);
                self.chat_widget.add_info_message(
                    format!("Goal {}", goal_status_label(goal.status)),
                    Some(goal_usage_summary(&goal)),
                );
            }
            Err(err) => self
                .chat_widget
                .add_error_message(format!("Failed to update thread goal: {err}")),
        }
    }

    pub(super) async fn clear_thread_goal<S>(&mut self, app_server: &mut S, thread_id: ThreadId)
    where
        S: crate::local_runtime_session::LocalRuntimeSession,
    {
        let result = app_server.thread_goal_clear(thread_id).await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        match result {
            Ok(response) => {
                if response.cleared {
                    self.chat_widget
                        .add_info_message("Goal cleared".to_string(), /*hint*/ None);
                } else {
                    self.chat_widget.add_info_message(
                        "No goal to clear".to_string(),
                        Some("This thread does not currently have a goal.".to_string()),
                    );
                }
            }
            Err(err) => self
                .chat_widget
                .add_error_message(format!("Failed to clear thread goal: {err}")),
        }
    }

    fn show_replace_thread_goal_confirmation(&mut self, thread_id: ThreadId, objective: String) {
        let replace_objective = objective.clone();
        let replace_actions: Vec<SelectionAction> = vec![Box::new(move |tx| {
            tx.send(AppEvent::SetThreadGoalObjective {
                thread_id,
                objective: replace_objective.clone(),
                mode: ThreadGoalSetMode::ReplaceExisting,
            });
        })];
        let items = vec![
            SelectionItem {
                name: "Replace current goal".to_string(),
                description: Some("Set the new objective and start it now".to_string()),
                actions: replace_actions,
                dismiss_on_select: true,
                ..Default::default()
            },
            SelectionItem {
                name: "Cancel".to_string(),
                description: Some("Keep the current goal".to_string()),
                dismiss_on_select: true,
                ..Default::default()
            },
        ];
        self.chat_widget.show_selection_view(SelectionViewParams {
            title: Some("Replace goal?".to_string()),
            subtitle: Some(format!("New objective: {objective}")),
            footer_hint: Some(standard_popup_hint_line()),
            items,
            ..Default::default()
        });
    }
}
