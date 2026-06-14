use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use tokio::sync::{broadcast, mpsc};
use tokio::time::{Duration, sleep};
use uuid::Uuid;
use vac_foundation::models::integrations::openai::{
    FunctionCall, ProgressType, ToolCall, ToolCallResult, ToolCallResultProgress,
    ToolCallResultStatus, ToolCallStreamInfo,
};
use vac_foundation::models::tools::ask_user::{AskUserOption, AskUserQuestion};
use vac_foundation::task_manager::TaskManager;
use vac_provider_core::Model;
use vac_tui::{InputEvent, OutputEvent};

const MATRIX_JSON: &str =
    include_str!("../../../../../tests/fixtures/tui-agent-tool-lifecycle/tool-matrix.json");

#[derive(Debug, Clone, Deserialize)]
struct ToolMatrix {
    prompt: String,
    tools: Vec<ToolSpec>,
    ask_user: AskUserSpec,
}

#[derive(Debug, Clone, Deserialize)]
struct ToolSpec {
    name: String,
    args: Value,
    result: String,
}

#[derive(Debug, Clone, Deserialize)]
struct AskUserSpec {
    name: String,
    args: Value,
    result_marker: String,
}

fn tool_call(index: usize, name: &str, args: &Value) -> ToolCall {
    ToolCall {
        id: Uuid::from_u128(index as u128 + 1).to_string(),
        r#type: "function".to_string(),
        function: FunctionCall {
            name: name.to_string(),
            arguments: serde_json::to_string(args).expect("tool args serialize"),
        },
        metadata: None,
    }
}

fn tool_result(call: ToolCall, result: String, status: ToolCallResultStatus) -> ToolCallResult {
    ToolCallResult {
        call,
        result,
        status,
    }
}

fn parse_ask_user_questions(spec: &AskUserSpec) -> Vec<AskUserQuestion> {
    let Some(items) = spec.args.get("questions").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .map(|item| AskUserQuestion {
            label: item
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or("Question")
                .to_string(),
            question: item
                .get("question")
                .and_then(Value::as_str)
                .unwrap_or("Continue?")
                .to_string(),
            options: item
                .get("options")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|option| AskUserOption {
                    value: option
                        .get("value")
                        .and_then(Value::as_str)
                        .unwrap_or("yes")
                        .to_string(),
                    label: option
                        .get("label")
                        .and_then(Value::as_str)
                        .unwrap_or("Yes")
                        .to_string(),
                    description: option
                        .get("description")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    selected: option
                        .get("selected")
                        .and_then(Value::as_bool)
                        .unwrap_or(false),
                })
                .collect(),
            allow_custom: item
                .get("allow_custom")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            multi_select: item
                .get("multi_select")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        })
        .collect()
}

