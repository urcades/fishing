# Fishing

Pure, bounded state machines and coupled dynamics for casual fishing minigames.
**Draft 0.2, schema revision 2.** Six Rust source files, no default dependencies,
no unsafe code, no clock, renderer, runtime, or hidden randomness.

```toml
[dependencies]
fishing = "0.2.0"
```

```rust
use fishing::{create_state, step, Config, Input, Mode, Parameters, Phase};
use fishing::geometry::Capture;

let rules = Config {
    mode: Mode::Tracking,
    dimensions: 1,
    parameters: Parameters::default(),
    capture: Capture::Rectangle,
    ..Config::default()
};
let state = create_state(42, &rules)?;
let next = step(&state, Input { primary: 1.0, steer: 0.0 }, &rules)?;
assert_eq!(next.state.phase, Phase::Waiting);
assert_eq!(state.phase, Phase::Ready); // The input was not mutated.
# Ok::<(), String>(())
```

The host calls `step` once per simulated 1/60 second and renders as often as it
likes. Keep the resolved configuration fixed during an encounter; persist it
with the state to resume a run. Replay uses that configuration, the initial seed,
and the same input sequence.

The simulation API is `Config`, `State`, `Input`, `create_state`, `step`, and
`observe`. Construct `Config` directly when integrating your own game rules.

| Layer | API | Responsibility |
| --- | --- | --- |
| Simulation | `create_state`, `step`, `observe` | Execute an encounter from explicit rules, input and state. |
| Optional authoring | `Fish`, `Rod`, `Bait`, `Definition`, `resolve`, `select` | One convenient profile model, including specific equipment and bait formulas. |

The authoring helpers are not required for protocol conformance. Their ownership
choices (for example, rods supplying window size) are conventions, not universal
fishing rules. A game may derive that same window size from skill, equipment,
accessibility settings, or any combination, then pass the result to `Config`.

World geography, casting aim, species availability, rarity, inventory, prices,
and XP belong to the game. Resolve their mechanical effects before creating an
encounter. Afterward, interpret the outcome and observations using the game's
reward rules. Any new rule that changes the next simulated state must instead
be explicitly represented in the protocol; hidden host callbacks break replay.

Performance statistics can be a separate pure fold over the run. Resource scoring
uses **pre-step** alignment: accumulate `observe(&state, &config)` only when the
old state is in struggle and that tick advances. Include the tick that ends the
struggle; exclude ready no-ops and terminal calls. What counts as a perfect catch
and what it earns are game decisions. See the executable
[game-boundary example](https://github.com/urcades/fishing-examples/blob/main/examples/game_rules.rs).

The lifecycle is ready → waiting → bite → optional struggle → caught/escaped.
Hook-only, pressure/release, 1D tracking and 2D tracking share that lifecycle.
During struggle, progress, tension and energy influence one another; spatial
modes couple them to capture overlap. Polygon capture supports one optional
hole. Terminal states absorb further input. Encounters default to 3,600 advancing ticks, with a configurable
limit up to 36,000; waiting at ready consumes none.

Fish can use a repeating `Vec<Segment>` (at most 16): each segment specifies
its behavior label, duration/jitter, pull intensity, speed multiplier and target
rule (`Keep`, `Hold`, `Wander`, `Point`, or `Opposite`). An empty pattern preserves
the original rest/warning/surge cycle. Optional `Nibbles` add false bites before
the real hook window. Both extensions keep all execution state in the snapshot.

Numerical validation enforces broad finite bounds; `Parameters::validate_recommended`
optionally checks the original demo tuning ranges. Loadout resolution retains
classic timing by default; select `TimingPolicy::Configured` to use the definition's
wait range and broader bite durations.

**Migrating from 0.1:** schema-1 snapshots are rejected. For an original recipe,
explicitly set the snapshot version to 2 and initialize `segmentIndex` and
`nibblesLeft` to zero. Existing JSON configs receive disabled extension defaults.
Rust struct literals need the new fields (`..Config::default()` is convenient).
See the specification for profile additions and exact compatibility boundaries.

Enable `features = ["serde"]` for serialization of all public data records.
Deserialization checks wire structure; call the validation methods or public
simulation functions to check numeric and semantic constraints. The default
build needs only the Rust standard library. This is not a `no_std` crate.

[The specification](https://github.com/urcades/fishing/blob/main/spec/PROTOCOL.md)
defines update ordering, equations, numeric bounds, RNG and terminal precedence.
An independently implemented [TypeScript port](https://github.com/urcades/fishing-system)
pins the protocol sources and checks against this published crate.
Floating-point conformance requires exact discrete state/events and a 1e−12
absolute tolerance at numerical checkpoints; it does not promise universal
bit-identical results across languages or architectures.

The [fishing-examples repository](https://github.com/urcades/fishing-examples)
consumes this crate. It owns the four playable demos, WebAssembly/JSON adapters,
CLI, profile catalogs, Python port and wire schemas. A representative conformance
corpus ships with this crate; the examples retain the full historical corpus and
check both ports against it.

Run `cargo test`, `cargo test --all-features`, and
`cargo clippy --all-targets --all-features -- -D warnings` to check the library.
Run `cargo doc --no-deps --all-features` for the field-level API reference.
The examples repository runs the cross-language fixtures and gameplay regressions.

Licensed under MIT OR Apache-2.0, at your option.
