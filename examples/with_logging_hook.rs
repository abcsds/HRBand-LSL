use hrband_lsl::hooks::{Hook, HookContext, HookPoint};
use std::sync::Arc;

/// LoggingHook logs all lifecycle events as they occur
struct LoggingHook;

impl Hook for LoggingHook {
    fn execute(&self, point: HookPoint, context: &HookContext) -> anyhow::Result<()> {
        match point {
            HookPoint::PreScan => {
                println!("[HOOK] PreScan: Starting BLE device scan...");
            }
            HookPoint::PostScan => {
                let count = context
                    .metadata
                    .get("device_count")
                    .map(|s| s.as_str())
                    .unwrap_or("0");
                println!("[HOOK] PostScan: Found {} devices", count);
            }
            HookPoint::PreConnect => {
                println!(
                    "[HOOK] PreConnect: Connecting to device '{}'",
                    context.device_name.as_deref().unwrap_or("Unknown")
                );
            }
            HookPoint::PostConnect => {
                println!(
                    "[HOOK] PostConnect: Successfully connected to '{}'",
                    context.device_name.as_deref().unwrap_or("Unknown")
                );
            }
            HookPoint::DataReceived => {
                if let Some(hr) = context.heart_rate {
                    println!("[HOOK] DataReceived: Heart rate = {} bpm", hr);
                }
                if !context.rr_intervals.is_empty() {
                    println!(
                        "[HOOK] DataReceived: RR intervals = {:?} ms",
                        context.rr_intervals
                    );
                }
            }
            HookPoint::PreStream => {
                println!(
                    "[HOOK] PreStream: Starting data stream from '{}'",
                    context.device_name.as_deref().unwrap_or("Unknown")
                );
            }
            HookPoint::PostStream => {
                println!("[HOOK] PostStream: Data stream ended");
            }
            HookPoint::PreDisconnect => {
                println!("[HOOK] PreDisconnect: Disconnecting from device");
            }
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("HRBand-LSL with Logging Hook Example");

    let logging_hook = Arc::new(LoggingHook);

    let mut app = hrband_lsl::Application::new();
    app.register_hook(logging_hook);

    println!("Starting application with logging hook...\n");
    app.run().await?;

    println!("\nApplication completed");
    Ok(())
}
