//! Wire types and validation. Profiles and example names do not live here.
use crate::geometry::{validate_capture, Capture};
use crate::program::{Nibbles, Segment};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
/// Validated core result. Error strings are diagnostics, not part of the wire contract.
pub type Result<T> = std::result::Result<T, String>;
/// State schema revision, independent of the crate version.
pub const VERSION: u32 = 2;
/// Fixed simulation frequency in ticks per second.
pub const HZ: u32 = 60;
/// Duration of one simulation tick in seconds.
pub const DT: f64 = 1.0 / 60.0;
/// Absolute tick ceiling (600 simulated seconds); each config sets its own lower limit.
pub const MAX_TICKS: u32 = 36_000;
/// Half-width of the normalized fish capture interval/square.
pub const FISH_RADIUS: f64 = 0.03;
/// Mechanism used during an encounter; combine with [`Config::dimensions`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Mode {
    /// A fresh press during the bite lands the fish immediately.
    Hook,
    /// Reel/release directly controls the resource dynamics; no spatial state.
    Pressure,
    /// Spatial overlap drives reeling and resource dynamics.
    Tracking,
}
/// Lifecycle phase. Caught and escaped are absorbing terminal states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Phase {
    /// Idle until a fresh primary press casts.
    Ready,
    /// Cast is waiting for a bite.
    Waiting,
    /// A fresh primary press can hook before the deadline.
    Bite,
    /// False bite: wait for it to end; a fresh hook here loses the fish.
    Nibble,
    /// Active pressure or tracking loop.
    Struggle,
    /// Fish landed; further input leaves the state unchanged.
    Caught,
    /// Encounter lost; further input leaves the state unchanged.
    Escaped,
}
/// Fish effort cycle during struggle: rest, warning, then surge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Behavior {
    /// Fish recovers with reduced pull and slower movement.
    Rest,
    /// Telegraph before a surge; spatial fish decelerate.
    Warning,
    /// Fish pulls harder and moves faster.
    Surge,
}
/// Why an encounter ended; available in [`State::reason`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Reason {
    /// Hook deadline expired.
    MissedBite,
    /// A fresh hook was attempted during a nibble.
    EarlyHook,
    /// Raw strain reached or exceeded 1.
    LineBroke,
    /// Raw progress reached or fell below 0.
    GotAway,
    /// Fish successfully caught.
    Landed,
    /// Encounter reached the advancing-tick limit.
    Timeout,
}
/// Ordered transition notifications. Hosts decide how to present them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Event {
    /// Cast entered the waiting phase.
    Cast,
    /// A fresh primary press can hook before the deadline.
    Bite,
    /// False bite: wait for it to end; a fresh hook here loses the fish.
    Nibble,
    /// A fresh hook input was accepted.
    Hooked,
    /// Fish recovers with reduced pull and slower movement.
    Rest,
    /// Telegraph before a surge; spatial fish decelerate.
    Warning,
    /// Fish pulls harder and moves faster.
    Surge,
    /// Fish landed; further input leaves the state unchanged.
    Caught,
    /// Encounter lost; further input leaves the state unchanged.
    Escaped,
}
/// Validated tuning in normalized space and seconds. See each field for its inclusive range.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(default, deny_unknown_fields, rename_all = "camelCase")
)]
pub struct Parameters {
    /// Fish pull multiplier, 0..=100
    pub strength: f64,
    /// Surge duration in seconds before jitter, DT..=600
    pub surge: f64,
    /// Rest duration in seconds before jitter, DT..=600
    pub rest: f64,
    /// Energy loss coefficient per second, 0..=60
    pub fatigue: f64,
    /// Maximum progress gain per second, 0..=60
    pub reel_rate: f64,
    /// Progress loss coefficient per second, 0..=60
    pub escape_rate: f64,
    /// Baseline pull while reeling or aligned, 0..=100
    pub base_tension: f64,
    /// Tension response time in seconds, DT..=600
    pub response: f64,
    /// Energy recovery coefficient per second, 0..=60
    pub recovery: f64,
    /// Warning duration in seconds before jitter, DT..=600
    pub warning: f64,
    /// Hook opportunity in seconds, DT..=600; expiry takes precedence over input.
    pub bite_window: f64,
    /// Fractional duration variation, 0..=1
    pub jitter: f64,
    /// Fish speed scale in normalized units per second, 0..=60
    pub fish_speed: f64,
    /// Tackle window side length in normalized units, 0.001..=1
    pub window_size: f64,
    /// Divisor converting pull into normalized strain, 0.01..=100
    pub line_capacity: f64,
    /// Control acceleration in normalized units per second squared, 0..=3600
    pub tackle_acceleration: f64,
    /// Velocity damping coefficient per second, 0..=60
    pub tackle_damping: f64,
    /// Maximum tackle speed in normalized units per second, 0..=60
    pub tackle_speed: f64,
    /// Minimum wait in seconds, DT..=600; must not exceed `wait_max`.
    pub wait_min: f64,
    /// Maximum wait in seconds, DT..=600
    pub wait_max: f64,
    /// Preferred vertical target center, 0..=1
    pub target_center: f64,
    /// Vertical target distribution scale, 0..=1
    pub target_spread: f64,
    /// Vertical target wander during rest, 0..=1
    pub rest_wander: f64,
}
impl Default for Parameters {
    fn default() -> Self {
        Self {
            strength: 1.0,
            surge: 1.8,
            rest: 1.2,
            fatigue: 0.1,
            reel_rate: 0.08,
            escape_rate: 0.08,
            base_tension: 0.3,
            response: 0.8,
            recovery: 0.2,
            warning: 0.5,
            bite_window: 1.1,
            jitter: 0.15,
            fish_speed: 0.34,
            window_size: 0.28,
            line_capacity: 1.0,
            tackle_acceleration: 3.2,
            tackle_damping: 3.8,
            tackle_speed: 0.9,
            wait_min: 1.5,
            wait_max: 3.0,
            target_center: 0.5,
            target_spread: 1.0,
            rest_wander: 0.16,
        }
    }
}
impl Parameters {
    /// Check finite values, inclusive bounds and cross-field constraints.
    /// Returns a diagnostic error without modifying the value.
    pub fn validate(&self) -> Result<()> {
        bounded(self.strength, 0.0, 100.0, "strength")?;
        bounded(self.surge, DT, 600.0, "surge")?;
        bounded(self.rest, DT, 600.0, "rest")?;
        bounded(self.fatigue, 0.0, 60.0, "fatigue")?;
        bounded(self.reel_rate, 0.0, 60.0, "reelRate")?;
        bounded(self.escape_rate, 0.0, 60.0, "escapeRate")?;
        bounded(self.base_tension, 0.0, 100.0, "baseTension")?;
        bounded(self.response, DT, 600.0, "response")?;
        bounded(self.recovery, 0.0, 60.0, "recovery")?;
        bounded(self.warning, DT, 600.0, "warning")?;
        bounded(self.bite_window, DT, 600.0, "biteWindow")?;
        bounded(self.jitter, 0.0, 1.0, "jitter")?;
        bounded(self.fish_speed, 0.0, 60.0, "fishSpeed")?;
        bounded(self.window_size, 0.001, 1.0, "windowSize")?;
        bounded(self.line_capacity, 0.01, 100.0, "lineCapacity")?;
        bounded(self.tackle_acceleration, 0.0, 3600.0, "tackleAcceleration")?;
        bounded(self.tackle_damping, 0.0, 60.0, "tackleDamping")?;
        bounded(self.tackle_speed, 0.0, 60.0, "tackleSpeed")?;
        bounded(self.wait_min, DT, 600.0, "waitMin")?;
        bounded(self.wait_max, DT, 600.0, "waitMax")?;
        bounded(self.target_center, 0.0, 1.0, "targetCenter")?;
        bounded(self.target_spread, 0.0, 1.0, "targetSpread")?;
        bounded(self.rest_wander, 0.0, 1.0, "restWander")?;
        if self.wait_min > self.wait_max {
            return Err("waitMin exceeds waitMax".into());
        }
        Ok(())
    }
    /// Check the original prototype tuning ranges after structural validation.
    /// This is an optional authoring recommendation, never required by step.
    pub fn validate_recommended(&self) -> Result<()> {
        bounded(self.strength, 0.4, 1.6, "strength")?;
        bounded(self.surge, 0.5, 3.0, "surge")?;
        bounded(self.rest, 0.4, 3.0, "rest")?;
        bounded(self.fatigue, 0.03, 0.25, "fatigue")?;
        bounded(self.reel_rate, 0.02, 0.2, "reelRate")?;
        bounded(self.escape_rate, 0.02, 0.2, "escapeRate")?;
        bounded(self.base_tension, 0.1, 0.5, "baseTension")?;
        bounded(self.response, 0.3, 2.0, "response")?;
        bounded(self.recovery, 0.05, 0.4, "recovery")?;
        bounded(self.warning, 0.3, 1.0, "warning")?;
        bounded(self.bite_window, 0.5, 2.0, "biteWindow")?;
        bounded(self.jitter, 0.0, 0.2, "jitter")?;
        bounded(self.fish_speed, 0.15, 0.65, "fishSpeed")?;
        bounded(self.window_size, 0.12, 0.45, "windowSize")?;
        bounded(self.line_capacity, 1.0, 2.0, "lineCapacity")?;
        bounded(self.tackle_acceleration, 1.8, 5.0, "tackleAcceleration")?;
        bounded(self.tackle_damping, 2.5, 6.0, "tackleDamping")?;
        bounded(self.tackle_speed, 0.55, 1.1, "tackleSpeed")?;
        bounded(self.wait_min, 0.4, 5.0, "waitMin")?;
        bounded(self.wait_max, 0.4, 8.0, "waitMax")?;
        bounded(self.target_center, 0.2, 0.8, "targetCenter")?;
        bounded(self.target_spread, 0.2, 1.0, "targetSpread")?;
        bounded(self.rest_wander, 0.03, 0.3, "restWander")?;
        if self.wait_min > self.wait_max {
            return Err("waitMin exceeds waitMax".into());
        }
        Ok(())
    }
}

