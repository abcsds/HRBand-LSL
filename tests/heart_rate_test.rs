use hrband_lsl::heart_rate::parse_heart_rate_measurement;

#[test]
fn test_parse_hr_uint8_format() {
    // flags: 0x08 = 0000_1000
    // Bit 0: 0 = uint8 format
    // Bits 1-2: 00 = not supported (sensor contact status)
    // Bit 3: 1 = energy expended present
    // Bit 4: 0 = no RR intervals
    // HR: 75 (uint8)
    // Energy expended: 100 (uint16 LE)
    let data = vec![0x08, 75, 100, 0];
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate, 75);
    assert_eq!(result.sensor_contact, None);
    assert_eq!(result.energy_expended, Some(100));
    assert!(result.rr_intervals.is_empty());
}

#[test]
fn test_parse_hr_uint16_format() {
    // flags: 0x01 = 0000_0001
    // Bit 0: 1 = uint16 format
    // Bits 1-2: 00 = not supported
    // Bit 3: 0 = no energy expended
    // Bit 4: 0 = no RR intervals
    // HR: 300 (uint16 LE at indices 1-2)
    let data = vec![0x01, 44, 1]; // 300 in LE = [44, 1] = 0x012C
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate, 300);
    assert_eq!(result.sensor_contact, None);
    assert_eq!(result.energy_expended, None);
    assert!(result.rr_intervals.is_empty());
}

#[test]
fn test_parse_with_rr_intervals() {
    // flags: 0x10 = 0001_0000
    // Bit 4: 1 = RR intervals present
    // HR: 72 (uint8)
    // RR intervals are wire-encoded in 1/1024-s units; the parser converts to ms.
    // raw 1000 → 977 ms; raw 1200 → 1172 ms.
    let data = vec![0x10, 72, 0xE8, 0x03, 0xB0, 0x04];
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate, 72);
    assert_eq!(result.sensor_contact, None);
    assert_eq!(result.energy_expended, None);
    assert_eq!(result.rr_intervals, vec![977, 1172]);
}

#[test]
fn test_parse_with_sensor_contact_detected() {
    // flags: 0x07 = 0000_0111
    // Bit 0: 1 = uint16 format
    // Bits 1-2: 11 = contact detected (sensor contact status)
    // Bit 3: 0 = no energy expended
    // Bit 4: 0 = no RR intervals
    // HR: 80 (uint16 LE)
    let data = vec![0x07, 80, 0];
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate, 80);
    assert_eq!(result.sensor_contact, Some(true));
    assert_eq!(result.energy_expended, None);
    assert!(result.rr_intervals.is_empty());
}

#[test]
fn test_parse_with_sensor_contact_no_contact() {
    // flags: 0x04 = 0000_0100
    // Bit 0: 0 = uint8 format
    // Bits 1-2: 10 = no contact (sensor contact status)
    // Bit 3: 0 = no energy expended
    // Bit 4: 0 = no RR intervals
    // HR: 65 (uint8)
    let data = vec![0x04, 65];
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate, 65);
    assert_eq!(result.sensor_contact, Some(false));
    assert_eq!(result.energy_expended, None);
    assert!(result.rr_intervals.is_empty());
}

#[test]
fn test_parse_all_fields_present() {
    // flags: 0x1F = 0001_1111
    // Bit 0: 1 = uint16 format
    // Bits 1-2: 11 = contact detected (sensor contact status)
    // Bit 3: 1 = energy expended present
    // Bit 4: 1 = RR intervals present
    // HR: 120 (uint16 LE at indices 1-2)
    // Energy expended: 500 (uint16 LE at indices 3-4) = [244, 1]
    // RR interval: raw 950 (1/1024 s units) → 928 ms after the parser conversion.
    let data = vec![0x1F, 120, 0, 244, 1, 182, 3];
    let result = parse_heart_rate_measurement(&data).unwrap();

    assert_eq!(result.heart_rate, 120);
    assert_eq!(result.sensor_contact, Some(true));
    assert_eq!(result.energy_expended, Some(500));
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
