use fishing::{geometry::Capture, *};

fn config(mode: Mode, dimensions: u8) -> Config {
    Config {
        mode,
        dimensions,
        parameters: Parameters::default(),
        capture: Capture::Rectangle,
    }
}

#[test]
fn all_mechanisms_are_pure_bounded_and_replayable() {
    for (mode, dimensions) in [
        (Mode::Hook, 0),
        (Mode::Pressure, 0),
        (Mode::Tracking, 1),
        (Mode::Tracking, 2),
    ] {
        let rules = config(mode, dimensions);
        let initial = create_state(42, &rules).unwrap();
        assert_eq!(
            step(&initial, Input::default(), &rules).unwrap().state,
            initial
        );
        let mut state = initial;
        for tick in 0..MAX_TICKS {
            // Fresh cast/hook edges, followed by alternating adversarial inputs.
            let input = Input {
                primary: if matches!(state.phase, Phase::Ready | Phase::Bite)
                    || (state.phase == Phase::Struggle && tick % 2 == 0)
                {
                    1.0
                } else {
                    0.0
                },
                steer: if tick % 3 == 0 { -1.0 } else { 1.0 },
            };
            let before = state.clone();
            let next = step(&state, input, &rules).unwrap();
            assert_eq!(before, state);
            assert_eq!(next, step(&before, input, &rules).unwrap());
            next.state.validate(&rules).unwrap();
            assert!(next.state.tick <= MAX_TICKS);
            state = next.state;
            if state.is_terminal() {
                break;
            }
        }
        assert!(state.is_terminal());
        let next = step(
            &state,
            Input {
                primary: 1.0,
                steer: -1.0,
            },
            &rules,
        )
        .unwrap();
        assert_eq!(next.state, state);
        assert!(next.events.is_empty());
    }
}

#[test]
fn fresh_hook_edges_and_deadlines_are_explicit() {
    let rules = config(Mode::Hook, 0);
    let mut state = create_state(0, &rules).unwrap();
    state = step(
        &state,
        Input {
            primary: 1.0,
            steer: 0.0,
        },
        &rules,
    )
    .unwrap()
    .state;
    // Holding continuously cannot auto-hook when the bite arrives.
    while state.phase == Phase::Waiting {
        state = step(
            &state,
            Input {
                primary: 1.0,
                steer: 0.0,
            },
            &rules,
        )
        .unwrap()
        .state;
    }
    assert_eq!(
        step(
            &state,
            Input {
                primary: 1.0,
                steer: 0.0
            },
            &rules
        )
        .unwrap()
        .state
        .phase,
        Phase::Bite
    );
    state = step(&state, Input::default(), &rules).unwrap().state;
    let caught = step(
        &state,
        Input {
            primary: 1.0,
            steer: 0.0,
        },
        &rules,
    )
    .unwrap();
    assert_eq!(caught.events, vec![Event::Hooked, Event::Caught]);
    assert_eq!(caught.state.reason, Some(Reason::Landed));
    state.phase_ticks = state.duration - 1;
    state.tick = state.tick.max(state.phase_ticks);
    let missed = step(
        &state,
        Input {
            primary: 1.0,
            steer: 0.0,
        },
        &rules,
    )
    .unwrap();
    assert_eq!(missed.state.reason, Some(Reason::MissedBite));
}

#[test]
fn invalid_rules_states_and_nonfinite_inputs_are_rejected() {
    for (mode, dimensions) in [
        (Mode::Hook, 1),
        (Mode::Pressure, 2),
        (Mode::Tracking, 0),
        (Mode::Tracking, 3),
    ] {
        assert!(create_state(42, &config(mode, dimensions)).is_err());
    }
    let rules = config(Mode::Tracking, 2);
    let mut state = create_state(42, &rules).unwrap();
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(step(
            &state,
            Input {
                primary: invalid,
                steer: 0.0
            },
            &rules
        )
        .is_err());
        assert!(step(
            &state,
            Input {
                primary: 0.0,
                steer: invalid
            },
            &rules
        )
        .is_err());
    }
    state.energy = 2.0;
    assert!(observe(&state, &rules).is_err());
}

#[test]
fn geometric_holes_change_capture_and_invalid_rings_are_rejected() {
    let ring = Capture::Polygon {
        rings: vec![
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]],
        ],
    };
    geometry::validate_capture(&ring).unwrap();
    assert_eq!(
        geometry::alignment(&ring, [0.5, 0.5], [0.5, 0.5], 0.4, 0.03),
        0.0
    );
    assert_eq!(
        geometry::alignment(&Capture::Rectangle, [0.5, 0.5], [0.5, 0.5], 0.4, 0.03),
        1.0
    );
    assert!(geometry::validate_capture(&Capture::Polygon { rings: vec![] }).is_err());
}

#[test]
fn specified_rng_uses_wrapping_u32_arithmetic() {
    assert_eq!(random(0).0, 1_013_904_223);
    assert_eq!(random(u32::MAX).0, 1_012_239_698);
    for seed in [0, 1, 42, u32::MAX] {
        assert!((0.0..1.0).contains(&random(seed).1));
    }
}
