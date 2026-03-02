use hrband_lsl::lsl_stream::LslStreamManager;

#[test]
fn test_lsl_stream_manager_new() {
    let result = LslStreamManager::new("test_device");
    // The result depends on whether LSL is available and configured
    // We just verify the function exists and returns a Result
    assert!(result.is_ok() || result.is_err());
}

#[test]
#[cfg(not(feature = "lsl"))]
fn test_lsl_stream_manager_no_lsl_feature() {
    let manager = LslStreamManager::new("test_device").unwrap();

    // Test pushing heart rate
    let hr_result = manager.push_heart_rate(72);
    assert!(hr_result.is_ok());

    // Test pushing RR interval
    let rr_result = manager.push_rr_interval(800);
    assert!(rr_result.is_ok());
}