pub(crate) fn bounded(value: f64, min: f64, max: f64, name: &str) -> Result<()> {
    if !value.is_finite() || value < min || value > max {
        Err(format!("{name} must be finite and between {min} and {max}"))
    } else {
        Ok(())
    }
}
/// Resolved encounter rules. Keep this configuration fixed for the entire run.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Config {
    /// Hook-only, pressure/release, or spatial tracking.
    pub mode: Mode,
    /// 0 for hook/pressure; 1 or 2 for tracking.
    pub dimensions: u8,
    /// Base numeric tuning; defaults apply when omitted during deserialization.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parameters: Parameters,
    /// Rectangle, or a polygon for two-dimensional tracking.
    pub capture: Capture,
    /// Ordered fish segments; empty selects the original rest/warning/surge cycle.
    #[cfg_attr(feature = "serde", serde(default))]
    pub pattern: Vec<Segment>,
    /// Optional false bites before the real bite. Count zero disables them.
    #[cfg_attr(feature = "serde", serde(default))]
    pub nibbles: Nibbles,
    /// Encounter tick limit in 1..=MAX_TICKS; defaults to 3600 (60 seconds).
    #[cfg_attr(
        feature = "serde",
        serde(default = "default_max_ticks", rename = "maxTicks")
    )]
    pub max_ticks: u32,
}
fn default_max_ticks() -> u32 {
    3600
}
impl Default for Config {
    fn default() -> Self {
        Self {
            mode: Mode::Tracking,
            dimensions: 1,
            parameters: Parameters::default(),
            capture: Capture::Rectangle,
            pattern: vec![],
            nibbles: Nibbles::default(),
            max_ticks: default_max_ticks(),
        }
    }
}
impl Config {
    /// Check finite values, inclusive bounds and cross-field constraints.
    /// Returns a diagnostic error without modifying the value.
    pub fn validate(&self) -> Result<()> {
        if !matches!(
            (self.mode, self.dimensions),
            (Mode::Tracking, 1 | 2) | (Mode::Hook | Mode::Pressure, 0)
        ) {
            return Err("mode and dimensions do not match".into());
        }
        if self.dimensions != 2 && self.capture != Capture::Rectangle {
            return Err("polygon capture requires two dimensions".into());
        }
        if self.max_ticks == 0 || self.max_ticks > MAX_TICKS {
            return Err("invalid encounter tick limit".into());
        }
        crate::program::validate_pattern(&self.pattern)?;
        self.nibbles.validate()?;
        if self.mode == Mode::Hook && !self.pattern.is_empty() {
            return Err("hook mode has no fish behavior pattern".into());
        }
        self.parameters.validate()?;
        validate_capture(&self.capture)
    }
}
/// Spatial state in normalized coordinates, present only in tracking mode.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(deny_unknown_fields, rename_all = "camelCase")
)]
pub struct Motion {
    /// Vertical fish center in 0..=1; increasing values move upward.
    pub fish_position: f64,
    /// Vertical fish velocity in normalized units per second.
    pub fish_velocity: f64,
    /// Vertical destination of the current movement behavior, in 0..=1.
    pub fish_target: f64,
    /// Vertical tackle center in 0..=1.
    pub tackle_position: f64,
    /// Vertical tackle velocity in normalized units per second.
    pub tackle_velocity: f64,
    /// Horizontal fish center in 0..=1.
    pub fish_x: f64,
    /// Horizontal fish velocity in normalized units per second.
    pub fish_velocity_x: f64,
    /// Horizontal destination of the current movement behavior, in 0..=1.
    pub fish_target_x: f64,
    /// Horizontal tackle center in 0..=1.
    pub tackle_x: f64,
    /// Horizontal tackle velocity in normalized units per second.
    pub tackle_velocity_x: f64,
    /// Horizontal control in -1..=1; active only in two-dimensional tracking.
    pub steer: f64,
}
impl Default for Motion {
    fn default() -> Self {
        Self {
            fish_position: 0.5,
            fish_velocity: 0.0,
            fish_target: 0.5,
            tackle_position: 0.5,
            tackle_velocity: 0.0,
            fish_x: 0.5,
            fish_velocity_x: 0.0,
            fish_target_x: 0.5,
            tackle_x: 0.5,
            tackle_velocity_x: 0.0,
            steer: 0.0,
        }
    }
}
/// Complete resumable simulation state, including previous input and RNG. No hidden state exists.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(deny_unknown_fields, rename_all = "camelCase")
)]
pub struct State {
    /// Wire schema revision; currently [`crate::VERSION`].
    pub version: u32,
    /// Hook-only, pressure/release, or spatial tracking.
    pub mode: Mode,
    /// 0 for hook/pressure; 1 or 2 for tracking.
    pub dimensions: u8,
    /// Advancing ticks since creation, bounded by [`crate::MAX_TICKS`].
    pub tick: u32,
    /// Current lifecycle phase.
    pub phase: Phase,
    /// Advancing ticks elapsed in the current phase.
    pub phase_ticks: u32,
    /// Current waiting/nibble/bite duration in ticks; zero for untimed phases.
    pub duration: u32,
    /// Current fish effort behavior.
    pub behavior: Behavior,
    /// Ticks elapsed in the current fish behavior.
    pub behavior_ticks: u32,
    /// Current fish behavior duration in ticks.
    pub behavior_duration: u32,
    /// Catch progress in 0..=1; reaching 1 lands the fish.
    pub progress: f64,
    /// Normalized strain in 0..=1; reaching 1 breaks the line.
    pub tension: f64,
    /// Fish energy in 0..=1; depletion weakens the fish but does not itself end the run.
    pub energy: f64,
    /// Previous primary control in 0..=1, retained to detect fresh presses.
    pub primary: f64,
    /// Spatial state for tracking, otherwise `None`.
    pub motion: Option<Motion>,
    /// Explicit 32-bit RNG state, updated only when a draw is consumed.
    pub rng: u32,
    /// Terminal outcome reason; `None` while nonterminal.
    pub reason: Option<Reason>,
    /// Index in the custom fish pattern; zero for the original cycle.
    #[cfg_attr(feature = "serde", serde(default))]
    pub segment_index: u8,
    /// Number of false bites still to enter in this encounter.
    #[cfg_attr(feature = "serde", serde(default))]
    pub nibbles_left: u8,
}
/// Host controls for one tick. Finite values are clamped to the documented ranges.
#[derive(Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
pub struct Input {
    /// Reel/lift control in 0..=1. A zero-to-positive edge casts or hooks.
    pub primary: f64,
    /// Horizontal control in -1..=1; active only in two-dimensional tracking.
    pub steer: f64,
}
/// Next state and ordered events produced by one call to [`crate::step`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Transition {
    /// Next state; the input state remains unchanged.
    pub state: State,
    /// Ordered events for this tick, empty when no transition occurs.
    pub events: Vec<Event>,
}
/// Instantaneous overlap and resource rates calculated from the current state.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Observation {
    /// Fraction of fish capture area covered by the tackle, in 0..=1.
    pub alignment: f64,
    /// Current fish pull before division by line capacity.
    pub pull: f64,
    /// Strain target toward which tension relaxes.
    pub target_tension: f64,
    /// Progress change per simulated second.
    pub progress_rate: f64,
    /// Strain change per simulated second.
    pub tension_rate: f64,
    /// Energy change per simulated second.
    pub energy_rate: f64,
    /// Horizontal bounding-window overlap; zero outside 2D tracking.
    pub alignment_x: f64,
    /// Vertical bounding-window overlap; zero outside tracking.
    pub alignment_y: f64,
}
impl State {
    /// Whether this state absorbs further input (caught or escaped).
    pub fn is_terminal(&self) -> bool {
        matches!(self.phase, Phase::Caught | Phase::Escaped)
    }
    /// Check snapshot invariants against an already validated configuration.
    /// This checks structural validity, not reachability from a particular seed.
    pub fn validate(&self, c: &Config) -> Result<()> {
        if self.version != VERSION {
            return Err("unsupported state version".into());
        }
        if self.mode != c.mode || self.dimensions != c.dimensions {
            return Err("state and config mechanisms must match".into());
        }
        for n in [
            self.tick,
            self.phase_ticks,
            self.duration,
            self.behavior_ticks,
            self.behavior_duration,
        ] {
            if n > MAX_TICKS {
                return Err("timer exceeds tick bound".into());
            }
        }
        if self.phase_ticks > self.tick || self.behavior_ticks > self.tick {
            return Err("elapsed phase/behavior time exceeds total time".into());
        }
        if self.tick > c.max_ticks || (!self.is_terminal() && self.tick == c.max_ticks) {
            return Err("nonterminal state at tick limit".into());
        }
        for (n, v) in [
            ("progress", self.progress),
            ("tension", self.tension),
            ("energy", self.energy),
            ("primary", self.primary),
        ] {
            bounded(v, 0.0, 1.0, n)?;
        }
        if self.is_terminal() != self.reason.is_some() {
            return Err("terminal reason does not match phase".into());
        }
        if self.phase == Phase::Caught
            && (self.reason != Some(Reason::Landed) || self.progress != 1.0)
        {
            return Err("caught requires landed and progress 1".into());
        }
        if self.phase == Phase::Escaped && self.reason == Some(Reason::Landed) {
            return Err("escaped cannot be landed".into());
        }
        if self.phase == Phase::Struggle && c.mode == Mode::Hook {
            return Err("hook mode has no struggle".into());
        }
        if matches!(self.phase, Phase::Waiting | Phase::Bite | Phase::Nibble)
            && (self.duration == 0 || self.phase_ticks >= self.duration)
        {
            return Err("invalid phase timer".into());
        }
        if self.nibbles_left > c.nibbles.count
            || (self.phase == Phase::Nibble && c.nibbles.count == 0)
        {
            return Err("invalid nibble state".into());
        }
        if (c.pattern.is_empty() && self.segment_index != 0)
            || (!c.pattern.is_empty() && self.segment_index as usize >= c.pattern.len())
        {
            return Err("invalid segment index".into());
        }
        if self.phase == Phase::Struggle
            && !c.pattern.is_empty()
            && self.behavior != c.pattern[self.segment_index as usize].behavior
        {
            return Err("behavior does not match segment".into());
        }
        if self.phase == Phase::Struggle
            && (self.behavior_duration == 0 || self.behavior_ticks >= self.behavior_duration)
        {
            return Err("invalid behavior timer".into());
        }
        if c.mode == Mode::Tracking {
            let max_pace = c
                .pattern
                .iter()
                .map(|s| s.pace)
                .reduce(f64::max)
                .unwrap_or(1.7);
            let m = self.motion.as_ref().ok_or("tracking requires motion")?;
            for v in [
                m.fish_position,
                m.fish_target,
                m.tackle_position,
                m.fish_x,
                m.fish_target_x,
                m.tackle_x,
            ] {
                bounded(v, 0.0, 1.0, "position")?;
            }
            for v in [m.fish_velocity, m.fish_velocity_x] {
                bounded(
                    v,
                    -c.parameters.fish_speed * max_pace,
                    c.parameters.fish_speed * max_pace,
                    "fish velocity",
                )?;
            }
            for v in [m.tackle_velocity, m.tackle_velocity_x] {
                bounded(
                    v,
                    -c.parameters.tackle_speed,
                    c.parameters.tackle_speed,
                    "tackle velocity",
                )?;
            }
            bounded(m.steer, -1.0, 1.0, "steer")?;
        } else if self.motion.is_some() {
            return Err("nonspatial mode must have null motion".into());
        }
        Ok(())
    }
}
