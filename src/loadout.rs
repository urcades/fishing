//! Pure profile resolution; catalogs, labels and UI metadata belong to hosts.
use crate::{dynamics::random, geometry::Capture, types::*};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
/// Fish mechanics and pond-selection weight; presentation names belong to the host.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(deny_unknown_fields, rename_all = "camelCase")
)]
pub struct Fish {
    /// Fish pull multiplier, 0.4..=1.6.
    pub strength: f64,
    /// Fish speed scale in normalized units per second, 0.15..=0.65.
    pub fish_speed: f64,
    /// Surge duration in seconds before jitter, 0.5..=3.0.
    pub surge: f64,
    /// Rest duration in seconds before jitter, 0.4..=3.0.
    pub rest: f64,
    /// Energy loss coefficient per second, 0.03..=0.25.
    pub fatigue: f64,
    /// Energy recovery coefficient per second, 0.05..=0.4.
    pub recovery: f64,
    /// Hook opportunity in seconds, 0.5..=2.0; expiry takes precedence over input.
    pub bite_window: f64,
    /// Preferred vertical target center, 0.2..=0.8.
    pub target_center: f64,
    /// Vertical target distribution scale, 0.2..=1.0.
    pub target_spread: f64,
    /// Vertical target wander during rest, 0.03..=0.3.
    pub rest_wander: f64,
    /// Host-defined bait preference key, at most 64 UTF-8 bytes.
    pub preference: String,
    /// Positive base selection weight, at least `f64::MIN_POSITIVE` and at most 10.
    pub pond_weight: f64,
}
impl Fish {
    /// Check finite values, inclusive bounds and cross-field constraints.
    /// Returns a diagnostic error without modifying the value.
    pub fn validate(&self) -> Result<()> {
        bounded(self.strength, 0.4, 1.6, "fish.strength")?;
        bounded(self.fish_speed, 0.15, 0.65, "fish.fishSpeed")?;
        bounded(self.surge, 0.5, 3.0, "fish.surge")?;
        bounded(self.rest, 0.4, 3.0, "fish.rest")?;
        bounded(self.fatigue, 0.03, 0.25, "fish.fatigue")?;
        bounded(self.recovery, 0.05, 0.4, "fish.recovery")?;
        bounded(self.bite_window, 0.5, 2.0, "fish.biteWindow")?;
        bounded(self.target_center, 0.2, 0.8, "fish.targetCenter")?;
        bounded(self.target_spread, 0.2, 1.0, "fish.targetSpread")?;
        bounded(self.rest_wander, 0.03, 0.3, "fish.restWander")?;
        bounded(self.pond_weight, f64::MIN_POSITIVE, 10.0, "pondWeight")?;
        if self.preference.len() > 64 {
            return Err("preference too long".into());
        }
        Ok(())
    }
}
/// Rod handling tradeoffs applied by [`resolve`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(deny_unknown_fields, rename_all = "camelCase")
)]
pub struct Rod {
    /// Tackle window side length in normalized units, 0.12..=0.45.
    pub window_size: f64,
    /// Maximum progress gain per second, 0.02..=0.2.
    pub reel_rate: f64,
    /// Divisor converting pull into normalized strain, 1.0..=2.0.
    pub line_capacity: f64,
    /// Control acceleration in normalized units per second squared, 1.8..=5.0.
    pub tackle_acceleration: f64,
    /// Velocity damping coefficient per second, 2.5..=6.0.
    pub tackle_damping: f64,
    /// Maximum tackle speed in normalized units per second, 0.55..=1.1.
    pub tackle_speed: f64,
}
impl Rod {
    /// Check finite values, inclusive bounds and cross-field constraints.
    /// Returns a diagnostic error without modifying the value.
    pub fn validate(&self) -> Result<()> {
        bounded(self.window_size, 0.12, 0.45, "rod.windowSize")?;
        bounded(self.reel_rate, 0.02, 0.2, "rod.reelRate")?;
        bounded(self.line_capacity, 1.0, 2.0, "rod.lineCapacity")?;
        bounded(self.tackle_acceleration, 1.8, 5.0, "rod.tackleAcceleration")?;
        bounded(self.tackle_damping, 2.5, 6.0, "rod.tackleDamping")?;
        bounded(self.tackle_speed, 0.55, 1.1, "rod.tackleSpeed")?;
        Ok(())
    }
}
/// Bite timing and preference weights applied by [`resolve`] and [`select`].
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(deny_unknown_fields, rename_all = "camelCase")
)]
pub struct Bait {
    /// Waiting-time divisor, 0.5..=2.5.
    pub attraction: f64,
    /// Additional hook time in seconds, 0..=0.5.
    pub bite_bonus: f64,
    /// Up to 64 preference multipliers in `f64::MIN_POSITIVE`..=5; missing keys use 1.
    pub affinity: BTreeMap<String, f64>,
}
impl Bait {
    /// Check finite values, inclusive bounds and cross-field constraints.
    /// Returns a diagnostic error without modifying the value.
    pub fn validate(&self) -> Result<()> {
        bounded(self.attraction, 0.5, 2.5, "bait.attraction")?;
        bounded(self.bite_bonus, 0.0, 0.5, "bait.biteBonus")?;
        if self.affinity.len() > 64 {
            return Err("too many affinities".into());
        }
        for (k, v) in &self.affinity {
            if k.len() > 64 {
                return Err("affinity key too long".into());
            }
            bounded(*v, f64::MIN_POSITIVE, 5.0, "affinity")?;
        }
        Ok(())
    }
}
/// Base rules before applying a fish, rod and bait loadout.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(deny_unknown_fields, rename_all = "camelCase")
)]
pub struct Definition {
    /// Hook-only, pressure/release, or spatial tracking.
    pub mode: Mode,
    /// 0 for hook/pressure; 1 or 2 for tracking.
    pub dimensions: u8,
    /// Base numeric tuning; defaults apply when omitted during deserialization.
    #[cfg_attr(feature = "serde", serde(default))]
    pub parameters: Parameters,
    /// Rectangle, or a polygon for two-dimensional tracking.
    pub capture: Capture,
    /// Mode-specific hook bonus in seconds, 0..=0.5.
    #[cfg_attr(feature = "serde", serde(default))]
    pub hook_bonus: f64,
}
/// The three profiles used to resolve an encounter.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Loadout {
    /// Selected fish profile.
    pub fish: Fish,
    /// Selected rod profile.
    pub rod: Rod,
    /// Selected bait profile.
    pub bait: Bait,
}
/// Validated resolved rules and seed. Persist these alongside a recording.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Encounter {
    /// Wire schema revision; currently [`crate::VERSION`].
    pub version: u32,
    /// Validated numeric rules for this encounter.
    pub config: Config,
    /// Seed to pass to the next operation; resolution does not consume it.
    pub seed: u32,
    /// Human-readable diagnostics for capped timing values; wording is not normative.
    pub notes: Vec<String>,
}
/// Weighted pond draw, including the next seed and normalized probabilities.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Selection {
    /// Index of the selected fish in the original pool.
    pub index: usize,
    /// RNG state after exactly one draw; carry this into encounter resolution.
    pub seed: u32,
    /// Normalized selection probability for each pool entry, in original order.
    pub odds: Vec<f64>,
}
/// Draw from 1..=64 fish using base weights multiplied by bait affinity.
/// Consumes exactly one RNG draw. Rejects invalid profiles or underflowed weights.
pub fn select(pool: &[Fish], bait: &Bait, seed: u32) -> Result<Selection> {
    if pool.is_empty() || pool.len() > 64 {
        return Err("pool requires 1..64 fish".into());
    }
    bait.validate()?;
    let mut weights = Vec::new();
    for fish in pool {
        fish.validate()?;
        weights
            .push(fish.pond_weight * bait.affinity.get(&fish.preference).copied().unwrap_or(1.0));
    }
    let total: f64 = weights.iter().sum();
    if !total.is_finite() || weights.iter().any(|w| !w.is_finite() || *w <= 0.0) {
        return Err("selection weight is zero or nonfinite after multiplication".into());
    }
    let odds: Vec<f64> = weights.iter().map(|w| w / total).collect();
    let (seed, r) = random(seed);
    let mut cumulative = 0.0;
    let mut index = pool.len() - 1;
    for (i, p) in odds.iter().enumerate() {
        cumulative += p;
        if r < cumulative {
            index = i;
            break;
        }
    }
    Ok(Selection { index, seed, odds })
}
fn cap(value: f64, min: f64, max: f64, name: &str, notes: &mut Vec<String>) -> f64 {
    let n = value.clamp(min, max);
    if n != value {
        notes.push(format!(
            "{name} capped at {n:.2} to stay within the model bounds."
        ));
    }
    n
}
/// Apply active profile attributes to base rules and validate the resulting configuration.
/// Timing is capped to protocol bounds with notes; inactive attributes do nothing.
/// This pure operation does not consume randomness or mutate its inputs.
pub fn resolve(d: &Definition, l: &Loadout, seed: u32) -> Result<Encounter> {
    l.fish.validate()?;
    l.rod.validate()?;
    l.bait.validate()?;
    d.parameters.validate()?;
    bounded(d.hook_bonus, 0.0, 0.5, "hookBonus")?;
    let mut p = d.parameters.clone();
    let f = &l.fish;
    let r = &l.rod;
    let b = &l.bait;
    let mut notes = vec![];
    if d.mode != Mode::Hook {
        p.strength = f.strength;
        p.surge = f.surge;
        p.rest = f.rest;
        p.fatigue = f.fatigue;
        p.recovery = f.recovery;
        p.reel_rate = r.reel_rate;
        p.line_capacity = r.line_capacity;
    }
    if d.mode == Mode::Tracking {
        p.fish_speed = f.fish_speed;
        p.target_center = f.target_center;
        p.target_spread = f.target_spread;
        p.rest_wander = f.rest_wander;
        p.window_size = r.window_size;
        p.tackle_acceleration = r.tackle_acceleration;
        p.tackle_damping = r.tackle_damping;
        p.tackle_speed = r.tackle_speed;
    }
    let affinity = b.affinity.get(&f.preference).copied().unwrap_or(1.0);
    p.wait_min = cap(
        1.5 / (b.attraction * affinity),
        0.4,
        5.0,
        "Minimum wait",
        &mut notes,
    );
    p.wait_max = cap(
        3.0 / (b.attraction * affinity),
        0.4,
        8.0,
        "Maximum wait",
        &mut notes,
    );
    p.bite_window = cap(
        f.bite_window + d.hook_bonus + b.bite_bonus,
        0.5,
        2.0,
        "Bite duration",
        &mut notes,
    );
    let config = Config {
        mode: d.mode,
        dimensions: d.dimensions,
        parameters: p,
        capture: d.capture.clone(),
    };
    config.validate()?;
    Ok(Encounter {
        version: VERSION,
        config,
        seed,
        notes,
    })
}
