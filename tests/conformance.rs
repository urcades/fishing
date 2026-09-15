#![cfg(feature = "serde")]
use fishing::*;
use serde_json::Value;
// Fixtures remain schema 1. Only explicit metadata/default-field migration
// is applied; numeric checkpoints, RNG and event ordering are never rewritten.
fn legacy(mut v: Value) -> Value {
    match &mut v {
        Value::Object(m) => {
            for key in [
                "segmentIndex",
                "nibblesLeft",
                "pattern",
                "nibbles",
                "maxTicks",
            ] {
                m.remove(key);
            }
            if m.get("version") == Some(&Value::from(2)) {
                m.insert("version".into(), Value::from(1));
            }
            for value in m.values_mut() {
                *value = legacy(value.take());
            }
        }
        Value::Array(a) => {
            for value in a {
                *value = legacy(value.take());
            }
        }
        _ => {}
    }
    v
}
fn upgraded(mut v: Value) -> Value {
    if let Some(m) = v.as_object_mut() {
        if m.get("version") == Some(&Value::from(1)) {
            m.insert("version".into(), Value::from(2));
        }
    }
    v
}

fn close(a: &Value, b: &Value, path: &str) {
    match (a, b) {
        (Value::Number(x), Value::Number(y)) => assert!(
            (x.as_f64().unwrap() - y.as_f64().unwrap()).abs() <= 1e-12,
            "{path}: {x} != {y}"
        ),
        (Value::Object(x), Value::Object(y)) => {
            assert_eq!(x.len(), y.len(), "{path}: key count");
            for (k, v) in x {
                close(
                    v,
                    y.get(k).unwrap_or_else(|| panic!("{path}: missing {k}")),
                    &format!("{path}.{k}"),
                );
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            assert_eq!(x.len(), y.len(), "{path}");
            for (i, (v, w)) in x.iter().zip(y).enumerate() {
                close(v, w, &format!("{path}[{i}]"));
            }
        }
        _ => assert_eq!(a, b, "{path}"),
    }
}
fn wire<T: serde::Serialize>(v: T) -> Value {
    legacy(serde_json::to_value(v).unwrap())
}
#[test]
fn accepted_demo_trajectories_and_all_events() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/trajectories.json")).unwrap();
    for case in fixture["traces"].as_array().unwrap() {
        let config: Config = serde_json::from_value(case["config"].clone()).unwrap();
        let mut state = create_state(case["seed"].as_u64().unwrap() as u32, &config).unwrap();
        let checkpoints = case["checkpoints"].as_array().unwrap();
        close(
            &wire(&state),
            &checkpoints[0]["state"],
            case["id"].as_str().unwrap(),
        );
        for (i, input) in case["inputs"].as_array().unwrap().iter().enumerate() {
            let before = state.clone();
            let r = step(
                &state,
                serde_json::from_value(input.clone()).unwrap(),
                &config,
            )
            .unwrap();
            assert_eq!(state, before);
            r.state.validate(&config).unwrap();
            if let Some(c) = checkpoints
                .iter()
                .find(|c| c["at"].as_u64() == Some(i as u64 + 1))
            {
                close(
                    &wire(&r.state),
                    &c["state"],
                    &format!("{} tick {}", case["id"], i + 1),
                );
                assert_eq!(wire(&r.events), c["events"]);
                // Every stored checkpoint is a valid resumable JSON snapshot.
                let resumed: State =
                    serde_json::from_value(serde_json::to_value(&r.state).unwrap()).unwrap();
                assert_eq!(resumed, r.state);
            } else {
                assert!(
                    r.events.is_empty(),
                    "unexpected event in {} at {}",
                    case["id"],
                    i + 1
                );
            }
            state = r.state;
        }
        assert!(state.is_terminal());
    }
}
#[test]
fn threshold_expiry_boundary_and_terminal_fixtures() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/trajectories.json")).unwrap();
    for case in fixture["steps"].as_array().unwrap() {
        let config: Config = serde_json::from_value(case["config"].clone()).unwrap();
        let state: State = serde_json::from_value(upgraded(case["state"].clone())).unwrap();
        let result = step(
            &state,
            serde_json::from_value(case["input"].clone()).unwrap(),
            &config,
        )
        .unwrap();
        close(
            &wire(result),
            &case["expected"],
            case["id"].as_str().unwrap(),
        );
    }
}
#[test]
fn canonical_polygon_fixtures() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/geometry.json")).unwrap();
    for c in fixture.as_array().unwrap() {
        let shape: geometry::Capture = serde_json::from_value(c["capture"].clone()).unwrap();
        geometry::validate_capture(&shape).unwrap();
        let n = |k: &str| c[k].as_f64().unwrap();
        let q = geometry::alignment(
            &shape,
            [n("fishX"), n("fishY")],
            [n("tackleX"), n("tackleY")],
            n("size"),
            n("radius"),
        );
        close(&wire(q), &c["expected"], c["id"].as_str().unwrap());
    }
}
#[test]
fn reject_invalid_geometry_instead_of_guessing_fill_rules() {
    use geometry::Capture::Polygon;
    for rings in [
        vec![vec![[0.0, 0.0], [1.0, 1.0], [0.0, 1.0], [1.0, 0.0]]],
        vec![
            vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
            vec![[0.0, 0.0], [0.5, 0.0], [0.5, 0.5]],
        ],
    ] {
        assert!(geometry::validate_capture(&Polygon { rings }).is_err());
    }
}
#[test]
fn profile_resolution_and_weighted_selection_fixtures() {
    let fixture: Value = serde_json::from_str(include_str!("fixtures/resolution.json")).unwrap();
    for c in fixture["resolve"].as_array().unwrap() {
        let d: Definition = serde_json::from_value(c["definition"].clone()).unwrap();
        let l: Loadout = serde_json::from_value(c["loadout"].clone()).unwrap();
        close(
            &wire(resolve(&d, &l, c["seed"].as_u64().unwrap() as u32).unwrap()),
            &c["expected"],
            c["id"].as_str().unwrap(),
        );
    }
    for c in fixture["select"].as_array().unwrap() {
        let pool: Vec<Fish> = serde_json::from_value(c["pool"].clone()).unwrap();
        let bait: Bait = serde_json::from_value(c["bait"].clone()).unwrap();
        close(
            &wire(select(&pool, &bait, c["seed"].as_u64().unwrap() as u32).unwrap()),
            &c["expected"],
            c["id"].as_str().unwrap(),
        );
    }
}

