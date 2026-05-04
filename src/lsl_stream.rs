use anyhow::Result;

use crate::heart_rate::IntervalKind;

#[cfg(feature = "lsl")]
use crate::lsl_ffi::LslOutlet;

/// Two LSL outlets for one device: one for heart-rate samples and one for
/// beat-to-beat intervals.
///
/// The HR outlet name is always `HR <device>` regardless of source. The
/// interval outlet's name and source-id swap between `RR` and `PP` based on
/// the [`IntervalKind`] detected by the BLE client — `RR <device>` for
/// ECG-derived bands (Polar H10, Wahoo TICKR, ...) and `PP <device>` for
/// PPG-derived bands (Polar Verity Sense, OH1, OH1+).
///
/// Layout per outlet matches the Android sibling and the Rust/Python refs:
/// type `Markers`, 1 int32 channel, irregular rate. All sample values are
/// **milliseconds**.
pub struct LslStreamManager {
    #[cfg(feature = "lsl")]
    hr_outlet: LslOutlet,
    #[cfg(feature = "lsl")]
    interval_outlet: LslOutlet,
    kind: IntervalKind,
}

impl LslStreamManager {
    /// Create a new LSL Stream Manager with two outlets (HR and the kind-
    /// appropriate interval stream).
    pub fn new(device_name: &str, kind: IntervalKind) -> Result<Self> {
        #[cfg(feature = "lsl")]
        {
            let tag = kind.lsl_tag(); // "RR" or "PP"

            let hr_outlet = LslOutlet::new(
                &format!("HR {device_name}"),
                "Markers",
                &format!("HR_markers_{device_name}"),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create HR outlet: {e}"))?;

            let interval_outlet = LslOutlet::new(
                &format!("{tag} {device_name}"),
                "Markers",
                &format!("{tag}_markers_{device_name}"),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create {tag} outlet: {e}"))?;

            println!("✓ LSL outlets created (HR + {tag})");

            Ok(LslStreamManager {
                hr_outlet,
                interval_outlet,
                kind,
            })
        }

        #[cfg(not(feature = "lsl"))]
        {
            println!("⚠ LSL feature disabled - no streams will be created");
            Ok(LslStreamManager { kind })
        }
    }

    /// Which interval characteristic this manager streams (`Rr` or `Pp`).
    pub fn kind(&self) -> IntervalKind {
        self.kind
    }

    /// Push a heart rate value (bpm) to the HR outlet.
    pub fn push_heart_rate(&self, heart_rate: u16) -> Result<()> {
        #[cfg(feature = "lsl")]
        {
            self.hr_outlet
                .push_sample(heart_rate as i32)
                .map_err(|e| anyhow::anyhow!("Failed to push HR: {e}"))?;
        }
        let _ = heart_rate;
        Ok(())
    }

    /// Push a beat-to-beat interval value (**milliseconds**) to the interval outlet.
    /// The outlet's stream name reflects whether this is RR or PP per `kind()`.
    pub fn push_interval(&self, interval_ms: u16) -> Result<()> {
        #[cfg(feature = "lsl")]
        {
            self.interval_outlet
                .push_sample(interval_ms as i32)
                .map_err(|e| anyhow::anyhow!("Failed to push interval: {e}"))?;
        }
        let _ = interval_ms;
        Ok(())
    }
}
