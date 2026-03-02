# HRBand-LSL Examples

This directory contains example implementations and usage patterns for the HRBand-LSL Rust library.

## with_logging_hook

A complete example demonstrating the hook system by implementing a `LoggingHook` that logs all lifecycle events.

### Building and Running

```bash
cargo build --example with_logging_hook
cargo run --example with_logging_hook
```

### What it Does

The LoggingHook logs messages at each stage of the application lifecycle:

- **PreScan**: Logs when BLE device scanning starts
- **PostScan**: Logs the number of devices found
- **PreConnect**: Logs device connection attempt
- **PostConnect**: Logs successful connection
- **DataReceived**: Logs heart rate and RR interval values
- **PreStream**: Logs when data streaming begins
- **PostStream**: Logs when data streaming ends
- **PreDisconnect**: Logs device disconnection

### Custom Hooks

To implement your own hook, create a struct that implements the `Hook` trait:

```rust
use hrband_lsl::hooks::{Hook, HookContext, HookPoint};

struct MyHook;

impl Hook for MyHook {
    fn execute(&self, point: HookPoint, context: &HookContext) -> anyhow::Result<()> {
        // Your custom logic here
        match point {
            HookPoint::PostScan => {
                // Handle post-scan event
            }
            // ... handle other hook points
            _ => {}
        }
        Ok(())
    }
}
```

## Integration with InfluxDB

A common use case for hooks is sending metrics to a time-series database like InfluxDB:

```rust
struct InfluxDBHook {
    client: InfluxClient,
}

impl Hook for InfluxDBHook {
    fn execute(&self, point: HookPoint, context: &HookContext) -> anyhow::Result<()> {
        match point {
            HookPoint::DataReceived => {
                if let Some(hr) = context.heart_rate {
                    self.client.write_heart_rate(
                        context.device_name.as_deref().unwrap_or("unknown"),
                        hr as u16,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
```

## Hook Context

The `HookContext` provides information about the current state:

- `device_name`: Optional device name
- `device_address`: Optional BLE device address
- `heart_rate`: Current heart rate value (if available)
- `rr_intervals`: Vector of RR interval values in milliseconds
- `metadata`: HashMap for arbitrary key-value pairs

## Hook Points

| Hook Point | Description | Context Data |
|------------|-------------|--------------|
| PreScan | Before BLE scanning | None |
| PostScan | After BLE scanning completes | device_count metadata |
| PreConnect | Before device connection | device_name, device_address |
| PostConnect | After successful connection | device_name, device_address |
| DataReceived | When HR data is received | heart_rate, rr_intervals |
| PreStream | Before LSL streaming | device_name |
| PostStream | After LSL streaming | device_name |
| PreDisconnect | Before device disconnect | device_name |

## Multiple Hooks

You can register multiple hooks to execute sequentially:

```rust
let app = Application::new();
app.register_hook(Arc::new(LoggingHook));
app.register_hook(Arc::new(MetricsHook));
app.register_hook(Arc::new(InfluxDBHook));

app.run().await?;
```

Hooks execute in the order they were registered.
