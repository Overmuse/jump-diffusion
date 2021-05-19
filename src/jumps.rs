use crate::math::zscore;
use statrs::distribution::{InverseCDF, Normal};

pub fn jump_detected(y: &[f64]) -> bool {
    let z = zscore(y);
    let d = Normal::new(0.0, 1.0).expect("Standard normal");
    z.abs() > d.inverse_cdf(0.999)
}

pub fn find_jump(y: &[f64]) -> (usize, f64) {
    y.iter()
        .enumerate()
        .fold((0, y[0]), |(idx_max, val_max), (idx, val)| {
            if val_max.abs() > val.abs() {
                (idx_max, val_max)
            } else {
                (idx, *val)
            }
        })
}
