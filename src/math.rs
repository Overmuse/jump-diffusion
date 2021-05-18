use itertools::Itertools;
use statrs::function::gamma::gamma;
use std::f64::consts::{FRAC_PI_2, PI};

pub fn mu(x: f64) -> f64 {
    f64::powf(2.0, x / 2.0) * gamma((x + 1.0) / 2.0) / gamma(0.5)
}

pub fn realized_variance(y: &[f64]) -> f64 {
    y.iter().map(|&x| f64::powi(x, 2)).sum()
}

pub fn bipower_variation(y: &[f64]) -> f64 {
    y.iter()
        .tuple_windows()
        .map(|(&a, &b)| f64::abs(a) * f64::abs(b))
        .sum::<f64>()
        * f64::powi(mu(1.0), -2)
}

pub fn tripower_quarticity(y: &[f64]) -> f64 {
    let len = y.len() as f64;
    y.iter()
        .tuple_windows::<(_, _, _)>()
        .map(|(&a, &b, &c)| {
            f64::powf(a.abs(), 4.0 / 3.0)
                * f64::powf(b.abs(), 4.0 / 3.0)
                * f64::powf(c.abs(), 4.0 / 3.0)
        })
        .sum::<f64>()
        * f64::powi(mu(4.0 / 3.0), -3)
        * f64::powi(len, 2)
        / (len - 2.0)
}

pub fn zscore(y: &[f64]) -> f64 {
    let rv = realized_variance(y);
    let bpv = bipower_variation(y);
    let tp = tripower_quarticity(y);

    let numerator = (rv - bpv) / rv;
    let denominator = f64::powf(
        (f64::powi(FRAC_PI_2, 2) + PI - 5.0) / y.len() as f64
            * f64::max(1.0, tp / f64::powi(bpv, 2)),
        0.5,
    );
    numerator / denominator
}
