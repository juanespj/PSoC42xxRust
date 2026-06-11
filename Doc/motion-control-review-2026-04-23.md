# Motion Control Review Context (2026-04-23)

Scope reviewed:
- `PSoC42rs.cydsn/rust_project/src/motor.rs`
- `PSoC42rs.cydsn/rust_project/src/encoder.rs`
- Integration context in `rust_project/src/lib.rs`, `sys.rs`, `serial.rs`
- Shared encoder logic in `rs-embedded/rs-core/src/encoder_core.rs`

## Key Findings

1. Concurrent mutation risk (ISR + foreground)
- `Pulser_InterruptHandler` mutates `Xaxis` in interrupt context while main-loop code mutates the same object via `Xaxis.get_mut()`.
- Without explicit critical-section discipline, this can cause inconsistent state transitions and data races.

2. Speed/state logic inconsistencies
- `curr_target_speed_hz` is compared against `I16F16::ZERO` even though speed fields are `i64`.
- Step interval behavior at zero speed is contradictory (`speed == 0` maps to a fast interval path), which can produce unintended pulse behavior.

3. Start path is inconsistent between move modes
- `START_SPD` sets `curr_target_speed_hz` before acceleration.
- `START_MOVE` does not always load `curr_target_speed_hz`, so it can accelerate toward stale values depending on prior state.

4. Encoder timing model mismatch
- `encoder_core` assumes fixed `DT_US`.
- Actual `encoder.update()` cadence is task-scheduled and jittered in the main loop, so derived velocity/acceleration scale can drift with load.

5. Position-mode semantics incomplete
- `target_pos_steps` and `curr_pos_steps` exist but are not used to drive stopping/deceleration logic in `run()`.
- Current behavior is effectively speed mode in both pathways.

6. Telemetry formatting issue for signed 64-bit data
- UART helper for `i64` prints concatenated 32-bit chunks, not true signed decimal conversion.
- This can make tuning/debug output misleading.

## Recommended Priority

P0 (Safety/Correctness):
- Eliminate shared mutable access races between ISR and foreground.
- Normalize speed typing and idle/zero-speed transition behavior.

P1 (Control Behavior):
- Unify motion start semantics (`START_MOVE` and `START_SPD`) through one target-setup function.
- Make encoder update timing deterministic or dt-aware.

P2 (Feature Completeness/Diagnostics):
- Implement real position profile control (or remove unused fields until implemented).
- Fix telemetry conversion utilities and test coverage around edge cases.

## Concrete Next Patch Set

Patch A: Concurrency hardening
- Restrict all `Stepper` field mutations to one context or wrap shared mutations in short critical sections.
- Keep ISR lean: pulse output and timebase only.

Patch B: Speed/state cleanup
- Replace mixed fixed/integer zero checks with integer constants.
- Ensure zero command leads to deterministic `IDLE` + no step pulse generation.

Patch C: Start behavior unification
- Add helper that computes signed target and sets `curr_target_speed_hz`.
- Call helper from both `START_MOVE` and `START_SPD`.

Patch D: Encoder timing consistency
- Option 1: run encoder sampling/update from fixed periodic ISR.
- Option 2: pass measured `dt_us` into update and scale equations accordingly.

Patch E: Position mode
- Add stop-distance check (`v^2 / 2a`) and state transition to `DECEL` near target.
- Use encoder position feedback for move completion.

Patch F: Tests
- Add deterministic tests for:
  - accel/decel clamping and IDLE transitions,
  - direction reversal while moving,
  - no-pulse-at-zero-speed,
  - start behavior for both start commands,
  - encoder update stability under variable dt.
