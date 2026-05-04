pub mod ble;
pub mod client;
pub mod heart_rate;
pub mod hooks;
#[cfg(feature = "lsl")]
pub mod lsl_ffi;
pub mod lsl_stream;
pub mod polar_pmd;

use anyhow::{Context, Result};
use ble::BleDeviceManager;
use btleplug::api::Peripheral as _;
use hooks::{HookContext, HookPoint, HookRegistry};
use inquire::Select;
use lsl_stream::LslStreamManager;
use std::sync::Arc;
use std::time::Duration;

/// Application struct managing the BLE-LSL integration
pub struct Application {
    hook_registry: HookRegistry,
}

impl Application {
    /// Create a new Application with an empty hook registry
    pub fn new() -> Self {
        Application {
            hook_registry: HookRegistry::new(),
        }
    }

    /// Create a new Application with a pre-configured hook registry
    pub fn with_hooks(hook_registry: HookRegistry) -> Self {
        Application { hook_registry }
    }

    /// Register a hook with the application
    pub fn register_hook(&mut self, hook: Arc<dyn hooks::Hook>) {
        self.hook_registry.register(hook);
    }

    /// Cheap clone of the hook registry for sharing into async closures.
    /// The trait objects themselves stay behind their `Arc`s — this just
    /// duplicates the (small) `Vec` of pointers.
    fn hook_registry_arc(&self) -> HookRegistry {
        self.hook_registry.clone()
    }

    /// Run the application: scan, filter, select, connect, and stream
    pub async fn run(&self) -> Result<()> {
        // Pre-scan hook
        self.hook_registry
            .execute(HookPoint::PreScan, &HookContext::default())?;

        // Create BLE manager
        let ble_manager = BleDeviceManager::new()
            .await
            .context("Failed to create BLE device manager")?;

        // Scan for devices
        println!("Scanning for BLE devices (5 seconds)...");
        let devices = ble_manager
            .scan_devices(Duration::from_secs(5))
            .await
            .context("Failed to scan BLE devices")?;
        println!("Found {} raw devices", devices.len());

        // Filter devices
        let filtered_devices = ble_manager
            .filter_devices(devices)
            .await
            .context("Failed to filter devices")?;
        println!("After filtering: {} devices", filtered_devices.len());

        // Post-scan hook
        let mut scan_context = HookContext::default();
        scan_context.metadata.insert(
            "device_count".to_string(),
            filtered_devices.len().to_string(),
        );
        self.hook_registry
            .execute(HookPoint::PostScan, &scan_context)?;

        // Handle no devices case with retry loop
        let selected_device = self
            .select_device(&ble_manager, &filtered_devices)
            .await
            .context("Failed to select device")?;

        let device = selected_device.context("No device selected")?;

        // Pre-connect hook
        let mut connect_context = HookContext::default();
        if let Ok(Some(props)) = device.properties().await {
            connect_context.device_name = props.local_name.clone();
            connect_context.device_address = Some(props.address.to_string());
        }
        self.hook_registry
            .execute(HookPoint::PreConnect, &connect_context)?;

        // Connect to device and stream
        self.connect_and_stream(device.clone(), &connect_context)
            .await?;

        // Post-connect hook
        self.hook_registry
            .execute(HookPoint::PostConnect, &connect_context)?;

        Ok(())
    }

    /// Select a device from available options with retry support
    async fn select_device(
        &self,
        ble_manager: &BleDeviceManager,
        initial_devices: &[btleplug::platform::Peripheral],
    ) -> Result<Option<btleplug::platform::Peripheral>> {
        let mut devices = initial_devices.to_vec();

        loop {
            if devices.is_empty() {
                println!("No devices found. Please check your BLE device.");
                let ans = inquire::Confirm::new("Retry scanning?").prompt()?;

                if !ans {
                    return Ok(None);
                }

                // Rescan
                let rescanned = ble_manager.scan_devices(Duration::from_secs(5)).await?;
                devices = ble_manager.filter_devices(rescanned).await?;
                continue;
            }

            // Collect device names
            let mut device_options = Vec::new();
            for device in &devices {
                if let Ok(Some(props)) = device.properties().await {
                    if let Some(name) = props.local_name {
                        device_options.push(name);
                    }
                }
            }

            if device_options.is_empty() {
                return Ok(None);
            }

            // Auto-select a Polar band when exactly one is in range (covers
            // the "I'm wearing the H10/Sense and want to debug" common case).
            // This lets the app run non-interactively from the Bash test loop;
            // when multiple devices match, fall through to the inquire prompt.
            let polar_matches: Vec<&String> = device_options
                .iter()
                .filter(|d| d.contains("Polar"))
                .collect();
            let selected_name = if polar_matches.len() == 1 {
                let name = polar_matches[0].clone();
                println!("Auto-selecting Polar device: {name}");
                name
            } else {
                match Select::new("Select device:", device_options).prompt() {
                    Ok(name) => name,
                    Err(_) => {
                        return Ok(None);
                    }
                }
            };

            for device in devices.iter() {
                if let Ok(Some(props)) = device.properties().await {
                    if let Some(name) = &props.local_name {
                        if name == &selected_name {
                            return Ok(Some(device.clone()));
                        }
                    }
                }
            }
        }
    }

