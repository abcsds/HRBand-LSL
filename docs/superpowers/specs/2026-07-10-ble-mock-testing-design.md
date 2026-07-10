# BLE mock testing for `HeartRateClient` + CI

**Date:** 2026-07-10
**Status:** Superseded — see Resolution below

## Resolution (2026-07-10)

After reviewing this design against its actual purpose (HRBand-LSL is personal
research infra, not a product with ongoing external contributions), the
`FakePeripheral`/generic-trait/CI design below was **not implemented**. Two
things drove the reversal:

1. **It didn't answer the question that motivated it.** The design was
   proposed in response to a real scare (btleplug 0.11→0.12: a manual test
   run failed with "Service discovery timed out" before a retry succeeded).
   But `FakePeripheral` tests *our* protocol/state-machine logic against a
   scripted fake — it never exercises btleplug's real BlueZ/dbus
   implementation, so it cannot catch a library-version regression in actual
   BLE behavior. Nothing that runs in CI can: GitHub-hosted runners have no
   Bluetooth radio. The only thing that ever answers "did the library update
   break real BLE" is running the real binary against real hardware — which
   is what already happened, manually, on 2026-07-08.
2. **The maintenance cadence doesn't need it.** Dependencies will be updated
   on a semesterly cadence, each update tagged, with a manual hardware smoke
   test (build + `cargo test` + run against a real band) before tagging —
   same process already exercised for the 2026-07-08 update. That's
   infrequent enough that the ~150+ lines of generic-trait/mock/CI machinery
   below isn't worth the ongoing maintenance for a single-user tool.

**What was actually done instead** (see the companion commit): removed the
misleading empty stub tests in `tests/ble_test.rs` (they asserted nothing
about BLE behavior despite the name), and added `proptest`-based
panic-safety tests for the two BLE frame parsers
(`parse_heart_rate_measurement`, `parse_ppi_frame`/`parse_cp_response`).
Those parsers already looked bounds-checked on inspection, so this isn't a
bug fix — it's cheap insurance (one dev-dependency, ~20 lines) against a
future edit accidentally introducing a panic that could crash a live
recording session on a single corrupted BLE frame. That failure mode is
real and CI-testable without any hardware or mocking; a version-compat
regression in btleplug is not.

The design below is kept for the record — the reasoning behind the
generic-`Peripheral`-trait approach and the CI workflow shape may be useful
if this project ever grows external contributors or a CI need re-emerges.

---

## Original design (not implemented)

## Problem

`tests/ble_test.rs` contains two stub tests that only assert the module compiles — no BLE behavior is actually exercised. The real BLE orchestration logic lives in `src/client.rs`'s `HeartRateClient::run()`: connect → discover services → subscribe to HR Measurement → drive the protocol-decision state machine (standard RR vs. Polar PPI fallback) → clean up (STOP_PPI + disconnect). This is the highest-risk, most complex BLE surface in the codebase, and it is currently verified only by manual runs against real hardware.

That gap was felt directly during the 2026-07-08 dependency update (btleplug 0.11→0.12): the first manual hardware run failed with "Service discovery timed out." The retry succeeded, and the failure was provisionally attributed to BLE flakiness, but nothing in the test suite could confirm or rule out a regression — the only signal was a human watching a terminal with a physical Polar H10 nearby.

## Goals

- Add unit-level test coverage for `HeartRateClient::run()`'s full orchestration: connect/discover error paths, protocol-decision branching, the exact BLE call sequence during the PPI fallback (unsubscribe/subscribe/subscribe/write ordering), and cleanup behavior (STOP_PPI only when appropriate, disconnect always).
- Make this coverage run without any real BLE hardware or radio, so it works unattended in CI.
- Wire up GitHub Actions CI so this (and the existing test suite) runs automatically on every push/PR — there is currently no CI in this repo at all.

## Non-goals

