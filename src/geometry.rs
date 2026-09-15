//! Normalized geometry. No SVG, trigonometry, render APIs or named shapes.
use crate::types::{bounded, Result};
use serde::{Deserialize, Serialize};
pub type Point = [f64; 2];
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Capture {
    Rectangle,
    Polygon { rings: Vec<Vec<Point>> },
}
pub fn area(points: &[Point]) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let mut sum = 0.0;
    for (i, p) in points.iter().enumerate() {
        let n = points[(i + 1) % points.len()];
        sum += p[0] * n[1] - n[0] * p[1];
    }
    sum.abs() / 2.0
}
fn cross(a: Point, b: Point, c: Point) -> f64 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}
fn on_segment(a: Point, b: Point, p: Point) -> bool {
    cross(a, b, p) == 0.0
        && p[0] >= a[0].min(b[0])
        && p[0] <= a[0].max(b[0])
        && p[1] >= a[1].min(b[1])
        && p[1] <= a[1].max(b[1])
}
fn intersects(a: Point, b: Point, c: Point, d: Point) -> bool {
    let (u, v, w, z) = (
        cross(a, b, c),
        cross(a, b, d),
        cross(c, d, a),
        cross(c, d, b),
    );
    ((u > 0.0 && v < 0.0 || u < 0.0 && v > 0.0) && (w > 0.0 && z < 0.0 || w < 0.0 && z > 0.0))
        || on_segment(a, b, c)
        || on_segment(a, b, d)
        || on_segment(c, d, a)
        || on_segment(c, d, b)
}
fn inside(p: Point, ring: &[Point]) -> bool {
    let mut hit = false;
    let mut j = ring.len() - 1;
    for (i, a) in ring.iter().enumerate() {
        let b = ring[j];
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < (b[0] - a[0]) * (p[1] - a[1]) / (b[1] - a[1]) + a[0]
        {
            hit = !hit;
        }
        j = i;
    }
    hit
}
pub fn validate_capture(c: &Capture) -> Result<()> {
    let Capture::Polygon { rings } = c else {
        return Ok(());
    };
    if rings.is_empty() || rings.len() > 2 {
        return Err("capture needs one outer ring and at most one hole".into());
    }
    for ring in rings {
        if !(3..=48).contains(&ring.len()) {
            return Err("ring requires 3..48 vertices".into());
        }
        for p in ring {
            for v in p {
                bounded(*v, 0.0, 1.0, "capture coordinate")?;
            }
        }
        if area(ring) <= 1e-12 {
            return Err("degenerate capture ring".into());
        }
        for i in 0..ring.len() {
            let a = ring[i];
            let b = ring[(i + 1) % ring.len()];
            if a == b {
                return Err("duplicate adjacent vertices".into());
            }
            for j in i + 1..ring.len() {
                if j == i + 1 || (i == 0 && j == ring.len() - 1) {
                    continue;
                }
                if intersects(a, b, ring[j], ring[(j + 1) % ring.len()]) {
                    return Err("self-intersecting ring".into());
                }
            }
        }
    }
    if rings.len() == 2 {
        for (i, p) in rings[1].iter().enumerate() {
            if !inside(*p, &rings[0]) {
                return Err("hole must be strictly inside outer ring".into());
            }
            for (j, a) in rings[0].iter().enumerate() {
                if intersects(
                    *p,
                    rings[1][(i + 1) % rings[1].len()],
                    *a,
                    rings[0][(j + 1) % rings[0].len()],
                ) {
                    return Err("hole touches or crosses outer ring".into());
                }
            }
        }
    }
    Ok(())
}
// Sutherland–Hodgman: four bounded clipping passes. Shoelace bridges cancel
// for disconnected pieces of a clipped concave polygon; holes subtract area.
fn clip(points: Vec<Point>, axis: usize, bound: f64, greater: bool) -> Vec<Point> {
    let mut out = Vec::new();
    if points.is_empty() {
        return out;
    }
    let mut prev = *points.last().unwrap();
    let mut was = if greater {
        prev[axis] >= bound
    } else {
        prev[axis] <= bound
    };
    for p in points {
        let yes = if greater {
            p[axis] >= bound
        } else {
            p[axis] <= bound
        };
        if yes != was {
            let t = (bound - prev[axis]) / (p[axis] - prev[axis]);
            out.push([
                prev[0] + t * (p[0] - prev[0]),
                prev[1] + t * (p[1] - prev[1]),
            ]);
        }
        if yes {
            out.push(p);
        }
        prev = p;
        was = yes;
    }
    out
}
pub fn interval_alignment(fish: f64, tackle: f64, size: f64, radius: f64) -> f64 {
    let overlap =
        (fish + radius).min(tackle + size / 2.0) - (fish - radius).max(tackle - size / 2.0);
    (overlap / (2.0 * radius)).clamp(0.0, 1.0)
}
pub fn alignment(c: &Capture, fish: Point, tackle: Point, size: f64, radius: f64) -> f64 {
    if matches!(c, Capture::Rectangle) {
        return interval_alignment(fish[0], tackle[0], size, radius)
            * interval_alignment(fish[1], tackle[1], size, radius);
    }
    let Capture::Polygon { rings } = c else {
        unreachable!()
    };
    let left = tackle[0] - size / 2.0;
    let bottom = tackle[1] - size / 2.0;
    let (min_x, max_x) = (
        (fish[0] - radius - left) / size,
        (fish[0] + radius - left) / size,
    );
    let (min_y, max_y) = (
        (fish[1] - radius - bottom) / size,
        (fish[1] + radius - bottom) / size,
    );
    if max_x <= 0.0 || min_x >= 1.0 || max_y <= 0.0 || min_y >= 1.0 {
        return 0.0;
    }
    let mut coverage = 0.0;
    for (i, r) in rings.iter().enumerate() {
        let a = area(&clip(
            clip(
                clip(clip(r.clone(), 0, min_x, true), 0, max_x, false),
                1,
                min_y,
                true,
            ),
            1,
            max_y,
            false,
        ));
        if i == 0 {
            coverage = a
        } else {
            coverage -= a
        }
    }
    let side = 2.0 * radius / size;
    (coverage / (side * side)).clamp(0.0, 1.0)
}
