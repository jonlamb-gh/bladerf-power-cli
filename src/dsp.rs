use rustfft::num_complex::Complex;

/// Converts i16, in the range [-2048, 2048) to [-1.0, 1.0).
/// Note that the lower bound here is inclusive, and the upper bound is exclusive.
/// Samples should always be within [-2048, 2047].
pub fn normalize_sc16_q11(s: i16) -> f64 {
    debug_assert!(s >= -2048);
    debug_assert!(s < 2048);
    f64::from(s) / 2048.0
}

// dB = 10.0 * power.log(10.0)
// amplitude = power.sqrt()
pub fn sample_to_power(s: &Complex<f64>) -> f64 {
    s.norm_sqr()
}

pub fn sample_to_amplitude(s: &Complex<f64>) -> f64 {
    s.norm_sqr().sqrt()
}

// Samples.len() must be a multiple of 2, converts I/Q pair to complex
pub fn push_normalize_sc16_q11(samples: &[i16], vec: &mut Vec<Complex<f64>>) {
    debug_assert!(samples.len() % 2 == 0);
    samples.chunks(2).for_each(|pair| {
        let (i, q) = (normalize_sc16_q11(pair[0]), normalize_sc16_q11(pair[1]));
        vec.push(Complex::new(i, q));
    });
}

pub fn sample_to_db(s: &Complex<f64>) -> f64 {
    let power = sample_to_power(s);
    10.0 * power.log(10.0)
}
