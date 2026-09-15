//! Fixed-step coupled dynamics. Every derivative reads the pre-step state.
use crate::{geometry, types::*};
pub fn random(rng: u32) -> (u32, f64) {
    let next = rng.wrapping_mul(1664525).wrapping_add(1013904223);
    (next, next as f64 / 4294967296.0)
}
fn ticks(seconds: f64) -> u32 {
    ((seconds * HZ as f64 + 0.5).floor() as u32).max(1)
}
pub fn create_state(seed: u32, c: &Config) -> Result<State> {
    c.validate()?;
    Ok(State {
        version: VERSION,
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
fn rates(s: &State, c: &Config, primary: f64) -> Observation {
    let p = &c.parameters;
    let active = s.phase == Phase::Struggle;
    let pull = if active {
        p.strength
            * s.energy
            * if s.behavior == Behavior::Surge {
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
    let pace = if s.behavior == Behavior::Surge {
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
    let target = if s.behavior == Behavior::Warning {
        m.fish_position
    } else {
        m.fish_target
    };
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
        let target = if s.behavior == Behavior::Warning {
            m.fish_x
        } else {
            m.fish_target_x
        };
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
/// Advance one tick. No clocks, mutation of arguments, callbacks or IO.
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
                n.phase = Phase::Bite;
                n.phase_ticks = 0;
                n.duration = ticks(c.parameters.bite_window);
                events.push(Event::Bite);
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
                enter_behavior(&mut n, Behavior::Rest, c);
                events.extend([Event::Hooked, Event::Rest]);
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
                let b = match s.behavior {
                    Behavior::Rest => Behavior::Warning,
                    Behavior::Warning => Behavior::Surge,
                    Behavior::Surge => Behavior::Rest,
                };
                enter_behavior(&mut n, b, c);
                events.push(match b {
                    Behavior::Rest => Event::Rest,
                    Behavior::Warning => Event::Warning,
                    Behavior::Surge => Event::Surge,
                });
            }
        }
        _ => unreachable!(),
    }
    if n.tick >= MAX_TICKS {
        return Ok(finish(n, Phase::Escaped, Reason::Timeout));
    }
    Ok(Transition { state: n, events })
}
