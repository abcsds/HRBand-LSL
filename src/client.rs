//! Connects to a heart-rate band, picks the right BLE protocol per device, and
//! emits unified [`Sample`] events. Mirrors the Android sibling's
//! `HeartRateClient`, adapted to btleplug's async-stream model.
//!
//! Protocol selection (decided on the first 0x2A37 notifications):
//!
//!   1. Connect, discover services, subscribe to the standard Heart Rate
//!      Measurement characteristic (0x2A37).
//!   2. On the FIRST notification, look at the flag byte:
//!        a. If RR-intervals bit (bit 4) is set → ECG-style band (Polar H10,
//!           Wahoo TICKR, Garmin HRM-Pro, etc.). Stay subscribed; emit
//!           `Sample { kind: Rr }`.
//!        b. Else if the device exposes the Polar PMD service → PPG-style band
//!           (Polar Verity Sense, OH1, OH1+). After [`HR_ONLY_DECISION_WINDOW`]
//!           HR-only frames, unsubscribe 0x2A37, write a "start PPI" request
//!           to the PMD control point, subscribe to PMD data; emit
//!           `Sample { kind: Pp }`.
//!        c. Else → HR-only band. Stay subscribed; emit
//!           `Sample { kind: Rr, intervals_ms: [] }`.
//!
//! The 3-frame decision window guards against a Polar H10 reconnect quirk
//! where the band sends an HR-only frame before computing its first R-R
//! interval — committing to PPI on that single frame would strand us waiting
//! for samples a band that doesn't produce them. See the Android sibling's
//! `HeartRateClient.kt` for the original write-up.
//!
//! Single-shot lifecycle: call [`HeartRateClient::run`] with a cancel future;
//! it returns when the peripheral disconnects, the cancel fires, or an
//! unrecoverable error occurs (e.g. PMD `INVALID_MTU`).

use std::future::Future;

use anyhow::{bail, Context, Result};
use btleplug::api::{Characteristic, Peripheral as _, ValueNotification, WriteType};
use btleplug::platform::Peripheral;
use futures::{FutureExt, StreamExt};

use crate::ble;
use crate::heart_rate::{self, IntervalKind, Sample};
use crate::polar_pmd;

/// How many HR-only 0x2A37 frames we tolerate before falling back to PMD/PPI
/// when the device exposes the PMD service. Three is enough for an ECG band
/// that's still warming up after reconnect to emit at least one RR-bearing frame.
pub const HR_ONLY_DECISION_WINDOW: u8 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Mode {
    Initial {
        hr_only_frames: u8,
    },
    StandardRr,
    HrOnly,
    /// PolarPpi: subscriptions in flight. `protocol_announced` flips to true
    /// after the PMD CP returns a SUCCESS response, at which point we know the
    /// band is happy with the request and PPI frames will arrive.
    PolarPpi {
        protocol_announced: bool,
    },
}

/// What the state machine wants to do in response to one 0x2A37 frame seen
/// while still in [`Mode::Initial`]. Extracted for unit testing — the BLE-side
/// effects (unsubscribe / write / subscribe PMD) are driven by [`Self::SwitchToPpi`].
#[derive(Debug, PartialEq, Eq)]
enum InitialDecision {
    /// Commit to standard RR mode and emit the current frame as a Sample.
    CommitRr,
    /// Commit to HR-only mode and emit the current frame as a Sample
    /// (intervals_ms will be empty).
    CommitHrOnly,
    /// Defer the decision — increment the HR-only frame counter and skip
    /// emitting this frame.
    Defer,
    /// Commit to Polar PPI mode and execute the PMD subscription chain.
    SwitchToPpi,
}

/// Decide what to do with a 0x2A37 frame while still in `Mode::Initial`. Pure
/// function for unit testing; the BLE-side effects are handled by the caller.
fn decide_initial(value: &[u8], hr_only_frames: u8, pmd_present: bool) -> InitialDecision {
    let rr_advertised = !value.is_empty() && (value[0] & 0x10) != 0;
    if rr_advertised {
        InitialDecision::CommitRr
    } else if !pmd_present {
        InitialDecision::CommitHrOnly
    } else if hr_only_frames + 1 < HR_ONLY_DECISION_WINDOW {
        InitialDecision::Defer
    } else {
        InitialDecision::SwitchToPpi
    }
}

