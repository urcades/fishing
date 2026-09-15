//! Bounded declarative fish behaviors and false-bite timing.
use crate::types::bounded;
use crate::{Behavior, Result, DT};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// Target selection on entering a segment, in normalized coordinates.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(
    feature = "serde",
    serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)
)]
pub enum TargetRule {
    /// Continue toward the previous target, without consuming a target draw.
    Keep {},
    /// Decelerate in place throughout this segment; no target draws.
    Hold {},
    /// Choose a random displacement independently on each active axis.
    Wander {
        /// Maximum displacement in 0..=1; targets clamp to the fish bounds.
        distance: f64,
    },
    /// Aim at a fixed point, without consuming a target draw.
    Point {
        /// Normalized [x, y] target; 1D tracking uses y only.
        point: [f64; 2],
    },
    /// Choose a random target in the opposite half, like the original warning.
    Opposite {},
}
impl Default for TargetRule {
    fn default() -> Self {
        Self::Keep {}
    }
}
/// One segment in a repeating sequence of at most 16 segments.
/// Labels emit events; effort and movement depend on intensity/pace/target,
/// so consecutive surges can have different movement and pull.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
pub struct Segment {
    /// Event label and visible behavior, independent of numeric effort.
    pub behavior: Behavior,
    /// Pull multiplier in 0..=100, applied to strength times energy.
    pub intensity: f64,
    /// Fish speed multiplier in 0..=10.
    pub pace: f64,
    /// Duration in seconds, DT..=600, before jitter and tick saturation.
    pub duration: f64,
    /// Fractional duration variation in 0..=1.
    pub jitter: f64,
    /// Rule evaluated when entering this segment.
    pub target: TargetRule,
}
impl Default for Segment {
    fn default() -> Self {
        Self {
            behavior: Behavior::Rest,
            intensity: 0.15,
            pace: 0.65,
            duration: 1.2,
            jitter: 0.0,
            target: TargetRule::Keep {},
        }
    }
}
pub(crate) fn validate_pattern(pattern: &[Segment]) -> Result<()> {
    if pattern.len() > 16 {
        return Err("pattern exceeds 16 segments".into());
    }
    for s in pattern {
        bounded(s.intensity, 0.0, 100.0, "intensity")?;
        bounded(s.pace, 0.0, 10.0, "pace")?;
        bounded(s.duration, DT, 600.0, "segment duration")?;
        bounded(s.jitter, 0.0, 1.0, "segment jitter")?;
        match s.target {
            TargetRule::Wander { distance } => bounded(distance, 0.0, 1.0, "wander distance")?,
            TargetRule::Point { point } => {
                for v in point {
                    bounded(v, 0.0, 1.0, "target coordinate")?;
                }
            }
            _ => {}
        }
    }
    Ok(())
}
/// False-bite sequence. Each nibble is followed by a gap, then another nibble
/// or the real bite. No additional RNG draws are consumed by this sequence.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde", serde(default, deny_unknown_fields))]
pub struct Nibbles {
    /// Number of false bites, 0..=8; zero disables the sequence.
    pub count: u8,
    /// Duration of each false bite in seconds, DT..=600.
    pub duration: f64,
    /// Waiting time after each false bite in seconds, DT..=600.
    pub gap: f64,
}
impl Default for Nibbles {
    fn default() -> Self {
        Self {
            count: 0,
            duration: 0.25,
            gap: 0.6,
        }
    }
}
impl Nibbles {
    /// Check finite durations and the bounded sequence length.
    pub fn validate(&self) -> Result<()> {
        if self.count > 8 {
            return Err("nibble count exceeds 8".into());
        }
        bounded(self.duration, DT, 600.0, "nibble duration")?;
        bounded(self.gap, DT, 600.0, "nibble gap")
    }
}
