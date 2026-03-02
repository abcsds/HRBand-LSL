use anyhow::bail;

/// Heart rate measurement data parsed from BLE characteristic 0x2A37
#[derive(Debug, Clone, PartialEq)]
pub struct HeartRateData {
    /// Heart rate in beats per minute
    pub heart_rate: u16,
    /// Sensor contact status (None = not supported, Some(true) = contact, Some(false) = no contact)
    pub sensor_contact: Option<bool>,
    /// Energy expended in kilojoules (if present)
    pub energy_expended: Option<u16>,
    /// RR intervals in milliseconds (if present)
    pub rr_intervals: Vec<u16>,
}

/// Parse BLE Heart Rate Measurement characteristic (0x2A37)
///
/// # Arguments
/// * `data` - Raw BLE measurement data
///
/// # Returns
/// * `Ok(HeartRateData)` - Parsed heart rate data
/// * `Err` - If data is invalid or insufficient
///
/// # BLE Spec
/// Byte 0 - Flags:
///   - Bit 0: HR format (0 = uint8, 1 = uint16)
///   - Bits 1-2: Sensor contact status
///   - Bit 3: Energy expended present
///   - Bit 4: RR interval present
pub fn parse_heart_rate_measurement(data: &[u8]) -> anyhow::Result<HeartRateData> {
    // Validate minimum data length (at least flags byte)
    if data.is_empty() {
        bail!("Heart rate data is empty");
    }

    let flags = data[0];

    // Extract individual flag bits
    let hr_format = flags & 0x01; // Bit 0
    let sensor_contact_bits = (flags >> 1) & 0x03; // Bits 1-2 (shift right by 1, mask 2 bits)
    let has_energy_expended = (flags & 0x08) != 0; // Bit 3
    let has_rr_intervals = (flags & 0x10) != 0; // Bit 4

    // Parse sensor contact status
    let sensor_contact = match sensor_contact_bits {
        0 | 1 => None,    // Not supported
        2 => Some(false), // No contact
        3 => Some(true),  // Contact detected
        _ => None,        // Shouldn't happen, but default to None
    };

    // Parse heart rate based on format
    let index;
    let heart_rate = if hr_format == 0 {
        // uint8 format
        if data.len() < 2 {
            bail!("Insufficient data for uint8 heart rate");
        }
        index = 2;
        data[1] as u16
    } else {
        // uint16 format (little-endian)
        if data.len() < 3 {
            bail!("Insufficient data for uint16 heart rate");
        }
        index = 3;
        u16::from_le_bytes([data[1], data[2]])
    };

    // Parse energy expended if present
    let mut idx = index;
    let energy_expended = if has_energy_expended {
        if data.len() < idx + 2 {
            bail!("Insufficient data for energy expended");
        }
        let energy = u16::from_le_bytes([data[idx], data[idx + 1]]);
        idx += 2;
        Some(energy)
    } else {
        None
    };

    // Parse RR intervals if present
    let mut rr_intervals = Vec::new();
    if has_rr_intervals {
        // RR intervals are pairs of bytes (uint16 LE) until end of data
        while idx + 1 < data.len() {
            let rr = u16::from_le_bytes([data[idx], data[idx + 1]]);
            rr_intervals.push(rr);
            idx += 2;
        }

        // Check if there's a trailing byte without a pair
        if idx < data.len() {
            bail!("Incomplete RR interval data (odd number of bytes)");
        }
    }

    Ok(HeartRateData {
        heart_rate,
        sensor_contact,
        energy_expended,
        rr_intervals,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hr_uint8_format() {
        let data = vec![0x08, 75, 100, 0];
        let result = parse_heart_rate_measurement(&data).unwrap();

        assert_eq!(result.heart_rate, 75);
        assert_eq!(result.sensor_contact, None);
        assert_eq!(result.energy_expended, Some(100));
        assert!(result.rr_intervals.is_empty());
    }

    #[test]
    fn test_parse_hr_uint16_format() {
        let data = vec![0x01, 44, 1];
        let result = parse_heart_rate_measurement(&data).unwrap();

        assert_eq!(result.heart_rate, 300);
        assert_eq!(result.sensor_contact, None);
        assert_eq!(result.energy_expended, None);
        assert!(result.rr_intervals.is_empty());
    }

    #[test]
    fn test_parse_with_rr_intervals() {
        let data = vec![0x10, 72, 0xE8, 0x03, 0xB0, 0x04];
        let result = parse_heart_rate_measurement(&data).unwrap();

        assert_eq!(result.heart_rate, 72);
        assert_eq!(result.sensor_contact, None);
        assert_eq!(result.energy_expended, None);
        assert_eq!(result.rr_intervals, vec![1000, 1200]);
    }

    #[test]
    fn test_parse_with_sensor_contact_detected() {
        let data = vec![0x07, 80, 0];
        let result = parse_heart_rate_measurement(&data).unwrap();

        assert_eq!(result.heart_rate, 80);
        assert_eq!(result.sensor_contact, Some(true));
        assert_eq!(result.energy_expended, None);
        assert!(result.rr_intervals.is_empty());
    }

    #[test]
    fn test_parse_with_sensor_contact_no_contact() {
        let data = vec![0x04, 65];
        let result = parse_heart_rate_measurement(&data).unwrap();

        assert_eq!(result.heart_rate, 65);
        assert_eq!(result.sensor_contact, Some(false));
        assert_eq!(result.energy_expended, None);
        assert!(result.rr_intervals.is_empty());
    }

    #[test]
    fn test_parse_all_fields_present() {
        let data = vec![0x1F, 120, 0, 244, 1, 182, 3];
        let result = parse_heart_rate_measurement(&data).unwrap();

        assert_eq!(result.heart_rate, 120);
        assert_eq!(result.sensor_contact, Some(true));
        assert_eq!(result.energy_expended, Some(500));
        assert_eq!(result.rr_intervals, vec![950]);
    }

    #[test]
    fn test_parse_insufficient_data() {
        let data = vec![0x01];
        let result = parse_heart_rate_measurement(&data);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_data() {
        let data = vec![];
        let result = parse_heart_rate_measurement(&data);
        assert!(result.is_err());
    }
}
