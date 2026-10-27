use std::f64::consts::PI;

pub const STRIPS: usize = 64;
pub const DURATION_US: f64 = 1_100_000.0;
pub const ANGLE: f64 = -0.42;

pub struct Layout {
    pub nx: f64,
    pub ny: f64,
    pub min_u: f64,
    pub span_u: f64,
    pub min_v: f64,
    pub span_v: f64,
}
impl Layout {
    pub fn step(&self) -> f64 {
        self.span_u / STRIPS as f64
    }
}
pub struct Strip {
    pub source: f64,
    pub x: f64,
    pub y: f64,
    pub scale_x: f64,
    pub front: f64,
    pub shade_start: f64,
    pub shade_end: f64,
}
pub struct Shadow {
    pub x: f64,
    pub y: f64,
    pub opacity: f64,
}
pub struct Sheet {
    pub layout: Layout,
    pub strips: Vec<Strip>,
    pub shadow: Shadow,
}

fn paper(width: f64, height: f64) -> Layout {
    Layout {
        nx: ANGLE.cos(),
        ny: ANGLE.sin(),
        min_u: ANGLE.sin() * height,
        span_u: ANGLE.cos() * width - ANGLE.sin() * height,
        min_v: 0.0,
        span_v: -ANGLE.sin() * width + ANGLE.cos() * height,
    }
}
fn ease(progress: f64) -> f64 {
    let t = if progress.is_finite() {
        progress.clamp(0.0, 1.0)
    } else {
        0.0
    };
    t * t * (3.0 - 2.0 * t)
}
fn bend(u: f64, fold: f64, radius: f64) -> (f64, f64, f64, f64) {
    let d = u - fold;
    if d <= 0.0 {
        return (u, 0.0, 1.0, 0.0);
    }
    let a = (d / radius).min(PI);
    (
        fold + radius * a.sin() - (d - PI * radius).max(0.0),
        radius * (1.0 - a.cos()),
        a.cos(),
        0.24 * a.sin().abs() + if a > PI / 2.0 { 0.035 } else { 0.0 },
    )
}

/// A diagonal cylindrical fold; flat paper is unchanged until the crease reaches
/// it. Each affine band meets the next exactly and samples one shared texture.
pub fn frame(width: f64, height: f64, progress: f64, forward: bool) -> Sheet {
    if !width.is_finite() || !height.is_finite() || width <= 0.0 || height <= 0.0 {
        return Sheet {
            layout: paper(0.0, 0.0),
            strips: vec![],
            shadow: Shadow {
                x: 0.0,
                y: 0.0,
                opacity: 0.0,
            },
        };
    }
    let l = paper(width, height);
    let p = ease(progress);
    let radius = width * (0.075 + 0.065 * (PI * p).sin());
    let fold = l.min_u + l.span_u - (l.span_u + width * 0.88) * p;
    let strips = (0..STRIPS)
        .map(|i| {
            let u = l.min_u + i as f64 * l.step();
            let a = bend(u, fold, radius);
            let b = bend(u + l.step(), fold, radius);
            let middle = bend(u + l.step() / 2.0, fold, radius);
            let x = l.nx * a.0 - l.ny * l.min_v;
            Strip {
                source: u,
                x: if forward { x } else { width - x },
                y: l.ny * a.0 + l.nx * l.min_v,
                scale_x: (b.0 - a.0) / l.step(),
                front: if middle.2 >= 0.0 { 1.0 } else { 0.0 },
                shade_start: a.3,
                shade_end: b.3,
            }
        })
        .collect();
    let shadow_u = fold - radius * 0.12;
    let shadow_x = l.nx * shadow_u - l.ny * l.min_v;
    let shadow = Shadow {
        x: if forward { shadow_x } else { width - shadow_x },
        y: l.ny * shadow_u + l.nx * l.min_v,
        opacity: (PI * p).sin(),
    };
    Sheet {
        layout: l,
        strips,
        shadow,
    }
}

#[cfg(test)]
fn project(
    width: f64,
    height: f64,
    x: f64,
    y: f64,
    progress: f64,
    forward: bool,
) -> (f64, f64, f64) {
    let l = paper(width, height);
    let p = ease(progress);
    let x = if forward { x } else { width - x };
    let u = l.nx * x + l.ny * y;
    let v = -l.ny * x + l.nx * y;
    let b = bend(
        u,
        l.min_u + l.span_u - (l.span_u + width * 0.88) * p,
        width * (0.075 + 0.065 * (PI * p).sin()),
    );
    let px = l.nx * b.0 - l.ny * v;
    (
        if forward { px } else { width - px },
        l.ny * b.0 + l.nx * v,
        b.1,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn corner_leads_bottom_and_reverse_mirrors() {
        let top = project(800.0, 600.0, 800.0, 0.0, 0.22, true);
        let bottom = project(800.0, 600.0, 800.0, 600.0, 0.22, true);
        assert!(top.2 > 30.0 && top.0 < 790.0 && top.1 > 0.0);
        assert!(
            (bottom.0 - 800.0).abs() < 0.001 && (bottom.1 - 600.0).abs() < 0.001 && bottom.2 == 0.0
        );
        for step in 0..=100 {
            let t = step as f64 / 100.0;
            let a = project(800.0, 600.0, 725.0, 180.0, t, true);
            let b = project(800.0, 600.0, 75.0, 180.0, t, false);
            assert!(
                (a.0 + b.0 - 800.0).abs() < 0.001
                    && (a.1 - b.1).abs() < 0.001
                    && (a.2 - b.2).abs() < 0.001
            );
        }
    }
    #[test]
    fn diagonal_paper_joins_and_exits() {
        for (w, h) in [(800.0, 600.0), (420.0, 900.0), (1400.0, 400.0)] {
            for forward in [false, true] {
                let sign = if forward { 1.0 } else { -1.0 };
                for step in 0..=100 {
                    let f = frame(w, h, step as f64 / 100.0, forward);
                    let l = &f.layout;
                    assert_eq!(f.strips.len(), STRIPS);
                    for pair in f.strips.windows(2) {
                        let a = &pair[0];
                        let b = &pair[1];
                        assert!(a.x.is_finite() && a.y.is_finite() && a.scale_x.is_finite());
                        assert!((a.x + sign * l.nx * l.step() * a.scale_x - b.x).abs() < 0.001);
                        assert!((a.y + l.ny * l.step() * a.scale_x - b.y).abs() < 0.001);
                        assert!(a.shade_start >= 0.0 && a.shade_end <= 0.28);
                    }
                }
                for x in [0.0, w / 2.0, w] {
                    for y in [0.0, h / 2.0, h] {
                        let flat = project(w, h, x, y, 0.0, forward);
                        assert!((flat.0 - x).abs() < 0.001 && (flat.1 - y).abs() < 0.001);
                        let end = project(w, h, x, y, 1.0, forward);
                        assert!(end.0 < 0.0 || end.0 > w || end.1 < 0.0 || end.1 > h);
                    }
                }
            }
        }
        assert!(frame(0.0, 600.0, 0.5, true).strips.is_empty());
    }
}
