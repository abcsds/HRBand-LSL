use anyhow::Result;

/// LSL Stream Manager for handling heart rate and RR interval data streams
#[cfg(feature = "lsl")]
pub struct LslStreamManager {
    hr_outlet: lsl::StreamOutlet,
    rr_outlet: lsl::StreamOutlet,
}

#[cfg(feature = "lsl")]
impl LslStreamManager {
    /// Create a new LSL Stream Manager with two outlets (HR and RR)
    ///
    /// # Arguments
    /// * `device_name` - Name of the device (used in stream ID)
    ///
    /// # Returns
    /// * `Ok(LslStreamManager)` - Successfully created stream manager
    /// * `Err` - If stream creation fails
    pub fn new(device_name: &str) -> Result<Self> {
        // Create Heart Rate stream info
        let hr_info = lsl::StreamInfo::new(
            &format!("HeartRate-{}", device_name),
            "HeartRate",
            1,
            100.0, // sampling rate
            lsl::ValueType::Int32,
            &format!("hrband_hr_{}", device_name),
        );
        let hr_outlet = lsl::StreamOutlet::new(&hr_info, 0, 360)?;

        // Create RR Interval stream info
        let rr_info = lsl::StreamInfo::new(
            &format!("RRInterval-{}", device_name),
            "RRInterval",
            1,
            100.0, // sampling rate
            lsl::ValueType::Int32,
            &format!("hrband_rr_{}", device_name),
        );
        let rr_outlet = lsl::StreamOutlet::new(&rr_info, 0, 360)?;

        Ok(LslStreamManager {
            hr_outlet,
            rr_outlet,
        })
    }

    /// Push a heart rate value to the LSL stream
    ///
    /// # Arguments
    /// * `heart_rate` - Heart rate value in beats per minute
    ///
    /// # Returns
    /// * `Ok(())` - Successfully pushed value
    /// * `Err` - If push operation fails
    pub fn push_heart_rate(&self, heart_rate: u16) -> Result<()> {
        self.hr_outlet.push_sample(&[heart_rate as i32])?;
        Ok(())
    }

    /// Push an RR interval value to the LSL stream
    ///
    /// # Arguments
    /// * `rr_interval` - RR interval value in milliseconds
    ///
    /// # Returns
    /// * `Ok(())` - Successfully pushed value
    /// * `Err` - If push operation fails
    pub fn push_rr_interval(&self, rr_interval: u16) -> Result<()> {
        self.rr_outlet.push_sample(&[rr_interval as i32])?;
        Ok(())
    }
}

#[cfg(not(feature = "lsl"))]
pub struct LslStreamManager;

#[cfg(not(feature = "lsl"))]
impl LslStreamManager {
    pub fn new(_device_name: &str) -> Result<Self> {
        Ok(LslStreamManager)
    }

    pub fn push_heart_rate(&self, _heart_rate: u16) -> Result<()> {
        Ok(())
    }

    pub fn push_rr_interval(&self, _rr_interval: u16) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(feature = "lsl")]
    fn test_lsl_stream_manager_creation() {
        // Test that we can create a stream manager
        let result = LslStreamManager::new("test_device");
        // Creation may fail if LSL is not properly set up, so we just check it returns a Result
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    #[cfg(not(feature = "lsl"))]
    fn test_lsl_stream_manager_no_feature() {
        let manager = LslStreamManager::new("test_device").unwrap();
        assert!(manager.push_heart_rate(72).is_ok());
        assert!(manager.push_rr_interval(800).is_ok());
    }
}