async fn send_next_tool(
    backend_tx: &mpsc::Sender<InputEvent>,
    pending: &mut VecDeque<ToolCall>,
    ask_user_call: &ToolCall,
    ask_user_questions: &[AskUserQuestion],
) {
    if let Some(call) = pending.pop_front() {
        let _ = backend_tx
            .send(InputEvent::StreamToolCallProgress(vec![
                ToolCallStreamInfo {
                    name: call.function.name.clone(),
                    args_tokens: call.function.arguments.len().saturating_div(4),
                    description: Some(format!("VAC_AGENT_TOOL_PENDING {}", call.function.name)),
                },
            ]))
            .await;
        let _ = backend_tx
            .send(InputEvent::MessageToolCalls(vec![call.clone()]))
            .await;
        let _ = backend_tx.send(InputEvent::RunToolCall(call)).await;
    } else {
        let _ = backend_tx
            .send(InputEvent::ShowAskUserPopup(
                ask_user_call.clone(),
                ask_user_questions.to_vec(),
            ))
            .await;
        let ask_tx = backend_tx.clone();
        tokio::spawn(async move {
            // Keep Ask User deterministic in CI PTYs. The popup is still rendered
            // and processed through the real TUI handler path; these events replace
            // fragile second-Enter timing on slow runners.
            sleep(Duration::from_millis(700)).await;
            let _ = ask_tx.send(InputEvent::AskUserSelectOption).await;
            sleep(Duration::from_millis(150)).await;
            let _ = ask_tx.send(InputEvent::AskUserConfirmQuestion).await;
            sleep(Duration::from_millis(150)).await;
            let _ = ask_tx.send(InputEvent::AskUserSubmit).await;
        });
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let matrix: ToolMatrix = serde_json::from_str(MATRIX_JSON).expect("tool matrix fixture parses");
    let (backend_tx, backend_rx) = mpsc::channel::<InputEvent>(200);
    let (output_tx, mut output_rx) = mpsc::channel::<OutputEvent>(200);
    let (shutdown_tx, _shutdown_rx) = broadcast::channel::<()>(1);

    let task_manager = TaskManager::new();
    let task_manager_handle = task_manager.handle();
    let task_manager_task = tokio::spawn(async move {
        task_manager.run().await;
    });

    let tool_calls = matrix
        .tools
        .iter()
        .enumerate()
        .map(|(index, spec)| tool_call(index, &spec.name, &spec.args))
        .collect::<Vec<_>>();
    let result_by_id = matrix
        .tools
        .iter()
        .zip(tool_calls.iter())
        .map(|(spec, call)| (call.id.clone(), spec.result.clone()))
        .collect::<HashMap<_, _>>();
    let ask_user_call = tool_call(10_000, &matrix.ask_user.name, &matrix.ask_user.args);
    let ask_user_questions = parse_ask_user_questions(&matrix.ask_user);
    let ask_user_result_marker = matrix.ask_user.result_marker.clone();
    let fixture_prompt = matrix.prompt.clone();
    let fixture_prompt_for_output = fixture_prompt.clone();

    let backend_tx_for_output = backend_tx.clone();
    let output_task = tokio::spawn(async move {
        let mut pending = VecDeque::from(tool_calls);
        let mut started = false;
        while let Some(event) = output_rx.recv().await {
            match event {
                OutputEvent::UserMessage(text, _, _, _) => {
                    let _ = backend_tx_for_output
                        .send(InputEvent::AddUserMessage(text.clone()))
                        .await;
                    if started || text.trim().is_empty() {
                        continue;
                    }
                    started = true;
                    let _ = backend_tx_for_output
                        .send(InputEvent::AssistantMessage(format!(
                            "VAC_AGENT_TOOL_SMOKE_STARTED deterministic agent stream prompt={fixture_prompt_for_output}"
                        )))
                        .await;
                    send_next_tool(
                        &backend_tx_for_output,
                        &mut pending,
                        &ask_user_call,
                        &ask_user_questions,
                    )
                    .await;
                }
                OutputEvent::AcceptTool(call) => {
                    let marker = format!("VAC_AGENT_TOOL_PROGRESS {}", call.function.name);
                    let _ = backend_tx_for_output
                        .send(InputEvent::StreamToolResult(ToolCallResultProgress {
                            id: Uuid::new_v4(),
                            message: marker,
                            progress_type: Some(ProgressType::CommandOutput),
                            task_updates: None,
                            progress: Some(100.0),
                        }))
                        .await;
                    sleep(Duration::from_millis(40)).await;
                    let result = result_by_id.get(&call.id).cloned().unwrap_or_else(|| {
                        format!("VAC_AGENT_TOOL_RESULT {} default", call.function.name)
                    });
                    let _ = backend_tx_for_output
                        .send(InputEvent::ToolResult(tool_result(
                            call.clone(),
                            result,
                            ToolCallResultStatus::Success,
                        )))
                        .await;
                    let _ = backend_tx_for_output
                        .send(InputEvent::AssistantMessage(format!(
                            "VAC_AGENT_TOOL_COMPLETED {}",
                            call.function.name
                        )))
                        .await;
                    sleep(Duration::from_millis(40)).await;
                    send_next_tool(
                        &backend_tx_for_output,
                        &mut pending,
                        &ask_user_call,
                        &ask_user_questions,
                    )
                    .await;
                }
                OutputEvent::RejectTool(call, _) => {
                    let _ = backend_tx_for_output
                        .send(InputEvent::ToolResult(tool_result(
                            call.clone(),
                            format!("VAC_AGENT_TOOL_REJECTED {}", call.function.name),
                            ToolCallResultStatus::Error,
                        )))
                        .await;
                }
                OutputEvent::AskUserResponse(response) => {
                    let result = format!("{} {}", ask_user_result_marker, response.result);
                    let _ = backend_tx_for_output
                        .send(InputEvent::ToolResult(tool_result(
                            response.call.clone(),
                            result,
                            response.status,
                        )))
                        .await;
                    let _ = backend_tx_for_output
                        .send(InputEvent::AssistantMessage(
                            "VAC_AGENT_TOOL_SMOKE_DONE all tool lifecycle markers observed"
                                .to_string(),
                        ))
                        .await;
                    sleep(Duration::from_millis(250)).await;
                    let _ = backend_tx_for_output.send(InputEvent::Quit).await;
                    break;
                }
                _ => {}
            }
        }
    });

    let result = vac_tui::run_tui(
        backend_rx,
        output_tx,
        None,
        shutdown_tx,
        None,
        true,
        false,
        true,
        None,
        None,
        "default".to_string(),
        None,
        Model::custom("agent-tool-smoke-model", "smoke"),
        None,
        (None, None, None),
        Some(fixture_prompt),
        true,
        Vec::new(),
        None,
        task_manager_handle,
    )
    .await;

    output_task.abort();
    task_manager_task.abort();
    if let Err(error) = result {
        eprintln!("VAC TUI agent tool smoke harness failed: {error}");
        std::process::exit(1);
    }
    std::process::exit(0);
}
