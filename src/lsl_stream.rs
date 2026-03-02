use anyhow::Result;

#[cfg(feature = "lsl")]
use crate::lsl_ffi::LslOutlet;

/// LSL Stream Manager for handling heart rate and RR interval data streams
pub struct LslStreamManager {
    #[cfg(feature = "lsl")]
    hr_outlet: LslOutlet,
    #[cfg(feature = "lsl")]
    rr_outlet: LslOutlet,
}

impl LslStreamManager {
    /// Create a new LSL Stream Manager with two outlets (HR and RR)
    pub fn new(device_name: &str) -> Result<Self> {
        #[cfg(feature = "lsl")]
        {
            let hr_outlet = LslOutlet::new(
                &format!("HR {}", device_name),
                "Markers",
                &format!("HR_markers_{}", device_name),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create HR outlet: {}", e))?;

            let rr_outlet = LslOutlet::new(
                &format!("RR {}", device_name),
                "Markers",
                &format!("RR_markers_{}", device_name),
            )
            .map_err(|e| anyhow::anyhow!("Failed to create RR outlet: {}", e))?;

            println!("✓ LSL outlets created");

            Ok(LslStreamManager {
                hr_outlet,
                rr_outlet,
            })
        }

        #[cfg(not(feature = "lsl"))]
        {
            println!("⚠ LSL feature disabled - no streams will be created");
            Ok(LslStreamManager {})
        }
    }

    /// Push a heart rate value to the LSL stream
    pub fn push_heart_rate(&self, heart_rate: u16) -> Result<()> {
        #[cfg(feature = "lsl")]
        {
            self.hr_outlet
                .push_sample(heart_rate as i32)
                .map_err(|e| anyhow::anyhow!("Failed to push HR: {}", e))?;
        }
        Ok(())
    }

    /// Push an RR interval value to the LSL stream
    pub fn push_rr_interval(&self, rr_interval: u16) -> Result<()> {
        #[cfg(feature = "lsl")]
        {
            self.rr_outlet
                .push_sample(rr_interval as i32)
                .map_err(|e| anyhow::anyhow!("Failed to push RR: {}", e))?;
        }
        Ok(())
    }
}
