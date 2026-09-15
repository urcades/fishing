//! Fixed-step coupled dynamics. Every derivative reads the pre-step state.
use crate::{geometry, types::*, TargetRule};
/// Advance the specified wrapping 32-bit LCG; return `(next_seed, value_in_0_to_1)`.
/// Zero is a valid seed. This is replay randomness, not a cryptographic generator.
pub fn random(rng: u32) -> (u32, f64) {
    let next = rng.wrapping_mul(1664525).wrapping_add(1013904223);
    (next, next as f64 / 4294967296.0)
}
fn ticks(seconds: f64) -> u32 {
    ((seconds * HZ as f64 + 0.5).floor() as u32).clamp(1, MAX_TICKS)
}
/// Create a ready state from an explicit seed and validated rules.
/// No random draw is consumed until the first cast. Returns an error for invalid rules.
pub fn create_state(seed: u32, c: &Config) -> Result<State> {
    c.validate()?;
    Ok(State {
        version: VERSION,
        segment_index: 0,
        nibbles_left: c.nibbles.count,
        mode: c.mode,
        dimensions: c.dimensions,
        tick: 0,
        phase: Phase::Ready,
        phase_ticks: 0,
        duration: 0,
        behavior: Behavior::Rest,
        behavior_ticks: 0,
        behavior_duration: 0,
        progress: 0.25,
        tension: 0.0,
        energy: 1.0,
        primary: 0.0,
        motion: if c.mode == Mode::Tracking {
            Some(Motion::default())
        } else {
            None
        },
        rng: seed,
        reason: None,
    })
}
fn enter_behavior(s: &mut State, b: Behavior, c: &Config) {
    let p = &c.parameters;
    let r = if b == Behavior::Warning {
        0.5
    } else {
        let (rng, r) = random(s.rng);
        s.rng = rng;
        r
    };
    let duration = match b {
        Behavior::Rest => p.rest,
        Behavior::Warning => p.warning,
        Behavior::Surge => p.surge,
    };
    s.behavior = b;
    s.behavior_ticks = 0;
    s.behavior_duration = ticks(duration * (1.0 + (r * 2.0 - 1.0) * p.jitter));
    if c.mode != Mode::Tracking || b == Behavior::Surge {
        return;
    }
    let (rng, r) = random(s.rng);
    s.rng = rng;
    let m = s.motion.as_mut().unwrap();
    let raw = if b == Behavior::Rest {
        (m.fish_position + (r * 2.0 - 1.0) * p.rest_wander).clamp(0.08, 0.92)
    } else if m.fish_position < 0.5 {
        0.6 + r * 0.3
    } else {
        0.1 + r * 0.3
    };
    // Preserve the accepted neutral trajectory without an extra subtract/add.
    m.fish_target = if p.target_center == 0.5 && p.target_spread == 1.0 {
        raw
    } else {
        (p.target_center + (raw - 0.5) * p.target_spread).clamp(0.08, 0.92)
    };
    if c.dimensions == 2 {
        let (rng, r) = random(s.rng);
        s.rng = rng;
        m.fish_target_x = if b == Behavior::Rest {
            (m.fish_x + (r * 2.0 - 1.0) * 0.16).clamp(0.08, 0.92)
        } else if m.fish_x < 0.5 {
            0.6 + r * 0.3
        } else {
            0.1 + r * 0.3
        };
    }
}
// Segment entry owns every random draw. Keeping this separate from movement
// makes snapshots sufficient to resume halfway through a behavior.
fn enter_segment(s: &mut State, index: usize, c: &Config) {
    let segment = &c.pattern[index];
    s.segment_index = index as u8;
    s.behavior = segment.behavior;
    s.behavior_ticks = 0;
    let (rng, r) = random(s.rng);
    s.rng = rng;
    s.behavior_duration = ticks(segment.duration * (1.0 + (2.0 * r - 1.0) * segment.jitter));
    let Some(m) = s.motion.as_mut() else {
        return;
    };
    for axis in 0..c.dimensions {
        let (position, target) = if axis == 0 {
            (m.fish_position, &mut m.fish_target)
        } else {
            (m.fish_x, &mut m.fish_target_x)
        };
        *target = match segment.target {
            TargetRule::Keep {} => *target,
            TargetRule::Hold {} => position,
            TargetRule::Point { point } => {
                point[if axis == 0 { 1 } else { 0 }].clamp(FISH_RADIUS, 1.0 - FISH_RADIUS)
            }
            TargetRule::Wander { distance } => {
                let (rng, r) = random(s.rng);
                s.rng = rng;
                (position + (2.0 * r - 1.0) * distance).clamp(FISH_RADIUS, 1.0 - FISH_RADIUS)
            }
            TargetRule::Opposite {} => {
                let (rng, r) = random(s.rng);
                s.rng = rng;
                if position < 0.5 {
                    0.6 + 0.3 * r
                } else {
                    0.1 + 0.3 * r
                }
            }
        };
    }
}
fn behavior_event(b: Behavior) -> Event {
    match b {
        Behavior::Rest => Event::Rest,
        Behavior::Warning => Event::Warning,
        Behavior::Surge => Event::Surge,
    }
}
fn rates(s: &State, c: &Config, primary: f64) -> Observation {
    let p = &c.parameters;
    let active = s.phase == Phase::Struggle;
    let pull = if active {
        p.strength
            * s.energy
            * if let Some(segment) = c.pattern.get(s.segment_index as usize) {
                segment.intensity
            } else if s.behavior == Behavior::Surge {
                1.0
            } else {
                0.15
            }
    } else {
        0.0
    };
    let (mut q, mut qx, mut qy) = (0.0, 0.0, 0.0);
    if let Some(m) = &s.motion {
        qy = geometry::interval_alignment(
            m.fish_position,
            m.tackle_position,
            p.window_size,
            FISH_RADIUS,
        );
        q = qy;
        if c.dimensions == 2 {
            qx = geometry::interval_alignment(m.fish_x, m.tackle_x, p.window_size, FISH_RADIUS);
            q = geometry::alignment(
                &c.capture,
                [m.fish_x, m.fish_position],
                [m.tackle_x, m.tackle_position],
                p.window_size,
                FISH_RADIUS,
            );
        }
    }
    let (target, progress, energy) = if !active {
        (0.0, 0.0, 0.0)
    } else if c.mode == Mode::Pressure {
        (
            primary * (p.base_tension + pull) / p.line_capacity,
            p.reel_rate * primary - p.escape_rate * pull * (1.0 - primary),
            p.recovery * (1.0 - primary) * (1.0 - s.energy)
                - p.fatigue * (s.tension * p.line_capacity) * s.energy,
        )
    } else {
        (
            (p.base_tension * q + pull * (0.3 + 1.1 * (1.0 - q))) / p.line_capacity,
            p.reel_rate * q - p.escape_rate * (0.4 + pull) * (1.0 - q),
            p.recovery * (1.0 - q) * (1.0 - s.energy)
                - p.fatigue * q * (s.tension * p.line_capacity) * s.energy,
        )
    };
    Observation {
        alignment: q,
        pull,
        target_tension: target,
        progress_rate: progress,
        tension_rate: if active {
            (target - s.tension) / p.response
        } else {
            0.0
        },
        energy_rate: energy,
        alignment_x: qx,
        alignment_y: qy,
    }
}
/// Inspect overlap and instantaneous rates without advancing time or RNG.
/// Uses the stored primary input and rejects invalid configuration/state pairs.
pub fn observe(s: &State, c: &Config) -> Result<Observation> {
    c.validate()?;
    s.validate(c)?;
    Ok(rates(s, c, s.primary))
}
fn move_axis(pos: f64, vel: f64, acc: f64, limit: f64, min: f64, max: f64) -> (f64, f64) {
    let speed = (vel + acc * DT).clamp(-limit, limit);
    let raw = pos + speed * DT;
    (
        raw.clamp(min, max),
        if raw <= min || raw >= max { 0.0 } else { speed },
    )
}
fn movement(s: &State, c: &Config, u: f64, steer: f64) -> Option<Motion> {
    let Some(m) = &s.motion else {
        return None;
    };
    let mut n = m.clone();
    let p = &c.parameters;
    let vigor = 0.35 + 0.65 * s.energy;
    let segment = c.pattern.get(s.segment_index as usize);
    let hold = segment.map_or(s.behavior == Behavior::Warning, |v| {
        v.target == TargetRule::Hold {}
    });
    let pace = if let Some(segment) = segment {
        segment.pace
    } else if s.behavior == Behavior::Surge {
        1.7
    } else {
        0.65
    };
    (n.tackle_position, n.tackle_velocity) = move_axis(
        m.tackle_position,
        m.tackle_velocity,
        (2.0 * u - 1.0) * p.tackle_acceleration - p.tackle_damping * m.tackle_velocity,
        p.tackle_speed,
        p.window_size / 2.0,
        1.0 - p.window_size / 2.0,
    );
    let target = if hold { m.fish_position } else { m.fish_target };
    (n.fish_position, n.fish_velocity) = move_axis(
        m.fish_position,
        m.fish_velocity,
        7.0 * vigor * (target - m.fish_position) - 3.0 * m.fish_velocity,
        p.fish_speed * vigor * pace,
        FISH_RADIUS,
        1.0 - FISH_RADIUS,
    );
    if c.dimensions == 2 {
        (n.tackle_x, n.tackle_velocity_x) = move_axis(
            m.tackle_x,
            m.tackle_velocity_x,
            steer * p.tackle_acceleration - p.tackle_damping * m.tackle_velocity_x,
            p.tackle_speed,
            p.window_size / 2.0,
            1.0 - p.window_size / 2.0,
        );
        let target = if hold { m.fish_x } else { m.fish_target_x };
        (n.fish_x, n.fish_velocity_x) = move_axis(
            m.fish_x,
            m.fish_velocity_x,
            7.0 * vigor * (target - m.fish_x) - 3.0 * m.fish_velocity_x,
            p.fish_speed * vigor * pace,
            FISH_RADIUS,
            1.0 - FISH_RADIUS,
        );
        n.steer = steer;
    }
    Some(n)
}
fn finish(mut s: State, phase: Phase, reason: Reason) -> Transition {
    s.phase = phase;
    s.phase_ticks = 0;
    s.duration = 0;
    s.reason = Some(reason);
    Transition {
        state: s,
        events: vec![if phase == Phase::Caught {
            Event::Caught
        } else {
            Event::Escaped
        }],
    }
}
/// Advance exactly one 1/60-second tick, returning state and ordered events.
/// No clocks, mutation of arguments, callbacks or IO. Config/state must match;
/// nonfinite controls are rejected and finite controls are clamped.
/// Idle ready states and terminal states do not advance.
/// Loss thresholds inspect raw results, before clamping. Behavior changes
/// follow integration; new movement contributes to alignment next tick.
pub fn step(s: &State, input: Input, c: &Config) -> Result<Transition> {
    c.validate()?;
    s.validate(c)?;
    if !input.primary.is_finite() || !input.steer.is_finite() {
        return Err("input must be finite".into());
    }
    if s.is_terminal() {
        return Ok(Transition {
            state: s.clone(),
            events: vec![],
        });
    }
    let u = input.primary.clamp(0.0, 1.0);
    let steer = input.steer.clamp(-1.0, 1.0);
    let pressed = u > 0.0 && s.primary == 0.0;
    if s.phase == Phase::Ready && !pressed {
        return Ok(Transition {
            state: s.clone(),
            events: vec![],
        });
    }
    let mut n = s.clone();
    n.tick += 1;
    n.phase_ticks += 1;
    n.primary = u;
    let mut events = vec![];
    match s.phase {
        Phase::Ready => {
            let (rng, r) = random(s.rng);
            n.rng = rng;
            n.phase = Phase::Waiting;
            n.phase_ticks = 0;
            n.duration =
                ticks(c.parameters.wait_min + r * (c.parameters.wait_max - c.parameters.wait_min));
            events.push(Event::Cast);
        }
        Phase::Waiting => {
            if n.phase_ticks >= s.duration {
                n.phase_ticks = 0;
                if s.nibbles_left > 0 {
                    n.nibbles_left -= 1;
                    n.phase = Phase::Nibble;
                    n.duration = ticks(c.nibbles.duration);
                    events.push(Event::Nibble);
                } else {
                    n.phase = Phase::Bite;
                    n.duration = ticks(c.parameters.bite_window);
                    events.push(Event::Bite);
                }
            }
        }
        Phase::Nibble => {
            // Like bite expiry, expiry is judged before the new input edge.
            if n.phase_ticks >= s.duration {
                n.phase = Phase::Waiting;
                n.phase_ticks = 0;
                n.duration = ticks(c.nibbles.gap);
            } else if pressed {
                return Ok(finish(n, Phase::Escaped, Reason::EarlyHook));
            }
        }
        Phase::Bite => {
            if n.phase_ticks >= s.duration {
                return Ok(finish(n, Phase::Escaped, Reason::MissedBite));
            }
            if pressed {
                if c.mode == Mode::Hook {
                    n.progress = 1.0;
                    let mut r = finish(n, Phase::Caught, Reason::Landed);
                    r.events.insert(0, Event::Hooked);
                    return Ok(r);
                }
                n.phase = Phase::Struggle;
                n.phase_ticks = 0;
                n.duration = 0;
                if c.pattern.is_empty() {
                    enter_behavior(&mut n, Behavior::Rest, c);
                } else {
                    enter_segment(&mut n, 0, c);
                }
                events.extend([Event::Hooked, behavior_event(n.behavior)]);
            }
        }
        Phase::Struggle => {
            let r = rates(s, c, u);
            let progress = s.progress + r.progress_rate * DT;
            let tension = s.tension + r.tension_rate * DT;
            n.motion = movement(s, c, u, steer);
            n.progress = progress.clamp(0.0, 1.0);
            n.tension = tension.clamp(0.0, 1.0);
            n.energy = (s.energy + r.energy_rate * DT).clamp(0.0, 1.0);
            n.behavior_ticks += 1;
            if tension >= 1.0 {
                return Ok(finish(n, Phase::Escaped, Reason::LineBroke));
            }
            if progress <= 0.0 {
                return Ok(finish(n, Phase::Escaped, Reason::GotAway));
            }
            if progress >= 1.0 {
                return Ok(finish(n, Phase::Caught, Reason::Landed));
            }
            if n.behavior_ticks >= s.behavior_duration {
                if c.pattern.is_empty() {
                    let b = match s.behavior {
                        Behavior::Rest => Behavior::Warning,
                        Behavior::Warning => Behavior::Surge,
                        Behavior::Surge => Behavior::Rest,
                    };
                    enter_behavior(&mut n, b, c);
                } else {
                    enter_segment(&mut n, (s.segment_index as usize + 1) % c.pattern.len(), c);
                }
                events.push(behavior_event(n.behavior));
            }
        }
        _ => unreachable!(),
    }
    if n.tick >= c.max_ticks {
        return Ok(finish(n, Phase::Escaped, Reason::Timeout));
    }
    Ok(Transition { state: n, events })
}
