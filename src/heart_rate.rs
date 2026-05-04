use anyhow::bail;

/// Convert the BLE 1/1024-s RR raw value to milliseconds, rounded to nearest.
/// Mirrors the Android sibling's `HeartRateParser.rawToMs`.
pub fn raw_to_ms(raw: u16) -> u16 {
    // (raw * 1000 + 512) / 1024 — half-up rounding via integer math. The i64
    // intermediate keeps headroom; max value is 65535*1000+512 = 65_535_512.
    (((raw as u64) * 1000 + 512) / 1024) as u16
}

/// Provenance of a beat-to-beat interval sample.
///
/// `Rr` — true R-R intervals from an ECG band's standard 0x2A37 RR-bit payload.
/// Highest accuracy; suitable for clinical-grade HRV analysis.
///
/// `Pp` — peak-to-peak intervals from a PPG band's Polar PMD/PPI stream. Lower
/// accuracy than RR — the PPG waveform smooths short cycles, so derived HRV
/// metrics (RMSSD, etc.) are an *approximation* of the underlying ECG truth.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntervalKind {
    Rr,
    Pp,
}

impl IntervalKind {
    /// Tag used in LSL stream names and source-ids: `RR` or `PP`.
    pub fn lsl_tag(&self) -> &'static str {
        match self {
            IntervalKind::Rr => "RR",
            IntervalKind::Pp => "PP",
        }
    }
}

/// One beat-to-beat sample as the BLE client emits it to the application.
/// Same shape regardless of source; `kind` carries provenance.
///
/// `heart_rate_bpm` is u16 to accommodate the BLE 0x2A37 wide format (`uint16`
/// HR flag bit). PMD/PPI frames carry a u8 HR value which gets widened
/// losslessly. Real heart rates always fit in u8, but the wider type avoids a
/// truncation step on the wide-format path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sample {
    pub heart_rate_bpm: u16,
    pub kind: IntervalKind,
    /// Always milliseconds. Empty if the band reports HR but no interval (e.g.
    /// HR-only band, or Polar H10 just after reconnect before the first RR).
    pub intervals_ms: Vec<u16>,
}

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

    // Parse RR intervals if present. The raw 1/1024-s units are converted to
    // milliseconds at the parse boundary so everything downstream (LSL outlets,
    // hooks) speaks ms — matching the Android sibling and PMD/PPI samples.
    let mut rr_intervals = Vec::new();
    if has_rr_intervals {
        while idx + 1 < data.len() {
            let raw = u16::from_le_bytes([data[idx], data[idx + 1]]);
            rr_intervals.push(raw_to_ms(raw));
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
        // Raw 1000 → 977 ms; raw 1200 → 1172 ms (1/1024 s units → ms via raw_to_ms).
        assert_eq!(result.rr_intervals, vec![977, 1172]);
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
        // Raw 950 (1/1024 s) → 928 ms.
        assert_eq!(result.rr_intervals, vec![928]);
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

    #[test]
    fn raw_to_ms_zero() {
        assert_eq!(raw_to_ms(0), 0);
    }

    #[test]
    fn raw_to_ms_exactly_one_second() {
        // 1024 raw units = 1 second exactly = 1000 ms.
        assert_eq!(raw_to_ms(1024), 1000);
    }

    #[test]
    fn raw_to_ms_rounds_to_nearest() {
        // Same rounding the Android sibling uses: ((raw * 1000 + 512) / 1024).
        // 1025 * 1000 / 1024 = 1000.976... → rounds up to 1001.
        assert_eq!(raw_to_ms(1025), 1001);
        // 1023 * 1000 / 1024 = 999.023... → rounds to 999.
        assert_eq!(raw_to_ms(1023), 999);
    }

    #[test]
    fn raw_to_ms_top_of_u16_range() {
        // u16::MAX = 65535. Largest value that can come through. Must not overflow
        // the intermediate. 65535*1000+512 = 65_535_512; / 1024 = 63999 rem 536.
        assert_eq!(raw_to_ms(u16::MAX), 63999);
    }

    #[test]
    fn interval_kind_lsl_tag() {
        assert_eq!(IntervalKind::Rr.lsl_tag(), "RR");
        assert_eq!(IntervalKind::Pp.lsl_tag(), "PP");
    }

    #[test]
    fn sample_constructor_round_trip() {
        let s = Sample {
            heart_rate_bpm: 72,
            kind: IntervalKind::Pp,
            intervals_ms: vec![800, 820],
        };
        assert_eq!(s.heart_rate_bpm, 72u16);
        assert_eq!(s.kind, IntervalKind::Pp);
        assert_eq!(s.intervals_ms, vec![800, 820]);
    }
}
