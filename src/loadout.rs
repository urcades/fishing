//! Pure profile resolution; catalogs, labels and UI metadata belong to hosts.
use crate::{dynamics::random, geometry::Capture, types::*};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Fish {
    pub strength: f64,
    pub fish_speed: f64,
    pub surge: f64,
    pub rest: f64,
    pub fatigue: f64,
    pub recovery: f64,
    pub bite_window: f64,
    pub target_center: f64,
    pub target_spread: f64,
    pub rest_wander: f64,
    pub preference: String,
    pub pond_weight: f64,
}
impl Fish {
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Rod {
    pub window_size: f64,
    pub reel_rate: f64,
    pub line_capacity: f64,
    pub tackle_acceleration: f64,
    pub tackle_damping: f64,
    pub tackle_speed: f64,
}
impl Rod {
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Bait {
    pub attraction: f64,
    pub bite_bonus: f64,
    pub affinity: BTreeMap<String, f64>,
}
impl Bait {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Definition {
    pub mode: Mode,
    pub dimensions: u8,
    #[serde(default)]
    pub parameters: Parameters,
    pub capture: Capture,
    #[serde(default)]
    pub hook_bonus: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Loadout {
    pub fish: Fish,
    pub rod: Rod,
    pub bait: Bait,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Encounter {
    pub version: u32,
    pub config: Config,
    pub seed: u32,
    pub notes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub index: usize,
    pub seed: u32,
    pub odds: Vec<f64>,
}
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
