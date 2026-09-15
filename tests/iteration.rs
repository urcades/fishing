use fishing::{geometry::Capture, *};

fn rules(mode: Mode) -> Config {
    Config {
        mode,
        dimensions: if mode == Mode::Tracking { 2 } else { 0 },
        capture: Capture::Rectangle,
        ..Config::default()
    }
}
fn advance(s: &State, c: &Config, primary: f64) -> Transition {
    step(
        s,
        Input {
            primary,
            steer: 0.0,
        },
        c,
    )
    .unwrap()
}
fn hooked(c: &Config) -> State {
    let mut s = advance(&create_state(42, c).unwrap(), c, 1.0).state;
    while s.phase != Phase::Bite {
        s = advance(&s, c, 0.0).state;
    }
    advance(&s, c, 1.0).state
}
#[test]
fn broad_tuning_is_valid_but_not_recommended() {
    let mut c = rules(Mode::Hook);
    c.parameters.bite_window = 4.0;
    c.parameters.wait_min = 20.0;
    c.parameters.wait_max = 20.0;
    c.max_ticks = 2400;
    assert!(c.validate().is_ok());
    assert!(c.parameters.validate_recommended().is_err());
    let s = hooked(&c);
    assert_eq!(s.phase, Phase::Caught);
    assert_eq!(s.tick, 1202);
}
#[test]
fn duplicate_segments_have_distinct_state_and_rates() {
    let mut c = rules(Mode::Tracking);
    c.pattern = vec![
        Segment {
            behavior: Behavior::Surge,
            intensity: 0.2,
            pace: 0.5,
            duration: DT,
            jitter: 0.0,
            target: TargetRule::Point { point: [0.2, 0.8] },
        },
        Segment {
            behavior: Behavior::Surge,
            intensity: 1.5,
            pace: 2.0,
            duration: 1.0,
            jitter: 0.0,
            target: TargetRule::Point { point: [0.8, 0.2] },
        },
    ];
    let s = hooked(&c);
    assert_eq!(s.segment_index, 0);
    assert_eq!(s.motion.as_ref().unwrap().fish_target, 0.8);
    let first = observe(&s, &c).unwrap().pull;
    let r = advance(&s, &c, 0.0);
    assert_eq!(r.state.segment_index, 1);
    assert_eq!(r.events, vec![Event::Surge]);
    assert_eq!(r.state.motion.as_ref().unwrap().fish_target, 0.2);
    assert!(observe(&r.state, &c).unwrap().pull > first);
    let (_, draw) = random(s.rng);
    let _ = draw;
    assert_eq!(r.state.rng, random(s.rng).0);
}
#[test]
fn nibbles_are_false_bites_with_explicit_expiry_and_resume() {
    let mut c = rules(Mode::Hook);
    c.parameters.wait_min = DT;
    c.parameters.wait_max = DT;
    c.nibbles = Nibbles {
        count: 2,
        duration: 3.0 * DT,
        gap: DT,
    };
    let cast = advance(&create_state(7, &c).unwrap(), &c, 1.0).state;
    let r = advance(&cast, &c, 0.0);
    assert_eq!(r.events, vec![Event::Nibble]);
    let nibble = r.state;
    assert_eq!(nibble.nibbles_left, 1);
    assert_eq!(
        advance(&nibble, &c, 1.0).state.reason,
        Some(Reason::EarlyHook)
    );
    let mut s = nibble.clone();
    for _ in 0..3 {
        s = advance(&s, &c, 0.0).state;
    }
    assert_eq!(s.phase, Phase::Waiting);
    s = advance(&s, &c, 0.0).state;
    assert_eq!(s.phase, Phase::Nibble);
    assert_eq!(s.nibbles_left, 0);
    for _ in 0..4 {
        s = advance(&s, &c, 0.0).state;
    }
    assert_eq!(s.phase, Phase::Bite);
    assert_eq!(advance(&s, &c, 1.0).state.phase, Phase::Caught);
    let mut end = nibble;
    end.phase_ticks = 2;
    end.tick += 2;
    assert_eq!(advance(&end, &c, 1.0).state.phase, Phase::Waiting); // Expiry before press.
}
#[test]
fn custom_limits_and_invalid_programs_are_checked() {
    let mut c = rules(Mode::Pressure);
    c.max_ticks = 2;
    let s = advance(&create_state(0, &c).unwrap(), &c, 1.0).state;
    assert_eq!(advance(&s, &c, 0.0).state.reason, Some(Reason::Timeout));
    c.max_ticks = 0;
    assert!(c.validate().is_err());
    c.max_ticks = 3600;
    c.pattern = vec![Segment::default(); 17];
    assert!(c.validate().is_err());
    c.pattern = vec![Segment {
        duration: 0.0,
        ..Segment::default()
    }];
    assert!(c.validate().is_err());
    c.pattern = vec![Segment {
        target: TargetRule::Point {
            point: [f64::NAN, 0.5],
        },
        ..Segment::default()
    }];
    assert!(c.validate().is_err());
}

