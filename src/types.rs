//! Wire types and validation. Profiles and example names do not live here.
use crate::geometry::{validate_capture, Capture};
use serde::{Deserialize, Serialize};
pub type Result<T> = std::result::Result<T, String>;
pub const VERSION: u32 = 1;
pub const HZ: u32 = 60;
pub const DT: f64 = 1.0 / 60.0;
pub const MAX_TICKS: u32 = 3600;
pub const FISH_RADIUS: f64 = 0.03;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    Hook,
    Pressure,
    Tracking,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Ready,
    Waiting,
    Bite,
    Struggle,
    Caught,
    Escaped,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Behavior {
    Rest,
    Warning,
    Surge,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Reason {
    MissedBite,
    LineBroke,
    GotAway,
    Landed,
    Timeout,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Cast,
    Bite,
    Hooked,
    Rest,
    Warning,
    Surge,
    Caught,
    Escaped,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields, rename_all = "camelCase")]
pub struct Parameters {
    pub strength: f64,
    pub surge: f64,
    pub rest: f64,
    pub fatigue: f64,
    pub reel_rate: f64,
    pub escape_rate: f64,
    pub base_tension: f64,
    pub response: f64,
    pub recovery: f64,
    pub warning: f64,
    pub bite_window: f64,
    pub jitter: f64,
    pub fish_speed: f64,
    pub window_size: f64,
    pub line_capacity: f64,
    pub tackle_acceleration: f64,
    pub tackle_damping: f64,
    pub tackle_speed: f64,
    pub wait_min: f64,
    pub wait_max: f64,
    pub target_center: f64,
    pub target_spread: f64,
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
    pub fn validate(&self) -> Result<()> {
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
pub fn bounded(value: f64, min: f64, max: f64, name: &str) -> Result<()> {
    if !value.is_finite() || value < min || value > max {
        Err(format!("{name} must be finite and between {min} and {max}"))
    } else {
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub mode: Mode,
    pub dimensions: u8,
    #[serde(default)]
    pub parameters: Parameters,
    pub capture: Capture,
}
impl Config {
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
        self.parameters.validate()?;
        validate_capture(&self.capture)
    }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Motion {
    pub fish_position: f64,
    pub fish_velocity: f64,
    pub fish_target: f64,
    pub tackle_position: f64,
    pub tackle_velocity: f64,
    pub fish_x: f64,
    pub fish_velocity_x: f64,
    pub fish_target_x: f64,
    pub tackle_x: f64,
    pub tackle_velocity_x: f64,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct State {
    pub version: u32,
    pub mode: Mode,
    pub dimensions: u8,
    pub tick: u32,
    pub phase: Phase,
    pub phase_ticks: u32,
    pub duration: u32,
    pub behavior: Behavior,
    pub behavior_ticks: u32,
    pub behavior_duration: u32,
    pub progress: f64,
    pub tension: f64,
    pub energy: f64,
    pub primary: f64,
    pub motion: Option<Motion>,
    pub rng: u32,
    pub reason: Option<Reason>,
}
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Input {
    pub primary: f64,
    pub steer: f64,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Transition {
    pub state: State,
    pub events: Vec<Event>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Observation {
    pub alignment: f64,
    pub pull: f64,
    pub target_tension: f64,
    pub progress_rate: f64,
    pub tension_rate: f64,
    pub energy_rate: f64,
    pub alignment_x: f64,
    pub alignment_y: f64,
}
impl State {
    pub fn is_terminal(&self) -> bool {
        matches!(self.phase, Phase::Caught | Phase::Escaped)
    }
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
        if !self.is_terminal() && self.tick == MAX_TICKS {
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
        if matches!(self.phase, Phase::Waiting | Phase::Bite)
            && (self.duration == 0 || self.phase_ticks >= self.duration)
        {
            return Err("invalid phase timer".into());
        }
        if c.mode == Mode::Tracking {
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
                    -c.parameters.fish_speed * 1.7,
                    c.parameters.fish_speed * 1.7,
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
