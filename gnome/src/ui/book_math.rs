use std::f64::consts::PI;

pub const STRIPS: usize = 32;
pub const DURATION_US: f64 = 460_000.0;

pub struct Strip {
    pub source: f32,
    pub x: f32,
    pub y: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub front: bool,
    pub shade: f32,
}

pub fn frame(width: f64, height: f64, progress: f64, forward: bool) -> Vec<Strip> {
    if width <= 0.0 || height <= 0.0 {
        return Vec::new();
    }
    let t = progress.clamp(0.0, 1.0);
    let p = t * t * t * (t * (t * 6.0 - 15.0) + 10.0);
    let bend = 0.65 * (PI * p).sin();
    let step = width / STRIPS as f64;
    let depth = width * 6.0;
    let (mut x, mut z) = (0.0, 0.0);
    (0..STRIPS)
        .map(|i| {
            let angle = PI * p + bend * (2.0 * (i as f64 + 0.5) / STRIPS as f64 - 1.0);
            let nx = x + step * angle.cos();
            let nz = z + step * angle.sin();
            let (a, b) = (depth / (depth - z), depth / (depth - nz));
            let left = width / 2.0 + (x - width / 2.0) * a;
            let right = width / 2.0 + (nx - width / 2.0) * b;
            let sy = (a + b) / 2.0;
            let strip = Strip {
                source: (if forward {
                    i as f64 * step
                } else {
                    width - (i + 1) as f64 * step
                }) as f32,
                x: (if forward { left } else { width - right }) as f32,
                y: (height * (1.0 - sy) / 2.0) as f32,
                scale_x: ((right - left) / step) as f32,
                scale_y: sy as f32,
                front: angle.cos() >= 0.0,
                shade: (0.18 * angle.sin().abs()) as f32,
            };
            (x, z) = (nx, nz);
            strip
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn flat_leaf_mirrored_bend_and_adjacent_strips() {
        for forward in [false, true] {
            let flat = frame(800.0, 600.0, 0.0, forward);
            for s in &flat {
                assert!((s.x - s.source).abs() < 0.001);
                assert!((s.scale_x - 1.0).abs() < 0.001 && s.front);
                assert!(s.y.abs() < 0.001 && s.shade == 0.0);
            }
            for i in 0..=100 {
                let f = frame(800.0, 600.0, i as f64 / 100.0, forward);
                assert!(
                    f.iter()
                        .all(|s| s.x.is_finite() && s.scale_y.is_finite() && s.scale_y > 0.0)
                );
                for pair in f.windows(2) {
                    let gap = if forward {
                        pair[0].x + 25.0 * pair[0].scale_x - pair[1].x
                    } else {
                        pair[1].x + 25.0 * pair[1].scale_x - pair[0].x
                    };
                    assert!(gap.abs() < 0.001);
                }
            }
        }
        let half = frame(800.0, 600.0, 0.5, true);
        assert!(half.iter().any(|s| s.front) && half.iter().any(|s| !s.front));
        assert!(frame(0.0, 600.0, 0.3, true).is_empty());
    }
}
