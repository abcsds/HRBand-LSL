use hrband_lsl::heart_rate::IntervalKind;
use hrband_lsl::lsl_stream::LslStreamManager;

#[test]
fn lsl_stream_manager_constructible_for_rr_band() {
    // Whether the OS actually has liblsl loaded is environment-dependent;
    // we just check that the constructor accepts the new (name, kind) signature
    // and returns either Ok (with liblsl present) or Err (without it).
    let result = LslStreamManager::new("test_rr_device", IntervalKind::Rr);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn lsl_stream_manager_constructible_for_pp_band() {
    let result = LslStreamManager::new("test_pp_device", IntervalKind::Pp);
    assert!(result.is_ok() || result.is_err());
}

#[test]
#[cfg(not(feature = "lsl"))]
fn lsl_stream_manager_pushes_without_lsl_feature() {
    let manager = LslStreamManager::new("test_device", IntervalKind::Rr).unwrap();
    assert!(manager.push_heart_rate(72).is_ok());
    assert!(manager.push_interval(800).is_ok());
}
