//! Polar Measurement Data (PMD) protocol — vendor BLE service used by Polar
//! OH1, OH1+, and Verity Sense to expose PPG-derived peak-to-peak intervals
//! (PPI). Polar publishes the protocol via their open polar-ble-sdk; this file
//! implements just enough of it to subscribe to PPI and parse frames.
//!
//! Protocol summary (PPI start request):
//!   1. Subscribe to indications on the PMD Control Point.
//!   2. Subscribe to notifications on the PMD Data characteristic.
//!   3. Write [0x02, 0x03] to the control point — "start measurement type=PPI".
//!      No additional setting bytes; PPI is event-driven (one sample per beat).
//!
//! Frame layout (PMD data notification, PPI):
//!   [0]    measurement type, must equal 0x03 (PMD_TYPE_PPI)
//!   [1..8] timestamp uint64 LE (ns since 2000-01-01) — we ignore it
//!   [9]    frame format
//!   [10..] sample blocks, 6 bytes each
//!
//! Each sample block:
//!   [0]    HR (uint8 bpm)
//!   [1..2] PPI (uint16 LE, milliseconds — already in the unit we want)
//!   [3..4] PPI error estimate (uint16 LE) — confidence indicator
//!   [5]    blocker flags
//!            bit 0 — skin contact bit (1 = no contact)
//!            bit 1 — skin contact supported
//!            bit 2 — low signal
//!
//! Filtering matches the Android sibling (RRStreamer): drop samples flagged as
//! no-skin-contact and samples with HR == 0 or PPI == 0.

use crate::ble::{PMD_OP_REQUEST_MEASUREMENT_START, PMD_OP_REQUEST_MEASUREMENT_STOP, PMD_TYPE_PPI};

/// Bytes to write to the PMD CP to start a PPI measurement.
pub const START_PPI_REQUEST: [u8; 2] = [PMD_OP_REQUEST_MEASUREMENT_START, PMD_TYPE_PPI];

/// Bytes to write to the PMD CP to stop a PPI measurement.
pub const STOP_PPI_REQUEST: [u8; 2] = [PMD_OP_REQUEST_MEASUREMENT_STOP, PMD_TYPE_PPI];

const HEADER_SIZE: usize = 10;
const SAMPLE_SIZE: usize = 6;
const CP_RESPONSE_OPCODE: u8 = 0xF0;

/// One accepted PPI sample block. Intervals are already in milliseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PpiSample {
    pub heart_rate_bpm: u8,
    pub ppi_ms: u16,
}

/// Parse a PMD data notification carrying PPI samples.
/// Returns one [`PpiSample`] per accepted block (no-contact + zero filter applied).
/// Empty if the frame is malformed or every block was filtered.
pub fn parse_ppi_frame(data: &[u8]) -> Vec<PpiSample> {
    if data.len() < HEADER_SIZE {
        return Vec::new();
    }
    if data[0] != PMD_TYPE_PPI {
        return Vec::new();
    }

    let mut out = Vec::new();
    let mut idx = HEADER_SIZE;
    while idx + SAMPLE_SIZE <= data.len() {
        let hr = data[idx];
        let ppi = u16::from_le_bytes([data[idx + 1], data[idx + 2]]);
        // err = u16::from_le_bytes([data[idx + 3], data[idx + 4]]);  // unused for now
        let blockers = data[idx + 5];
        idx += SAMPLE_SIZE;

        let no_contact = (blockers & 0x01) != 0;
        if no_contact || hr == 0 || ppi == 0 {
            continue;
        }
        out.push(PpiSample {
            heart_rate_bpm: hr,
            ppi_ms: ppi,
        });
    }
    out
}

/// PMD control-point response indication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CpResponse {
    pub op_code: u8,
    pub measurement_type: u8,
    pub error_code: u8,
}

impl CpResponse {
    pub fn is_success(&self) -> bool {
        self.error_code == 0
    }