- Mocking `BleDeviceManager` (scanning/filtering in `src/ble.rs`). Lower risk, simpler logic; deferred as a follow-up if desired.
- Any hardware-in-the-loop / self-hosted-runner testing against a real band.
- Fixing the ~20 pre-existing clippy warnings mentioned in `flake.nix` — CI intentionally does not gate on clippy, matching existing project decisions.
- Diagnosing the specific "Service discovery timed out" incident's root cause (BlueZ session state vs. a real regression) — the new `run_surfaces_discover_services_error_with_context` test guards against *future* regressions in error propagation, it does not retroactively diagnose that incident.

## Design

### Architecture

`HeartRateClient` becomes generic over btleplug's own `Peripheral` trait, which is already the exact abstraction `client.rs` calls through today (it just happens to be pinned to the concrete `btleplug::platform::Peripheral` type):

```rust
pub struct HeartRateClient<P: btleplug::api::Peripheral> {
    device: P,
}

impl<P: btleplug::api::Peripheral> HeartRateClient<P> {
    pub fn new(device: P) -> Self { ... }
    pub async fn run<F, S, C>(&self, cancel: C, on_protocol: F, on_sample: S) -> Result<()> { ... }
    // handle_notification, handle_hr_frame gain the same P bound
}
```

This is a pure type-parameterization — no logic changes. `btleplug::platform::Peripheral` already implements `btleplug::api::Peripheral` (that's how the current code compiles today via `use btleplug::api::Peripheral as _`), so the real production path needs zero adapter/glue code.

Call-site impact: the only place `HeartRateClient::new` is invoked is `src/lib.rs:319`, inside `connect_and_stream(device: btleplug::platform::Peripheral, ...)`. Since that argument is already a concrete `btleplug::platform::Peripheral`, Rust's generic inference resolves `P` automatically — **no changes needed outside `client.rs`.**

Rejected alternatives:
- A hand-rolled narrow trait (only the ~9 methods `client.rs` uses) implemented for both the real peripheral and a fake: more decoupled, but requires writing and maintaining an adapter layer that duplicates what btleplug's own trait already provides, and doesn't automatically flag itself when btleplug's trait shape changes.
- Real software-emulated BLE via BlueZ virtual controllers (`hci_vhci`): most faithful to production, but heavy, Linux-only, fiddly to run reliably in CI, and produces coarse black-box results instead of unit-level call-ordering assertions.

### `FakePeripheral` test double

Implements `btleplug::api::Peripheral` directly (satisfying `Send + Sync + Clone + Debug`). Lives in `src/client.rs`'s `#[cfg(test)] mod tests`.

```rust
struct FakePeripheral {
    inner: Arc<Mutex<FakeState>>,
}

struct FakeState {
    services: BTreeSet<Service>,           // pre-seeded HR / PMD services+characteristics per test
    connect_result: Option<String>,         // None = succeed; Some(msg) = fail with msg
    discover_result: Option<String>,
    calls: Vec<RecordedCall>,               // ordered log, for sequence assertions
    notif_rx: Option<mpsc::UnboundedReceiver<ValueNotification>>,
}

enum RecordedCall {
    Connect,
    Disconnect,
    DiscoverServices,
    Subscribe(Uuid),
    Unsubscribe(Uuid),
    Write(Uuid, Vec<u8>, WriteType),
}
```

`FakePeripheral::new(services) -> (FakePeripheral, FakeHandle)`. `FakeHandle` is the test-side control surface:
- `push_notification(uuid, bytes)` — feed a `ValueNotification` into the stream `notifications()` returns.
- `calls() -> Vec<RecordedCall>` — snapshot the call log for assertions.
- `fail_connect(msg)` / `fail_discover_services(msg)` — configure error injection before the test drives `run()`.

Tests spawn `client.run(cancel, on_protocol, on_sample)` as a `tokio::task`, drive it through the `FakeHandle`, fire `cancel` (or drop the notification sender to simulate a peripheral disconnect), then assert on the recorded call sequence and the samples/protocol events collected via the `on_sample`/`on_protocol` closures (which write into shared state the test can inspect afterward).

### Test scenarios (new, in `client.rs`'s test module)

1. `run_commits_standard_rr_and_emits_samples` — RR-bit frame → `on_protocol(Rr)` fires once, sample emitted, no PMD calls.
2. `run_falls_back_to_ppi_after_decision_window` — HR-only frames with PMD present → asserts the exact call order `unsubscribe(hr) → subscribe(cp) → subscribe(data) → write(START_PPI)`; then a CP SUCCESS response → `on_protocol(Pp)` fires once; a subsequent PMD data frame emits a `Pp` sample.
3. `run_writes_stop_ppi_on_cancel_after_protocol_announced` — reach `PolarPpi{announced:true}`, fire cancel, assert STOP_PPI was written before disconnect.
4. `run_skips_stop_ppi_if_protocol_never_announced` — cancel before the CP responds, assert STOP_PPI is *not* written, disconnect still happens.
5. `run_ends_cleanly_when_notification_stream_closes` — drop the notification sender (simulated peripheral disconnect) → `run()` returns `Ok(())`, disconnect called.
6. `run_surfaces_discover_services_error_with_context` — inject a discover-services failure → assert the error message contains "discover services". Direct regression guard for the "Service discovery timed out" class of failure seen during manual testing.
7. `run_surfaces_connect_error_with_context` — same, for `connect()`.
8. `run_errors_when_hr_measurement_characteristic_missing` — services without the 0x2A37 characteristic → assert the error names the missing characteristic.

### CI workflow

`flake.nix` already defines a `checks.build` output (`buildRustPackage` with `doCheck = true` by default) and a `checks.fmt` output (`cargo fmt --check`). This means `nix flake check` already runs the full `cargo test` suite — including the new mock tests — for free. No new Nix wiring is needed.

New file, `.github/workflows/ci.yml`:

```yaml
name: CI
on:
  push:
    branches: [main]
  pull_request:

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: DeterminateSystems/nix-installer-action@main
      - uses: DeterminateSystems/magic-nix-cache-action@main
      - run: nix flake check -L
```

- `nix-installer-action` gets Nix onto the stock Ubuntu runner. No Bluetooth/dbus hardware setup is needed — `FakePeripheral` tests never touch real BLE.
- `magic-nix-cache-action` caches the Nix store via GitHub's built-in Actions cache (no secrets required); matters because `liblsl` is a C++ dependency built from source on every uncached run.
- `nix flake check -L` runs both `checks.build` (release build + `cargo test`) and `checks.fmt`. `-L` prints full build logs on failure.
- Clippy stays excluded from CI, matching the existing `flake.nix` comment about ~20 pre-existing warnings — out of scope for this change.

### Error handling

No new error-handling logic is introduced. `client.rs` already produces descriptive errors via `.context(...)` on `connect()`, `discover_services()`, etc. The new tests (#6, #7 above) verify those existing messages actually surface correctly through `run()` — closing the gap that let the manual-testing incident go unverified by anything but a human watching a live BLE session.

### Testing plan

Purely additive: no existing test changes, no behavior changes to `HeartRateClient`. New tests live in `client.rs`'s existing `#[cfg(test)] mod tests` block alongside the current `decide_initial`/`handle_pmd_cp`/`handle_pmd_data` tests. `cargo test` and `nix flake check` pick them up automatically. Total new surface: `FakePeripheral` + `FakeHandle` (~80-120 lines) + 8 test functions + one CI workflow file.

## Open questions / follow-ups

- Mocking `BleDeviceManager` (scan/filter) is deferred; revisit if scan-side regressions become a recurring pain point.
- If the "Service discovery timed out" incident recurs on real hardware, capture BlueZ/dbus logs to correlate against btleplug 0.12.0's changelog entry for a BlueZ disconnection bugfix (noted in the independent code review of the 2026-07-08 dependency-update commit).
