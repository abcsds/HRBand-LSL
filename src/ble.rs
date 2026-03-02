use anyhow::{Context, Result};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::time::Duration;
use uuid::Uuid;

// Heart Rate Service UUID
pub const HR_SERVICE_UUID: Uuid = Uuid::from_u128(0x0000180d_0000_1000_8000_00805f9b34fb);

// Heart Rate Measurement Characteristic UUID
pub const HR_MEASUREMENT_UUID: Uuid = Uuid::from_u128(0x00002a37_0000_1000_8000_00805f9b34fb);

/// BLE Device Manager for scanning and filtering BLE devices
pub struct BleDeviceManager {
    adapter: Adapter,
}

impl BleDeviceManager {
    /// Create a new BleDeviceManager from the first available adapter
    pub async fn new() -> Result<Self> {
        let manager = Manager::new()
            .await
            .context("Failed to create BLE manager")?;

        let adapters = manager
            .adapters()
            .await
            .context("Failed to get BLE adapters")?;

        let adapter = adapters
            .into_iter()
            .next()
            .context("No BLE adapters found")?;

        Ok(BleDeviceManager { adapter })
    }

    /// Scan for BLE devices for the specified duration
    pub async fn scan_devices(&self, duration: Duration) -> Result<Vec<Peripheral>> {
        self.adapter
            .start_scan(ScanFilter::default())
            .await
            .context("Failed to start BLE scan")?;

        tokio::time::sleep(duration).await;

        self.adapter
            .stop_scan()
            .await
            .context("Failed to stop BLE scan")?;

        let peripherals = self
            .adapter
            .peripherals()
            .await
            .context("Failed to get BLE peripherals")?;

        Ok(peripherals)
    }

    /// Filter devices to exclude those without names or with "-" in name
    pub async fn filter_devices(&self, devices: Vec<Peripheral>) -> Result<Vec<Peripheral>> {
        let mut filtered = Vec::new();

        for device in devices {
            if let Ok(Some(properties)) = device.properties().await {
                if let Some(local_name) = properties.local_name {
                    // Exclude devices with "-" in name
                    if !local_name.contains('-') && !local_name.is_empty() {
                        println!("  ✓ {}", local_name);
                        filtered.push(device);
                    }
                }
            }
        }

        Ok(filtered)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constants_exist() {
        // Verify Heart Rate Service UUID
        assert_eq!(
            HR_SERVICE_UUID,
            Uuid::from_u128(0x0000180d_0000_1000_8000_00805f9b34fb)
        );

        // Verify Heart Rate Measurement UUID
        assert_eq!(
            HR_MEASUREMENT_UUID,
            Uuid::from_u128(0x00002a37_0000_1000_8000_00805f9b34fb)
        );
    }
}
