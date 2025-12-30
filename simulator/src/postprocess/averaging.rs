//! Multi-trace averaging simulation
//!
//! This module simulates the averaging process that OTDRs use to improve
//! signal-to-noise ratio. Real OTDRs acquire many individual traces and
//! average them together.
//!
//! For simulation purposes, we typically generate a single trace with
//! the expected noise level after averaging, but this module provides
//! the option to simulate the actual averaging process.

use rand::SeedableRng;
use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Simulate averaging by generating multiple noisy traces and averaging
/// 
/// This is the "accurate but slow" method that actually generates N traces.
/// For most uses, add_noise with reduced sigma is preferred.
/// 
/// Args:
/// - base_trace: The clean trace (without noise)
/// - noise_std_per_trace: Noise standard deviation per individual trace
/// - num_averages: Number of traces to average
/// - seed: Random seed for reproducibility
/// 
/// Returns: Averaged trace
pub fn simulate_averaging(
    base_trace: &[f64],
    noise_std_per_trace: f64,
    num_averages: usize,
    seed: Option<u64>,
) -> Vec<f64> {
    if num_averages == 0 {
        return base_trace.to_vec();
    }
    
    let mut rng: ChaCha8Rng = match seed {
        Some(s) => ChaCha8Rng::seed_from_u64(s),
        None => ChaCha8Rng::from_entropy(),
    };
    
    let n = base_trace.len();
    let mut sum = vec![0.0; n];
    
    // Generate and sum multiple noisy traces
    for _ in 0..num_averages {
        for i in 0..n {
            let noise = generate_gaussian(&mut rng);
            sum[i] += base_trace[i] + noise * noise_std_per_trace;
        }
    }
    
    // Average
    let num_avg_f64 = num_averages as f64;
    sum.iter().map(|&s| s / num_avg_f64).collect()
}

/// Generate a single Gaussian random number using Box-Muller transform
fn generate_gaussian(rng: &mut ChaCha8Rng) -> f64 {
    let u1: f64 = rng.gen_range(0.0001..1.0); // Avoid exactly 0 for ln()
    let u2: f64 = rng.gen_range(0.0..1.0);
    
    (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos()
}

/// Calculate equivalent noise level after averaging
/// 
/// The noise standard deviation reduces by √N with N averages.
/// 
/// Args:
/// - noise_std_per_trace: Noise level of a single trace
/// - num_averages: Number of averages
/// 
/// Returns: Effective noise standard deviation after averaging
pub fn noise_after_averaging(noise_std_per_trace: f64, num_averages: f64) -> f64 {
    noise_std_per_trace / num_averages.sqrt()
}

/// Calculate dynamic range improvement from averaging
/// 
/// Dynamic range improves by 5×log10(N) dB with N averages.
/// 
/// Args:
/// - num_averages: Number of averages
/// 
/// Returns: Dynamic range improvement in dB
pub fn dynamic_range_improvement(num_averages: f64) -> f64 {
    5.0 * num_averages.log10()
}

/// Calculate required averaging time for target dynamic range improvement
/// 
/// Args:
/// - target_improvement_db: Desired improvement in dB
/// - trace_rate_hz: Acquisition rate in Hz
/// 
/// Returns: Averaging time in seconds
pub fn averaging_time_for_improvement(target_improvement_db: f64, trace_rate_hz: f64) -> f64 {
    // DR_improvement = 5 × log10(N)
    // N = 10^(DR/5)
    let num_averages = 10.0_f64.powf(target_improvement_db / 5.0);
    num_averages / trace_rate_hz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simulate_averaging_reduces_noise() {
        let base_trace = vec![1e-6; 1000];
        let noise_per_trace = 1e-8;
        
        // Single trace
        let avg_1 = simulate_averaging(&base_trace, noise_per_trace, 1, Some(42));
        
        // 100 trace average
        let avg_100 = simulate_averaging(&base_trace, noise_per_trace, 100, Some(42));
        
        // Calculate variance of both
        let mean_1: f64 = avg_1.iter().sum::<f64>() / avg_1.len() as f64;
        let var_1: f64 = avg_1.iter().map(|x| (x - mean_1).powi(2)).sum::<f64>() 
            / avg_1.len() as f64;
        
        let mean_100: f64 = avg_100.iter().sum::<f64>() / avg_100.len() as f64;
        let var_100: f64 = avg_100.iter().map(|x| (x - mean_100).powi(2)).sum::<f64>() 
            / avg_100.len() as f64;
        
        // Variance should reduce by factor of ~100
        assert!(var_1 / var_100 > 50.0); // Allow some statistical variation
        assert!(var_1 / var_100 < 200.0);
    }

    #[test]
    fn test_noise_after_averaging() {
        let noise_1 = 1e-8;
        let noise_100 = noise_after_averaging(noise_1, 100.0);
        
        // Should reduce by factor of 10
        assert!((noise_100 - 1e-9).abs() / 1e-9 < 0.001);
    }

    #[test]
    fn test_dynamic_range_improvement() {
        // 1000 averages should give 15 dB improvement
        let improvement = dynamic_range_improvement(1000.0);
        assert!((improvement - 15.0).abs() < 0.01);
        
        // 100 averages should give 10 dB improvement
        let improvement_100 = dynamic_range_improvement(100.0);
        assert!((improvement_100 - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_averaging_time_calculation() {
        // For 10 dB improvement at 10000 Hz
        let time = averaging_time_for_improvement(10.0, 10000.0);
        
        // 10 dB needs N = 10^(10/5) = 100 averages
        // At 10000 Hz, that's 0.01 s
        assert!((time - 0.01).abs() < 0.001);
    }

    #[test]
    fn test_zero_averages() {
        let base_trace = vec![1.0, 2.0, 3.0];
        let result = simulate_averaging(&base_trace, 0.1, 0, None);
        
        for i in 0..base_trace.len() {
            assert_eq!(result[i], base_trace[i]);
        }
    }
}
