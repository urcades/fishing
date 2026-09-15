# Fishing Protocol

A Rust reference implementation of a small, pure fishing protocol, with a playable browser host and an independent Python port. **Draft 0.1, schema revision 1.** Native ports are checked against shared fixtures; the protocol is not tied to Rust, WebAssembly, a renderer, or a game engine.

The four examples explore reaction-only hooking, pressure/release, vertical tracking, and two-axis geometric capture. Fish, rod, bait and capture geometry are authored as data. The library contains no named game examples, fish species, equipment catalog, SVG paths or browser APIs.

## Start here

- [Protocol specification](spec/PROTOCOL.md): semantics, ordering, equations, RNG, arithmetic, validation and bounds.
- [JSON schemas](spec/fishing.schema.json) and [parameter defaults/bounds](spec/parameters.json).
- [Four documented examples](docs/examples/README.md), each with a definition and loadout JSON.
- [Rust library](src/lib.rs), [state types](src/types.rs), [dynamics](src/dynamics.rs), [geometry](src/geometry.rs), [profile resolution](src/loadout.rs).
- [Independent Python port](ports/python/fishing.py) and [conformance guide](conformance/README.md).

## Play the demo

The included WebAssembly binary was compiled from this Rust source:

```sh
git clone https://github.com/urcades/fishing.git
cd fishing
python3 -m http.server 8057 --bind 127.0.0.1
```

Open http://127.0.0.1:8057/ (redirects to `/playground/`). The page is static; it requires no backend service or package installation. After Rust changes, rebuild WASM before reloading:

```sh
rustup target add wasm32-unknown-unknown  # one-time toolchain setup
sh scripts/build-wasm.sh
```

The browser uses the **same Rust code** as native hosts. JavaScript maps inputs, renders, records/replays, and translates the previous demo's presentation fields. There is no duplicate JavaScript implementation of the dynamics. The inspector now shows the canonical protocol state/config.

## Use Rust directly

Add the local crate as a path dependency:

```toml
[dependencies]
fishing-protocol = { path = "path/to/fishing" }
```

```rust
use fishing_protocol::{create_state, step, Config, Input, Mode, Parameters};
use fishing_protocol::geometry::Capture;

let config = Config {
    mode: Mode::Tracking,
    dimensions: 1,
    parameters: Parameters::default(),
    capture: Capture::Rectangle,
};
let state = create_state(42, &config)?;
let result = step(&state, Input { primary: 1.0, steer: 0.0 }, &config)?;
// result.state.phase == Phase::Waiting; result.events == [Event::Cast]
// In a host function returning Result<_, String>.
```

For profiles, call `resolve(&definition, &loadout, seed)`, then `create_state(encounter.seed, &encounter.config)`. For a pond draw, call `select` first and carry its returned seed into resolution. Hosts own the clock; call step once per 1/60 simulated second. Rendering need not run at that frequency.

Run the headless example host against any documented recipe:

```sh
cargo run --locked --example play -- one-good-bite
cargo run --locked --example play -- open-water
```

A line-delimited JSON host is also available:

```sh
cargo run --locked --bin fishing-json
```

Supply one request per line, for example:

```json
{"op":"create","seed":42,"config":{"mode":"hook","dimensions":0,"capture":{"kind":"rectangle"}}}
```

The response is `{"ok": <state>}` or `{"error": "diagnostic"}`. Operations are `config`, `create`, `step`, `observe`, `resolve`, `select`, `random`, `capture`, `validate_state`. The 64 KiB transport bound applies per request. Native typed calls avoid JSON overhead.

## Verify

```sh
sh scripts/check.sh
```

This checks Rust formatting, runs Clippy, tests native Rust, checks Python against fixtures, rebuilds WASM, and runs the original 32 gameplay regression tests plus WASM conformance. `cargo fmt --check` checks formatting without modifying it. Dependencies are locked (`serde`, `serde_json` and their transitive build dependencies); no JavaScript packages or Python packages are needed. Rust dependencies can be fetched by Cargo if absent; the verified local build used cached crates.

The fixtures capture 12 accepted demo traces / 8,080 advancing inputs, eight transition edge cases, 30 geometric overlaps, 24 profile resolutions and 12 weighted selections. Every event and sampled numerical checkpoint is checked. Historical regression tests additionally check exact complete-trajectory hashes for the original spatial and pressure games, catchability for 192 fish/rod/bait/style combinations, all six capture shapes, bounds and replay.

Conformance uses exact discrete state/events and 1e−12 absolute tolerance for numeric checkpoints. This is evidence from a bounded suite, not exhaustive equivalence or universal bit-identical replay. Shape/loadout balancing and future mechanics remain exploratory. The rectangle still contains all other shapes at equal bounding size.

## Layout

```text
src/                    Canonical Rust implementation, native JSON transport
spec/                   Language-independent contract and schemas
conformance/fixtures/   Checked-in expected results from the accepted demo
ports/python/           Independent native implementation
docs/examples/          Four recipes, profile/geometry data, documentation
examples/play.rs        Minimal headless host using the public Rust API
playground/             Browser host, WASM binary and presentation adapters
scripts/                Rebuild and verification commands
```