#[test]
fn zero_rates_reach_the_absolute_bound_without_drift() {
    let mut c = rules(Mode::Pressure);
    c.max_ticks = MAX_TICKS;
    c.parameters.wait_min = DT;
    c.parameters.wait_max = DT;
    c.parameters.reel_rate = 0.0;
    c.parameters.escape_rate = 0.0;
    c.parameters.strength = 0.0;
    c.parameters.base_tension = 0.0;
    c.parameters.fatigue = 0.0;
    c.parameters.recovery = 0.0;
    c.pattern = vec![Segment {
        duration: 600.0,
        jitter: 1.0,
        ..Segment::default()
    }];
    let mut s = hooked(&c);
    while !s.is_terminal() {
        s = advance(&s, &c, 0.0).state;
        s.validate(&c).unwrap();
    }
    assert_eq!(s.tick, MAX_TICKS);
    assert_eq!(s.reason, Some(Reason::Timeout));
    assert_eq!((s.progress, s.tension, s.energy), (0.25, 0.0, 1.0));
    assert_eq!(advance(&s, &c, 1.0).state, s);
}

#[test]
fn holding_through_nibbles_does_not_create_a_hook_edge() {
    let mut c = rules(Mode::Hook);
    c.parameters.wait_min = DT;
    c.parameters.wait_max = DT;
    c.parameters.bite_window = 3.0 * DT;
    c.nibbles = Nibbles {
        count: 2,
        duration: DT,
        gap: DT,
    };
    let mut s = create_state(42, &c).unwrap();
    while !s.is_terminal() {
        s = advance(&s, &c, 1.0).state;
    }
    assert_eq!(s.reason, Some(Reason::MissedBite));
}

#[test]
fn every_target_rule_is_bounded_and_consumes_only_entry_draws() {
    let targets = [
        TargetRule::Keep {},
        TargetRule::Hold {},
        TargetRule::Wander { distance: 1.0 },
        TargetRule::Point { point: [0.0, 1.0] },
        TargetRule::Opposite {},
    ];
    for dimensions in 0..=2 {
        for target in &targets {
            let mut c = rules(if dimensions == 0 {
                Mode::Pressure
            } else {
                Mode::Tracking
            });
            c.dimensions = dimensions;
            c.parameters.wait_min = DT;
            c.parameters.wait_max = DT;
            c.pattern = vec![Segment {
                duration: 600.0,
                target: target.clone(),
                ..Segment::default()
            }];
            let s = hooked(&c);
            // One waiting draw, one segment duration draw, then one target draw per active axis.
            let draws = 2
                + if matches!(target, TargetRule::Wander { .. } | TargetRule::Opposite {}) {
                    dimensions
                } else {
                    0
                };
            let mut seed = 42;
            for _ in 0..draws {
                seed = random(seed).0;
            }
            assert_eq!(s.rng, seed);
            let next = advance(&s, &c, 0.0).state;
            assert_eq!(next.rng, seed);
            next.validate(&c).unwrap();
            if let Some(m) = &s.motion {
                if matches!(target, TargetRule::Point { .. }) {
                    assert_eq!(m.fish_target, 0.97);
                    assert_eq!(m.fish_target_x, if dimensions == 2 { 0.03 } else { 0.5 });
                }
            }
        }
    }
}

#[test]
fn extreme_effort_is_finite_and_resolves_before_clamping() {
    let mut c = rules(Mode::Tracking);
    c.parameters.strength = 100.0;
    c.parameters.line_capacity = 0.01;
    c.parameters.response = DT;
    c.parameters.base_tension = 100.0;
    c.parameters.fish_speed = 60.0;
    c.parameters.tackle_acceleration = 3600.0;
    c.parameters.tackle_speed = 60.0;
    c.pattern = vec![Segment {
        intensity: 100.0,
        pace: 10.0,
        ..Segment::default()
    }];
    let s = hooked(&c);
    assert!(observe(&s, &c).unwrap().pull.is_finite());
    let r = advance(&s, &c, 1.0);
    assert_eq!(r.state.reason, Some(Reason::LineBroke));
    r.state.validate(&c).unwrap();
    let mut invalid = s.clone();
    invalid.segment_index = 1;
    assert!(step(&invalid, Input::default(), &c).is_err());
    invalid = s;
    invalid.behavior = Behavior::Surge;
    assert!(step(&invalid, Input::default(), &c).is_err());
}
