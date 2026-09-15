#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![deny(missing_docs)]

mod dynamics;
pub mod geometry;
mod loadout;
mod program;
mod types;
pub use program::{Nibbles, Segment, TargetRule};

pub use dynamics::{create_state, observe, random, step};
pub use loadout::{
    resolve, select, Bait, Definition, Encounter, Fish, Loadout, Rod, Selection, TimingPolicy,
};
pub use types::*;
