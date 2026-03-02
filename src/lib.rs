pub mod ble;
pub mod heart_rate;
pub mod hooks;
pub mod lsl_stream;

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
        let devices = ble_manager
            .scan_devices(Duration::from_secs(5))
            .await
            .context("Failed to scan BLE devices")?;

        // Filter devices
        let filtered_devices = ble_manager
            .filter_devices(devices)
            .await
            .context("Failed to filter devices")?;

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

            match Select::new("Select device:", device_options).prompt() {
                Ok(selected_name) => {
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
                Err(_) => {
                    return Ok(None);
                }
            }
        }
    }

    /// Connect to a device and stream heart rate data
    pub async fn connect_and_stream(
        &self,
        device: btleplug::platform::Peripheral,
        context: &HookContext,
    ) -> Result<()> {
        // Connect to the device
        device
            .connect()
            .await
            .context("Failed to connect to device")?;

        // Discover services
        device
            .discover_services()
            .await
            .context("Failed to discover services")?;

        // Create LSL stream manager
        let device_name = context.device_name.as_deref().unwrap_or("unknown");
        let _lsl_manager = LslStreamManager::new(device_name)?;

        // Subscribe to heart rate characteristic notifications
        let characteristics = device.characteristics();

        // Find heart rate measurement characteristic (0x2A37)
        let hr_characteristic = characteristics
            .iter()
            .find(|c| c.uuid == ble::HR_MEASUREMENT_UUID)
            .context("Heart rate characteristic not found")?;

        // Subscribe to notifications
        device
            .subscribe(hr_characteristic)
            .await
            .context("Failed to subscribe to heart rate notifications")?;

        // Pre-stream hook
        self.hook_registry.execute(HookPoint::PreStream, context)?;

        println!("Connected to device. Listening for heart rate data (Ctrl+C to stop)...");

        // Get notification stream
        let mut notification_stream = device
            .notifications()
            .await
            .context("Failed to get notification stream")?;

        // Create LSL stream manager for actual streaming
        let mut lsl_manager = LslStreamManager::new(context.device_name.as_deref().unwrap_or("unknown"))?;

        // Listen for notifications and process heart rate data
        use futures::StreamExt;
        loop {
            tokio::select! {
                Some(notification) = notification_stream.next() => {
                    // Check if this is the heart rate characteristic
                    if notification.uuid == ble::HR_MEASUREMENT_UUID {
                        match heart_rate::parse_heart_rate_measurement(&notification.value) {
                            Ok(hr_data) => {
                                // Create context with received data
                                let mut data_context = context.clone();
                                data_context.heart_rate = Some(hr_data.heart_rate as u8);
                                data_context.rr_intervals = hr_data.rr_intervals.clone();

                                // DataReceived hook
                                if let Err(e) = self.hook_registry
                                    .execute(HookPoint::DataReceived, &data_context) {
                                    eprintln!("Hook error: {}", e);
                                }

                                // Print and stream heart rate
                                println!("HR: {}", hr_data.heart_rate);
                                if let Err(e) = lsl_manager.push_heart_rate(hr_data.heart_rate) {
                                    eprintln!("Failed to push HR to LSL: {}", e);
                                }

                                // Process RR intervals
                                for rr in &hr_data.rr_intervals {
                                    println!("    RR: {}", rr);
                                    if let Err(e) = lsl_manager.push_rr_interval(*rr) {
                                        eprintln!("Failed to push RR to LSL: {}", e);
                                    }
                                }

                                // PostStream hook
                                if let Err(e) = self.hook_registry
                                    .execute(HookPoint::PostStream, &data_context) {
                                    eprintln!("Hook error: {}", e);
                                }
                            }
                            Err(e) => {
                                eprintln!("Failed to parse heart rate data: {}", e);
                            }
                        }
                    }
                }
                else => {
                    // Notification stream ended
                    println!("Notification stream ended");
                    break;
                }
            }
        }

        // Post-stream hook (final)
        self.hook_registry.execute(HookPoint::PostStream, context)?;

        // Disconnect
        device
            .disconnect()
            .await
            .context("Failed to disconnect device")?;

        Ok(())
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
