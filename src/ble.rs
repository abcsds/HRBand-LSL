use anyhow::{Context, Result};
use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Adapter, Manager, Peripheral};
use std::time::Duration;
use uuid::Uuid;

// ---------- Standard Heart Rate Service (0x180D) — H10, TICKR, Garmin HRM, etc. ----------
pub const HR_SERVICE_UUID: Uuid = Uuid::from_u128(0x0000180d_0000_1000_8000_00805f9b34fb);
pub const HR_MEASUREMENT_UUID: Uuid = Uuid::from_u128(0x00002a37_0000_1000_8000_00805f9b34fb);
pub const CCCD_UUID: Uuid = Uuid::from_u128(0x00002902_0000_1000_8000_00805f9b34fb);

// ---------- Polar Measurement Data (PMD) service ----------
// Vendor service exposed by Polar OH1, OH1+, Verity Sense (and ancillary on H10).
// Used to subscribe to PPI samples from PPG bands that don't advertise R-R intervals
// via the standard 0x2A37 flag bit. UUIDs and protocol constants come from the
// open polar-ble-sdk.
pub const POLAR_PMD_SERVICE_UUID: Uuid = Uuid::from_u128(0xFB005C80_02E7_F387_1CAD_8ACD2D8DF0C8);
pub const POLAR_PMD_CP_UUID: Uuid = Uuid::from_u128(0xFB005C81_02E7_F387_1CAD_8ACD2D8DF0C8);
pub const POLAR_PMD_DATA_UUID: Uuid = Uuid::from_u128(0xFB005C82_02E7_F387_1CAD_8ACD2D8DF0C8);

// PMD measurement type codes (subset; PPI is the only one we need).
pub const PMD_TYPE_PPI: u8 = 0x03;

// PMD control-point op codes.
pub const PMD_OP_REQUEST_MEASUREMENT_START: u8 = 0x02;
pub const PMD_OP_REQUEST_MEASUREMENT_STOP: u8 = 0x03;

/// BLE Device Manager for scanning and filtering BLE devices
pub struct BleDeviceManager {
    adapter: Adapter,
}

impl BleDeviceManager {
    /// Create a new BleDeviceManager from the first available adapter.
    ///
    /// On hosts with multiple BLE radios (e.g. built-in plus USB dongle),
    /// whichever adapter the platform layer enumerates first is used; a
    /// warning is logged so the user knows they may need to disable the
    /// unwanted radio at the OS level.
    pub async fn new() -> Result<Self> {
        let manager = Manager::new()
            .await
            .context("Failed to create BLE manager")?;

        let adapters = manager
            .adapters()
            .await
            .context("Failed to get BLE adapters")?;

        let count = adapters.len();
        let adapter = adapters
            .into_iter()
            .next()
            .context("No BLE adapters found")?;

        if count > 1 {
            eprintln!(
                "Warning: {count} BLE adapters available; using the first. Disable unwanted radios at the OS level if this picks the wrong one."
            );
        }

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

    /// Filter devices to exclude those without names or with "-" in name.
    ///
    /// The `-` filter is inherited from the Python and Android refs (see
    /// `RRStreamer/CLAUDE.md`); it drops devices like `Govee_H6056_-714B`
    /// whose advertising names contain hyphens, on the assumption these are
    /// background MAC-suffix-style noise rather than HR bands. **Caveat:**
    /// some Polar firmwares emit hyphenated suffixes (e.g.
    /// `Polar H10 ABCD-1234`); if you're missing a known band, this filter
    /// is the first place to look.
    pub async fn filter_devices(&self, devices: Vec<Peripheral>) -> Result<Vec<Peripheral>> {
        let mut filtered = Vec::new();

        for device in devices {
            if let Ok(Some(properties)) = device.properties().await {
                if let Some(local_name) = properties.local_name {
                    if !local_name.contains('-') && !local_name.is_empty() {
                        println!("  [ok] {}", local_name);
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

        // Standard CCCD descriptor UUID (used to enable notifications/indications).
        assert_eq!(
            CCCD_UUID,
            Uuid::from_u128(0x00002902_0000_1000_8000_00805f9b34fb)
        );

        // Polar PMD service + characteristics: vendor-specific UUIDs published
        // via polar-ble-sdk. Used for PPI on PPG-only bands (Verity Sense, OH1).
        assert_eq!(
            POLAR_PMD_SERVICE_UUID,
            Uuid::from_u128(0xFB005C80_02E7_F387_1CAD_8ACD2D8DF0C8)
        );
        assert_eq!(
            POLAR_PMD_CP_UUID,
            Uuid::from_u128(0xFB005C81_02E7_F387_1CAD_8ACD2D8DF0C8)
        );
        assert_eq!(
            POLAR_PMD_DATA_UUID,
            Uuid::from_u128(0xFB005C82_02E7_F387_1CAD_8ACD2D8DF0C8)
        );
    }
}