/// Handles one BLE peripheral over its session lifetime: connects, discovers
/// services, runs protocol selection, streams samples, and tears down cleanly
/// (including writing STOP_PPI to a Polar PPG band so its battery isn't drained
/// by an orphaned subscription).
pub struct HeartRateClient {
    device: Peripheral,
}

impl HeartRateClient {
    pub fn new(device: Peripheral) -> Self {
        Self { device }
    }

    /// Connect, run protocol selection, and stream samples until `cancel`
    /// fires or the peripheral disconnects.
    ///
    /// `on_protocol` fires exactly once, after the protocol commits (for PP,
    /// after the PMD CP returns SUCCESS — not before, because a band that
    /// rejects the START_PPI request must surface a clear error rather than
    /// pretending it's streaming).
    ///
    /// `on_sample` fires for every accepted sample.
    pub async fn run<P, S, C>(&self, cancel: C, mut on_protocol: P, mut on_sample: S) -> Result<()>
    where
        P: FnMut(IntervalKind) -> Result<()>,
        S: FnMut(Sample) -> Result<()>,
        C: Future<Output = ()>,
    {
        // Connect + discover -----------------------------------------------
        self.device.connect().await.context("connect")?;
        self.device
            .discover_services()
            .await
            .context("discover services")?;

        let chars = self.device.characteristics();
        let services = self.device.services();

        let hr_char = chars
            .iter()
            .find(|c| c.uuid == ble::HR_MEASUREMENT_UUID)
            .context("Heart Rate Measurement characteristic (0x2A37) not present on device")?
            .clone();
        let pmd_present = services
            .iter()
            .any(|s| s.uuid == ble::POLAR_PMD_SERVICE_UUID);
        let pmd_cp_char = chars
            .iter()
            .find(|c| c.uuid == ble::POLAR_PMD_CP_UUID)
            .cloned();
        let pmd_data_char = chars
            .iter()
            .find(|c| c.uuid == ble::POLAR_PMD_DATA_UUID)
            .cloned();

        if pmd_present {
            println!("Polar PMD service detected; PPI fallback available if no RR seen");
        }

        // Subscribe to standard HR -----------------------------------------
        let mut notifs = self
            .device
            .notifications()
            .await
            .context("get notification stream")?;
        self.device
            .subscribe(&hr_char)
            .await
            .context("subscribe to HR Measurement (0x2A37)")?;

        // Drive the state machine ------------------------------------------
        let mut mode = Mode::Initial { hr_only_frames: 0 };
        let cancel = std::pin::pin!(cancel);
        let mut cancel = cancel.fuse();
        let mut run_result: Result<()> = Ok(());

        loop {
            tokio::select! {
                biased;
                _ = &mut cancel => {
                    println!("Cancel signal received; shutting down BLE session");
                    break;
                }
                next = notifs.next() => {
                    let Some(notif) = next else {
                        println!("Notification stream ended (peripheral disconnected?)");
                        break;
                    };
                    let result = self.handle_notification(
                        &notif,
                        &mut mode,
                        pmd_present,
                        pmd_cp_char.as_ref(),
                        pmd_data_char.as_ref(),
                        &hr_char,
                        &mut on_protocol,
                        &mut on_sample,
                    ).await;
                    if let Err(e) = result {
                        run_result = Err(e);
                        break;
                    }
                }
            }
        }

        // Cleanup ----------------------------------------------------------
        // Best-effort STOP_PPI so the band's battery isn't drained by an
        // orphaned subscription. Failures are non-fatal — we're closing anyway.
        // Only fire when streaming actually started (protocol_announced=true);
        // a band that rejected START_PPI (INVALID_MTU, NOT_SUPPORTED) was never
        // streaming, so STOP_PPI would be a wasted GATT round-trip.
        if matches!(
            mode,
            Mode::PolarPpi {
                protocol_announced: true
            }
        ) {
            if let Some(cp) = pmd_cp_char.as_ref() {
                if let Err(e) = self
                    .device
                    .write(cp, &polar_pmd::STOP_PPI_REQUEST, WriteType::WithResponse)
                    .await
                {
                    eprintln!("STOP_PPI write threw (non-fatal): {e}");
                }
            }
        }
        if let Err(e) = self.device.disconnect().await {
            eprintln!("Disconnect threw (non-fatal): {e}");
        }
        run_result
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_notification<P, S>(
        &self,
        notif: &ValueNotification,
        mode: &mut Mode,
        pmd_present: bool,
        pmd_cp_char: Option<&Characteristic>,
        pmd_data_char: Option<&Characteristic>,
        hr_char: &Characteristic,
        on_protocol: &mut P,
        on_sample: &mut S,
    ) -> Result<()>
    where
        P: FnMut(IntervalKind) -> Result<()>,
        S: FnMut(Sample) -> Result<()>,
    {
        match notif.uuid {
            uuid if uuid == ble::HR_MEASUREMENT_UUID => {
                self.handle_hr_frame(
                    &notif.value,
                    mode,
                    pmd_present,
                    pmd_cp_char,
                    pmd_data_char,
                    hr_char,
                    on_protocol,
                    on_sample,
                )
                .await
            }
            uuid if uuid == ble::POLAR_PMD_DATA_UUID => {
                handle_pmd_data(&notif.value, mode, on_sample)
            }
            uuid if uuid == ble::POLAR_PMD_CP_UUID => {
                handle_pmd_cp(&notif.value, mode, on_protocol)
            }
            _ => Ok(()),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn handle_hr_frame<P, S>(
        &self,
        value: &[u8],
        mode: &mut Mode,
        pmd_present: bool,
        pmd_cp_char: Option<&Characteristic>,
        pmd_data_char: Option<&Characteristic>,
        hr_char: &Characteristic,
        on_protocol: &mut P,
        on_sample: &mut S,
    ) -> Result<()>
    where
        P: FnMut(IntervalKind) -> Result<()>,
        S: FnMut(Sample) -> Result<()>,
    {
        if let Mode::Initial { hr_only_frames } = *mode {
            // `hr_only_frames` is the count of *prior* HR-only frames; the
            // current frame is the (hr_only_frames + 1)-th frame seen on this
            // session. So "RR seen on frame N" reads N = hr_only_frames + 1.
            match decide_initial(value, hr_only_frames, pmd_present) {
                InitialDecision::CommitRr => {
                    *mode = Mode::StandardRr;
                    println!(
                        "Protocol selected: StandardRr (RR seen on frame {})",
                        hr_only_frames + 1
                    );
                    on_protocol(IntervalKind::Rr)?;
                    // fall through to parse-and-emit
                }
                InitialDecision::CommitHrOnly => {
                    *mode = Mode::HrOnly;
                    println!("Protocol selected: HrOnly (no PMD service)");
                    // The kind label is "RR" — that's just what we observed
                    // first (no RR bit seen). Future frames from this band
                    // may still carry RR intervals (the parser handles them
                    // either way); they'll be emitted on the same outlet.
                    on_protocol(IntervalKind::Rr)?;
                    // fall through to parse-and-emit
                }
                InitialDecision::Defer => {
                    *mode = Mode::Initial {
                        hr_only_frames: hr_only_frames + 1,
                    };
                    println!(
                        "Frame {} is HR-only; waiting up to {} for RR before falling back to PMD",
                        hr_only_frames + 1,
                        HR_ONLY_DECISION_WINDOW
                    );
                    return Ok(());
                }
                InitialDecision::SwitchToPpi => {
                    *mode = Mode::PolarPpi {
                        protocol_announced: false,
                    };
                    println!(
                        "Protocol selected: PolarPpi (no RR after {} frames; PMD present)",
                        HR_ONLY_DECISION_WINDOW
                    );
                    let cp = pmd_cp_char
                        .context("PMD CP characteristic missing despite PMD service present")?;
                    let data = pmd_data_char
                        .context("PMD data characteristic missing despite PMD service present")?;
                    self.device
                        .unsubscribe(hr_char)
                        .await
                        .context("unsubscribe HR Measurement before PMD switch")?;
                    self.device
                        .subscribe(cp)
                        .await
                        .context("subscribe to PMD Control Point")?;
                    self.device
                        .subscribe(data)
                        .await
                        .context("subscribe to PMD Data")?;
                    self.device
                        .write(cp, &polar_pmd::START_PPI_REQUEST, WriteType::WithResponse)
                        .await
                        .context("write START_PPI to PMD CP")?;
                    println!("PMD start-PPI request sent; awaiting CP response and data frames");
                    return Ok(());
                }
            }
        }

        // StandardRr or HrOnly: parse the 0x2A37 frame and emit a Sample.
        match heart_rate::parse_heart_rate_measurement(value) {
            Ok(data) => {
                on_sample(Sample {
                    heart_rate_bpm: data.heart_rate,
                    kind: IntervalKind::Rr,
                    intervals_ms: data.rr_intervals,
                })?;
            }
            Err(e) => {
                eprintln!("Failed to parse 0x2A37 notification: {e}");
            }
        }
        Ok(())
    }
}

fn handle_pmd_data<S>(value: &[u8], mode: &Mode, on_sample: &mut S) -> Result<()>
where
    S: FnMut(Sample) -> Result<()>,
{
    // Gate on `protocol_announced: true` (not just any PolarPpi sub-state):
    // PMD data notifications can arrive between `subscribe(data).await` and
    // the CP success indication that calls `on_protocol`. Until on_protocol
    // has fired, downstream consumers (LSL outlets) don't exist yet, so
    // emitting here would force the caller to either panic on the missing
    // outlet or buffer indefinitely. Drop the early frames; the band keeps
    // emitting at >1Hz so we don't lose anything meaningful.
    if !matches!(
        mode,
        Mode::PolarPpi {
            protocol_announced: true
        }
    ) {
        return Ok(());
    }
    let samples = polar_pmd::parse_ppi_frame(value);
    if samples.is_empty() {
        // Either a non-PPI frame on this characteristic (shouldn't happen) or
        // every sample was filtered (no contact / HR=0 / PPI=0). Quiet by
        // default — set HRBAND_LSL_DEBUG_PMD=1 to see hex dumps when chasing
        // contact / signal-quality issues. Mirrors the Android sibling's
        // debug logging at HeartRateClient.kt:282-283.
        if std::env::var_os("HRBAND_LSL_DEBUG_PMD").is_some() {
            let hex: String = value
                .iter()
                .take(16)
                .map(|b| format!("{b:02x}"))
                .collect::<Vec<_>>()
                .join(" ");
            eprintln!("PMD frame produced 0 samples ({}B): {}", value.len(), hex);
        }
        return Ok(());
    }
    for s in samples {
        on_sample(Sample {
            heart_rate_bpm: u16::from(s.heart_rate_bpm),
            kind: IntervalKind::Pp,
            intervals_ms: vec![s.ppi_ms],
        })?;
    }
    Ok(())
}

fn handle_pmd_cp<P>(value: &[u8], mode: &mut Mode, on_protocol: &mut P) -> Result<()>
where
    P: FnMut(IntervalKind) -> Result<()>,
{
    let Some(resp) = polar_pmd::parse_cp_response(value) else {
        return Ok(());
    };
    // ALREADY_IN_STATE (6) means the band is already streaming the requested
    // measurement type — usually because a previous session was killed (e.g.
    // by SIGTERM, OOM, USB unplug) before STOP_PPI could be written. The
    // Polar Sense holds PPI state across reconnects for a while. The state
    // we want IS the state we're in, so treat this as success.
    let effectively_ok = resp.is_success() || resp.error_code == 6;
    if !effectively_ok {
        // The H10 exposes the PMD service but answers START_PPI with
        // NOT_SUPPORTED. INVALID_MTU appears on systems where MTU negotiation
        // didn't reach 232 (Polar's required value for the START_PPI request).
        bail!(
            "PMD {} for op=0x{:02X} type=0x{:02X}",
            resp.error_name(),
            resp.op_code,
            resp.measurement_type
        );
    }
    if let Mode::PolarPpi { protocol_announced } = mode {
        if !*protocol_announced {
            on_protocol(IntervalKind::Pp)?;
            *protocol_announced = true;
            let label = if resp.is_success() {
                "SUCCESS"
            } else {
                "ALREADY_IN_STATE (band was already streaming)"
            };
            println!("PMD CP returned {label}; PPI streaming committed");
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decide_initial_commits_rr_when_rr_bit_set() {
        // flags 0x10 → RR-intervals-present bit set
        let frame = [0x10, 72, 0xE8, 0x03];
        assert_eq!(decide_initial(&frame, 0, true), InitialDecision::CommitRr,);
        // PMD presence is irrelevant when RR is already advertised.
        assert_eq!(decide_initial(&frame, 0, false), InitialDecision::CommitRr,);
    }

    #[test]
    fn decide_initial_commits_hr_only_when_no_rr_and_no_pmd() {
        // flags 0x00 → no RR bit
        let frame = [0x00, 65];
        assert_eq!(
            decide_initial(&frame, 0, false),
            InitialDecision::CommitHrOnly,
        );
    }

    #[test]
    fn decide_initial_defers_first_frames_when_pmd_available() {
        let frame = [0x00, 65];
        // Frames 0 and 1 (0-indexed counter) defer
        assert_eq!(decide_initial(&frame, 0, true), InitialDecision::Defer,);
        assert_eq!(decide_initial(&frame, 1, true), InitialDecision::Defer,);
    }

    #[test]
    fn decide_initial_switches_to_ppi_after_decision_window() {
        let frame = [0x00, 65];
        // hr_only_frames = 2 means we've already seen 2 HR-only frames; this
        // (hypothetical) third one would push us over the window threshold.
        assert_eq!(
            decide_initial(&frame, HR_ONLY_DECISION_WINDOW - 1, true),
            InitialDecision::SwitchToPpi,
        );
    }

    #[test]
    fn decide_initial_handles_empty_frame_as_no_rr() {
        assert_eq!(decide_initial(&[], 0, false), InitialDecision::CommitHrOnly,);
    }

    #[test]
    fn handle_pmd_cp_announces_protocol_on_success_and_only_once() {
        let mut mode = Mode::PolarPpi {
            protocol_announced: false,
        };
        let mut announced = Vec::new();
        let mut on_proto = |k: IntervalKind| {
            announced.push(k);
            Ok(())
        };
        // SUCCESS response: 0xF0 (resp opcode), 0x02 (op=START), 0x03 (type=PPI), 0x00 (SUCCESS)
        let success = [0xF0, 0x02, 0x03, 0x00];
        handle_pmd_cp(&success, &mut mode, &mut on_proto).unwrap();
        // Second invocation must NOT re-announce
        handle_pmd_cp(&success, &mut mode, &mut on_proto).unwrap();
        assert_eq!(announced, vec![IntervalKind::Pp]);
        assert_eq!(
            mode,
            Mode::PolarPpi {
                protocol_announced: true,
            }
        );
    }

    #[test]
    fn handle_pmd_cp_errors_on_invalid_mtu() {
        let mut mode = Mode::PolarPpi {
            protocol_announced: false,
        };
        let mut on_proto = |_| -> Result<()> { Ok(()) };
        // Error response: error_code = 10 (INVALID_MTU)
        let err = [0xF0, 0x02, 0x03, 0x0A];
        let result = handle_pmd_cp(&err, &mut mode, &mut on_proto);
        let msg = format!("{:#}", result.unwrap_err());
        assert!(
            msg.contains("INVALID_MTU"),
            "expected INVALID_MTU in error message, got: {msg}"
        );
    }

    #[test]
    fn handle_pmd_cp_treats_already_in_state_as_success() {
        // If a previous session crashed without sending STOP_PPI, the band can
        // still be in PPI streaming state when we connect again. The CP returns
        // ALREADY_IN_STATE — the state we want is the state we're in, so we
        // commit to PolarPpi mode rather than aborting.
        let mut mode = Mode::PolarPpi {
            protocol_announced: false,
        };
        let mut announced = Vec::new();
        let mut on_proto = |k: IntervalKind| {
            announced.push(k);
            Ok(())
        };
        // Error response: error_code = 6 (ALREADY_IN_STATE)
        let already = [0xF0, 0x02, 0x03, 0x06];
        handle_pmd_cp(&already, &mut mode, &mut on_proto).unwrap();
        assert_eq!(announced, vec![IntervalKind::Pp]);
        assert_eq!(
            mode,
            Mode::PolarPpi {
                protocol_announced: true,
            }
        );
    }

    #[test]
    fn handle_pmd_cp_errors_on_not_supported() {
        // The H10 exposes PMD but answers START_PPI with NOT_SUPPORTED.
        let mut mode = Mode::PolarPpi {
            protocol_announced: false,
        };
        let mut on_proto = |_| -> Result<()> { Ok(()) };
        let err = [0xF0, 0x02, 0x03, 0x03]; // 0x03 = NOT_SUPPORTED
        let result = handle_pmd_cp(&err, &mut mode, &mut on_proto);
        let msg = format!("{:#}", result.unwrap_err());
        assert!(msg.contains("NOT_SUPPORTED"));
    }

    #[test]
    fn handle_pmd_cp_ignores_non_response_frames() {
        let mut mode = Mode::PolarPpi {
            protocol_announced: false,
        };
        let mut on_proto = |_| -> Result<()> { Ok(()) };
        let nonsense = [0x00, 0x00, 0x00, 0x00];
        // Should not error or announce.
        handle_pmd_cp(&nonsense, &mut mode, &mut on_proto).unwrap();
        assert_eq!(
            mode,
            Mode::PolarPpi {
                protocol_announced: false,
            }
        );
    }

    #[test]
    fn handle_pmd_data_emits_pp_samples() {
        let mode = Mode::PolarPpi {
            protocol_announced: true,
        };
        let mut samples = Vec::new();
        let mut on_sample = |s: Sample| {
            samples.push(s);
            Ok(())
        };
        // PMD frame: header + one valid sample (HR=72, PPI=850 ms, contact ok)
        let mut frame = vec![0x03];
        frame.extend_from_slice(&[0u8; 8]); // timestamp
        frame.push(0); // frame format
        frame.extend_from_slice(&[72, 0x52, 0x03, 0x0A, 0x00, 0x02]);
        handle_pmd_data(&frame, &mode, &mut on_sample).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].heart_rate_bpm, 72u16);
        assert_eq!(samples[0].kind, IntervalKind::Pp);
        assert_eq!(samples[0].intervals_ms, vec![850u16]);
    }

    #[test]
    fn handle_pmd_data_does_nothing_before_protocol_announced() {
        // Race window: PMD data notifications can in principle arrive between
        // `subscribe(data).await` and the CP success indication that calls
        // `on_protocol`. Until protocol is announced, downstream LSL outlets
        // don't exist yet — emitting samples here would force lib.rs to either
        // panic on the missing outlet or buffer for an unbounded window. Drop
        // the early frames; the band keeps emitting at >1Hz so we don't lose
        // anything meaningful.
        let mode = Mode::PolarPpi {
            protocol_announced: false,
        };
        let mut samples: Vec<Sample> = Vec::new();
        let mut on_sample = |s: Sample| {
            samples.push(s);
            Ok(())
        };
        let mut frame = vec![0x03];
        frame.extend_from_slice(&[0u8; 8]);
        frame.push(0);
        frame.extend_from_slice(&[72, 0x52, 0x03, 0x0A, 0x00, 0x02]);
        handle_pmd_data(&frame, &mode, &mut on_sample).unwrap();
        assert!(
            samples.is_empty(),
            "should drop PMD data before protocol announce"
        );
    }

    #[test]
    fn handle_pmd_data_does_nothing_if_not_in_ppi_mode() {
        let mode = Mode::StandardRr;
        let mut samples: Vec<Sample> = Vec::new();
        let _ = &mut samples;
        let mut on_sample = |s: Sample| {
            samples.push(s);
            Ok(())
        };
        let mut frame = vec![0x03];
        frame.extend_from_slice(&[0u8; 8]);
        frame.push(0);
        frame.extend_from_slice(&[72, 0x52, 0x03, 0x0A, 0x00, 0x02]);
        handle_pmd_data(&frame, &mode, &mut on_sample).unwrap();
        assert!(samples.is_empty());
    }
}