#[test]
fn hand_derived_extension_checkpoints() {
    fn subset(actual: &Value, expected: &Value) {
        if let Value::Object(fields) = expected {
            for (key, value) in fields {
                subset(&actual[key], value);
            }
        } else {
            close(actual, expected, "extension");
        }
    }
    let fixtures: Value = serde_json::from_str(include_str!("fixtures/extensions.json")).unwrap();
    for case in fixtures["cases"].as_array().unwrap() {
        let config: Config = serde_json::from_value(case["config"].clone()).unwrap();
        let mut state = create_state(case["seed"].as_u64().unwrap() as u32, &config).unwrap();
        for (i, input) in case["inputs"].as_array().unwrap().iter().enumerate() {
            let next = step(
                &state,
                serde_json::from_value(input.clone()).unwrap(),
                &config,
            )
            .unwrap();
            next.state.validate(&config).unwrap();
            for check in case["checks"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|v| v["tick"].as_u64() == Some(i as u64 + 1))
            {
                subset(&serde_json::to_value(&next.state).unwrap(), &check["state"]);
                assert_eq!(serde_json::to_value(&next.events).unwrap(), check["events"]);
            }
            let snapshot = serde_json::to_string(&next.state).unwrap();
            state = serde_json::from_str(&snapshot).unwrap();
        }
    }
}

#[test]
fn configured_profiles_use_broad_timing_and_fish_patterns() {
    let fixtures: Value = serde_json::from_str(include_str!("fixtures/resolution.json")).unwrap();
    let case = &fixtures["resolve"][0];
    let mut definition: Definition = serde_json::from_value(case["definition"].clone()).unwrap();
    let mut loadout: Loadout = serde_json::from_value(case["loadout"].clone()).unwrap();
    definition.timing = TimingPolicy::Configured;
    definition.parameters.wait_min = 20.0;
    definition.parameters.wait_max = 20.0;
    loadout.bait.attraction = 1.0;
    loadout.bait.affinity.clear();
    loadout.bait.bite_bonus = 0.0;
    loadout.fish.bite_window = 4.0;
    definition.hook_bonus = 0.0;
    loadout.fish.pattern = vec![Segment::default()];
    let result = resolve(&definition, &loadout, 42).unwrap();
    assert_eq!(result.config.parameters.wait_min, 20.0);
    assert_eq!(result.config.parameters.bite_window, 4.0);
    if definition.mode != Mode::Hook {
        assert_eq!(result.config.pattern, loadout.fish.pattern);
    }
}

#[test]
fn target_rules_reject_unknown_members() {
    // Empty struct variants matter: Serde unit variants ignore extra fields.
    for kind in ["keep", "hold", "opposite"] {
        let invalid = serde_json::json!({"kind": kind, "distance": 1});
        assert!(serde_json::from_value::<TargetRule>(invalid).is_err());
    }
}
