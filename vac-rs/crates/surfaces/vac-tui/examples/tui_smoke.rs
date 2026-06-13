use tokio::sync::{broadcast, mpsc};
use vac_foundation::task_manager::TaskManager;
use vac_provider_core::Model;
use vac_tui::{InputEvent, OutputEvent};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let (backend_tx, backend_rx) = mpsc::channel::<InputEvent>(100);
    let (output_tx, mut output_rx) = mpsc::channel::<OutputEvent>(100);
    let (shutdown_tx, _shutdown_rx) = broadcast::channel::<()>(1);

    let task_manager = TaskManager::new();
    let task_manager_handle = task_manager.handle();
    let task_manager_task = tokio::spawn(async move {
        task_manager.run().await;
    });

    let backend_tx_for_output = backend_tx.clone();
    let output_task = tokio::spawn(async move {
        while let Some(event) = output_rx.recv().await {
            match event {
                OutputEvent::PlanModeActivated(_) => {
                    let _ = backend_tx_for_output
                        .send(InputEvent::PlanModeChanged(true))
                        .await;
                }
                OutputEvent::UserMessage(text, _, _, _) => {
                    let _ = backend_tx_for_output
                        .send(InputEvent::AddUserMessage(text))
                        .await;
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
        Model::custom("smoke-model", "smoke"),
        None,
        (None, None, None),
        None,
        false,
        Vec::new(),
        None,
        task_manager_handle,
    )
    .await;

    output_task.abort();
    task_manager_task.abort();
    result
}
