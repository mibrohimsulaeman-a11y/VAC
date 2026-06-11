//! Example: Streaming with Anthropic

use futures::StreamExt;
use vac_provider_core::{GenerateRequest, Inference, Message, Model, Role, StreamEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Requires ANTHROPIC_API_KEY environment variable

    let client = Inference::new();

    let mut request = GenerateRequest::new(
        Model::custom("claude-3-5-sonnet-20241022", "anthropic"),
        vec![Message::new(
            Role::User,
            "Write a short poem about Rust programming.",
        )],
    );
    request.options.temperature = Some(0.8);
    request.options.max_tokens = Some(300);

    println!("🤖 Streaming with Claude...\n");

    let mut stream = client.stream(&request).await?;

    while let Some(event) = stream.next().await {
        match event? {
            StreamEvent::TextDelta { delta, .. } => {
                print!("{}", delta);
                std::io::Write::flush(&mut std::io::stdout())?;
            }
            StreamEvent::Finish { usage, reason } => {
                println!("\n\n✓ Done!");
                println!("Usage: {:?}", usage);
                println!("Finish reason: {:?}", reason);
            }
            StreamEvent::Error { message } => {
                eprintln!("\n✗ Error: {}", message);
            }
            _ => {}
        }
    }

    Ok(())
}