    /// Human-readable error name for logging. Returns a `Cow` so the common
    /// case (known error code) is allocation-free; only unknown codes
    /// allocate to format `ERR_<code>`.
    pub fn error_name(&self) -> std::borrow::Cow<'static, str> {
        let known: Option<&'static str> = match self.error_code {
            0 => Some("SUCCESS"),
            1 => Some("INVALID_OP_CODE"),
            2 => Some("INVALID_MEASUREMENT_TYPE"),
            3 => Some("NOT_SUPPORTED"),
            4 => Some("INVALID_LENGTH"),
            5 => Some("INVALID_PARAMETER"),
            6 => Some("ALREADY_IN_STATE"),
            7 => Some("INVALID_RESOLUTION"),
            8 => Some("INVALID_SAMPLE_RATE"),
            9 => Some("INVALID_RANGE"),
            10 => Some("INVALID_MTU"),
            _ => None,
        };
        match known {
            Some(s) => std::borrow::Cow::Borrowed(s),
            None => std::borrow::Cow::Owned(format!("ERR_{}", self.error_code)),
        }
    }
}

/// Parse a PMD Control Point response indication.
///
/// Layout:
///   [0] = 0xF0 (response opcode)
///   [1] = original op code (0x02 = START, 0x03 = STOP, ...)
///   [2] = measurement type (0x03 = PPI)
///   [3] = error code (0 = SUCCESS; 3 = NOT_SUPPORTED; 10 = INVALID_MTU; ...)
///   [4..] optional parameter frame (only on SUCCESS) — ignored
///
/// Returns `None` if the frame isn't a CP response we understand; the caller
/// should ignore those.
pub fn parse_cp_response(data: &[u8]) -> Option<CpResponse> {
    if data.len() < 4 || data[0] != CP_RESPONSE_OPCODE {
        return None;
    }
    Some(CpResponse {
        op_code: data[1],
        measurement_type: data[2],
        error_code: data[3],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(samples: &[[u8; 6]]) -> Vec<u8> {
        let mut out = vec![PMD_TYPE_PPI];
        out.extend_from_slice(&[0u8; 8]); // timestamp, ignored
        out.push(0); // frame format
        for s in samples {
            out.extend_from_slice(s);
        }
        out
    }

    #[test]
    fn parse_ppi_frame_empty_input_returns_empty() {
        assert_eq!(parse_ppi_frame(&[]), vec![]);
    }

    #[test]
    fn parse_ppi_frame_too_short_returns_empty() {
        // Less than the 10-byte header.
        assert_eq!(parse_ppi_frame(&[0x03, 0x00, 0x00, 0x00]), vec![]);
    }

    #[test]
    fn parse_ppi_frame_wrong_measurement_type_returns_empty() {
        // Header is the right size but the type byte is not PPI (0x03).
        let mut frame = vec![0u8; HEADER_SIZE];
        frame[0] = 0x01; // ECG, not PPI
        assert_eq!(parse_ppi_frame(&frame), vec![]);
    }

    #[test]
    fn parse_ppi_frame_single_valid_sample() {
        // HR = 72 bpm, PPI = 850 ms (0x352), error = 10, blockers = 0x02 (contact OK)
        let sample: [u8; 6] = [72, 0x52, 0x03, 0x0A, 0x00, 0x02];
        let frame = make_frame(&[sample]);
        assert_eq!(
            parse_ppi_frame(&frame),
            vec![PpiSample {
                heart_rate_bpm: 72,
                ppi_ms: 850
            }],
        );
    }

    #[test]
    fn parse_ppi_frame_multiple_samples() {
        let s1: [u8; 6] = [70, 0xE8, 0x03, 0x00, 0x00, 0x02]; // 1000 ms
        let s2: [u8; 6] = [71, 0xC4, 0x03, 0x00, 0x00, 0x02]; // 964 ms
        let frame = make_frame(&[s1, s2]);
        assert_eq!(
            parse_ppi_frame(&frame),
            vec![
                PpiSample {
                    heart_rate_bpm: 70,
                    ppi_ms: 1000
                },
                PpiSample {
                    heart_rate_bpm: 71,
                    ppi_ms: 964
                },
            ],
        );
    }

    #[test]
    fn parse_ppi_frame_drops_no_contact_samples() {
        let bad: [u8; 6] = [70, 0xE8, 0x03, 0x00, 0x00, 0x01]; // bit 0 set = no contact
        let good: [u8; 6] = [71, 0xC4, 0x03, 0x00, 0x00, 0x02];
        let frame = make_frame(&[bad, good]);
        assert_eq!(
            parse_ppi_frame(&frame),
            vec![PpiSample {
                heart_rate_bpm: 71,
                ppi_ms: 964
            }],
        );
    }

    #[test]
    fn parse_ppi_frame_drops_zero_hr() {
        let bad: [u8; 6] = [0, 0xE8, 0x03, 0x00, 0x00, 0x02];
        let good: [u8; 6] = [71, 0xC4, 0x03, 0x00, 0x00, 0x02];
        let frame = make_frame(&[bad, good]);
        assert_eq!(
            parse_ppi_frame(&frame),
            vec![PpiSample {
                heart_rate_bpm: 71,
                ppi_ms: 964
            }],
        );
    }

    #[test]
    fn parse_ppi_frame_drops_zero_ppi() {
        let bad: [u8; 6] = [70, 0x00, 0x00, 0x00, 0x00, 0x02];
        let good: [u8; 6] = [71, 0xC4, 0x03, 0x00, 0x00, 0x02];
        let frame = make_frame(&[bad, good]);
        assert_eq!(
            parse_ppi_frame(&frame),
            vec![PpiSample {
                heart_rate_bpm: 71,
                ppi_ms: 964
            }],
        );
    }

    #[test]
    fn parse_ppi_frame_ignores_partial_trailing_block() {
        let s1: [u8; 6] = [70, 0xE8, 0x03, 0x00, 0x00, 0x02];
        let mut frame = make_frame(&[s1]);
        frame.extend_from_slice(&[0xAB, 0xCD]); // 2 trailing bytes, not a full block
        assert_eq!(
            parse_ppi_frame(&frame),
            vec![PpiSample {
                heart_rate_bpm: 70,
                ppi_ms: 1000
            }],
        );
    }

    #[test]
    fn parse_cp_response_returns_none_on_short_input() {
        assert_eq!(parse_cp_response(&[0xF0, 0x02, 0x03]), None);
    }

    #[test]
    fn parse_cp_response_returns_none_on_wrong_opcode() {
        assert_eq!(parse_cp_response(&[0xAA, 0x02, 0x03, 0x00]), None);
    }

    #[test]
    fn parse_cp_response_success() {
        let resp = parse_cp_response(&[0xF0, 0x02, 0x03, 0x00, 0xAA]).unwrap();
        assert_eq!(resp.op_code, 0x02);
        assert_eq!(resp.measurement_type, 0x03);
        assert_eq!(resp.error_code, 0);
        assert!(resp.is_success());
        assert_eq!(resp.error_name(), "SUCCESS");
    }

    #[test]
    fn parse_cp_response_invalid_mtu_named() {
        // The error case we most expect to see in the wild on Linux/BlueZ
        // when MTU negotiation didn't reach the 232 the PMD CP wants.
        let resp = parse_cp_response(&[0xF0, 0x02, 0x03, 0x0A]).unwrap();
        assert!(!resp.is_success());
        assert_eq!(resp.error_name(), "INVALID_MTU");
    }

    #[test]
    fn parse_cp_response_unknown_code_falls_back() {
        let resp = parse_cp_response(&[0xF0, 0x02, 0x03, 0xFE]).unwrap();
        assert_eq!(resp.error_name(), "ERR_254");
    }

    #[test]
    fn start_ppi_request_bytes() {
        assert_eq!(START_PPI_REQUEST, [0x02, 0x03]);
    }

    #[test]
    fn stop_ppi_request_bytes() {
        assert_eq!(STOP_PPI_REQUEST, [0x03, 0x03]);
    }
}
