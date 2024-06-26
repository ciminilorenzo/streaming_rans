use anyhow::{bail, Result};
use std::cmp::max;
use std::ops::Neg;

/// Tries to scale frequencies in `freqs` by using the new common denominator `new_frame`. This algorithm
/// gives priority to low frequency symbols in order to be sure that the space available is firstly given to symbols
/// with approximated frequency lower than 0. This happens when we are trying to approximate a distribution with a
/// frame size that is smaller compared to the original one.
///
/// # Returns
/// The approximated frequencies if is possibile to approximate with the given `new_frame` else, if too
/// many symbols have frequency lower than 1 - meaning that M is not big enough to handle the whole
/// set of symbols - an error is returned.
pub fn scale_freqs(
    freqs: &[usize],
    sorted_indices: &[usize],
    n: usize,
    mut m: usize,
    mut new_m: isize,
) -> Result<Vec<usize>> {
    let mut approx_freqs = freqs.to_vec();
    let ratio = new_m as f64 / m as f64;

    for (index, sym_index) in sorted_indices.iter().enumerate() {
        let sym_freq = freqs[*sym_index];
        let second_ratio = new_m as f64 / m as f64;
        let scale = (n - index) as f64 * ratio / n as f64 + index as f64 * second_ratio / n as f64;
        approx_freqs[*sym_index] = max(1, (0.5 + scale * sym_freq as f64).floor() as usize);

        new_m -= approx_freqs[*sym_index] as isize;
        m -= sym_freq;

        if new_m < 0 {
            bail!("Too many symbols have frequency lower than 1!. Need a bigger frame size");
        }
    }
    Ok(approx_freqs)
}

pub fn entropy(distr: &[usize], total_freq: f64) -> f64 {
    let mut entropy = 0.0;

    for freq in distr {
        let pr = *freq as f64 / total_freq;
        entropy += pr * f64::log2(pr);
    }
    entropy.neg()
}

/// Given the real probability distributions P and Q, calculates the cross entropy as follow:
/// ```text
/// cross-entropy(P|Q) = - ∑ p(x) * log(q(x))
/// ```
pub fn cross_entropy(distr: &[usize], m: f64, other_distr: &[usize], other_m: f64) -> f64 {
    assert_eq!(
        distr.len(),
        other_distr.len(),
        "Distr must have same length!"
    );

    let mut cross_entropy = 0.0;

    for index in 0..distr.len() {
        if distr[index] == 0 {
            continue;
        }
        let p_x = distr[index] as f64 / m;
        let q_x = other_distr[index] as f64 / other_m;
        cross_entropy += p_x * f64::log2(q_x);
    }
    cross_entropy.neg()
}

pub fn self_entropy(distr: &[usize], m: f64) -> f64 {
    let mut self_entropy = 0.0;

    distr.iter().for_each(|freq| {
        if *freq == 0 {
            return;
        }

        self_entropy += *freq as f64 * f64::log2(m / *freq as f64);
    });
    self_entropy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy() {
        let distr = [3_usize, 3, 4];
        assert_eq!("1.57", format!("{:.2}", entropy(&distr, 10_f64)));
    }

    #[test]
    fn test_distr_cross_entropy() {
        let distr = [3_usize, 3, 4];
        let other_distr = [4_usize, 2, 4];
        assert_eq!(
            "1.62",
            format!("{:.2}", cross_entropy(&distr, 10_f64, &other_distr, 10_f64))
        );
    }

    #[test]
    fn test_self_entropy() {
        let distr = [3, 3, 2, 1, 1];
        let m = 10;

        assert_eq!("21.71", format!("{:.2}", self_entropy(&distr, m as f64)));
    }
}