    /// Connect to a device and stream heart rate data. Delegates the GATT
    /// state machine (protocol selection, PMD/PPI subscription chain, STOP_PPI
    /// on close) to [`client::HeartRateClient`]. The closures bridge events
    /// from the client into LSL outlets and registered hooks.
    pub async fn connect_and_stream(
        &self,
        device: btleplug::platform::Peripheral,
        context: &HookContext,
    ) -> Result<()> {
        let device_name = context
            .device_name
            .clone()
            .unwrap_or_else(|| "unknown".to_string());
        let context_for_protocol = context.clone();
        let context_for_sample = context.clone();
        let hook_registry_for_protocol = self.hook_registry_arc();
        let hook_registry_for_sample = self.hook_registry_arc();

        // The LSL manager is built lazily once HeartRateClient announces the
        // detected interval kind — we don't know whether to name the outlet
        // "RR <name>" or "PP <name>" until the first frame is parsed.
        let lsl_slot: Arc<std::sync::Mutex<Option<LslStreamManager>>> =
            Arc::new(std::sync::Mutex::new(None));
        let lsl_for_protocol = lsl_slot.clone();
        let lsl_for_sample = lsl_slot.clone();

        let on_protocol = move |kind: heart_rate::IntervalKind| -> Result<()> {
            println!("Building LSL outlets for kind={}...", kind.lsl_tag());
            let mgr = LslStreamManager::new(&device_name, kind)
                .with_context(|| format!("Failed to build LSL manager for kind {:?}", kind))?;
            *lsl_for_protocol.lock().unwrap() = Some(mgr);

            let mut ctx = context_for_protocol.clone();
            ctx.interval_kind = Some(kind);
            hook_registry_for_protocol.execute(HookPoint::PreStream, &ctx)?;
            Ok(())
        };

        let on_sample = move |sample: heart_rate::Sample| -> Result<()> {
            // Push to LSL
            let lsl_lock = lsl_for_sample.lock().unwrap();
            let mgr = lsl_lock
                .as_ref()
                .context("Sample received before protocol announce — internal invariant broken")?;
            if let Err(e) = mgr.push_heart_rate(sample.heart_rate_bpm) {
                eprintln!("Failed to push HR to LSL: {e}");
            }
            for ms in &sample.intervals_ms {
                if let Err(e) = mgr.push_interval(*ms) {
                    eprintln!("Failed to push interval to LSL: {e}");
                }
            }

            // Console output
            print!("HR: {}", sample.heart_rate_bpm);
            if !sample.intervals_ms.is_empty() {
                print!("  {}: {:?} ms", sample.kind.lsl_tag(), sample.intervals_ms);
            }
            println!();

            // Build per-sample HookContext. RR vs PP is split into separate
            // fields so hooks can branch on physiological provenance.
            let mut data_context = context_for_sample.clone();
            data_context.heart_rate = Some(sample.heart_rate_bpm.min(u8::MAX as u16) as u8);
            data_context.interval_kind = Some(sample.kind);
            match sample.kind {
                heart_rate::IntervalKind::Rr => data_context.rr_intervals = sample.intervals_ms,
                heart_rate::IntervalKind::Pp => data_context.pp_intervals = sample.intervals_ms,
            }
            if let Err(e) = hook_registry_for_sample.execute(HookPoint::DataReceived, &data_context)
            {
                eprintln!("Hook error (DataReceived): {e}");
            }
            if let Err(e) = hook_registry_for_sample.execute(HookPoint::PostStream, &data_context) {
                eprintln!("Hook error (PostStream): {e}");
            }
            Ok(())
        };

        // Catch both SIGINT (Ctrl-C) AND SIGTERM (`timeout`, `kill`, systemd
        // shutdown). Without SIGTERM handling, a `timeout 60 cargo run` kills
        // the process before STOP_PPI can be written, leaving the Polar band
        // streaming PPI for the next session — which then trips ALREADY_IN_STATE.
        let cancel = async {
            #[cfg(unix)]
            {
                use tokio::signal::unix::{signal, SignalKind};
                let mut sigterm = match signal(SignalKind::terminate()) {
                    Ok(s) => s,
                    Err(e) => {
                        eprintln!("Failed to install SIGTERM handler: {e}");
                        let _ = tokio::signal::ctrl_c().await;
                        println!("\nCtrl-C received; stopping...");
                        return;
                    }
                };
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => println!("\nCtrl-C received; stopping..."),
                    _ = sigterm.recv() => println!("\nSIGTERM received; stopping..."),
                }
            }
            #[cfg(not(unix))]
            {
                let _ = tokio::signal::ctrl_c().await;
                println!("\nCtrl-C received; stopping...");
            }
        };

        let client = client::HeartRateClient::new(device);
        let result = client.run(cancel, on_protocol, on_sample).await;

        // PreDisconnect hook (best-effort — the client has already disconnected).
        if let Err(e) = self
            .hook_registry
            .execute(HookPoint::PreDisconnect, context)
        {
            eprintln!("Hook error (PreDisconnect): {e}");
        }

        result
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_new() {
        let app = Application::new();
        // Just verify it can be created
        std::mem::drop(app);
    }

    #[test]
    fn test_application_default() {
        let app = Application::default();
        std::mem::drop(app);
    }

    #[test]
    fn test_hook_context_creation() {
        let mut context = HookContext::default();
        context.device_name = Some("test_device".to_string());
        assert_eq!(context.device_name, Some("test_device".to_string()));
    }
}
