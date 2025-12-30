//! Gaussian noise generation for OTDR traces
//!
//! This module adds realistic detector noise to simulated OTDR traces.
//! The noise is modeled as Gaussian (normal) distributed, which is a good
//! approximation for most OTDR detectors.
//!
//! Noise model:
//! - Baseline noise floor: The minimum detectable signal level (doesn't reduce with averaging)
//! - Variance noise: Random fluctuations that reduce with sqrt(N) averaging
//!
//! After fiber ends, only the noise floor remains visible as a "messy" region.

use rand::SeedableRng;
use rand::Rng;
use rand_chacha::ChaCha8Rng;
use rayon::prelude::*;
use crate::units::{Power, watt, milliwatt};

/// Add Gaussian noise to a trace
/// 
/// The noise model has two components:
/// 1. Baseline noise at noise_floor level (appears where signal is below noise floor)
/// 2. Random variance that's added to the signal (reduced by averaging)
/// 
/// Args:
/// - trace: Input trace
/// - noise_floor: Noise floor power level
/// - noise_std: Standard deviation of variance noise
/// - seed: Optional seed for reproducibility
/// 
/// Returns: Noisy trace
pub fn add_noise_with_floor(
    trace: &[Power], 
    _noise_floor: Power,
    noise_std: Power, 
    seed: Option<u64>
) -> Vec<Power> {
    // Determine chunk size for parallel processing
    // For small traces, don't bother parallelizing or use a large chunk to keep it single-threaded
    let chunk_size = 4096;
    
    trace.par_chunks(chunk_size)
        .enumerate()
        .flat_map(|(chunk_idx, chunk)| {
            // Create seeded or random RNG for this chunk
            let mut rng: ChaCha8Rng = match seed {
                Some(s) => {
                    // Create a deterministic seed for this chunk based on base seed and chunk index
                    // Using a simple mixing strategy
                    let chunk_seed = s.wrapping_add((chunk_idx as u64).wrapping_mul(0x517cc1b727220a95));
                    ChaCha8Rng::seed_from_u64(chunk_seed)
                },
                None => ChaCha8Rng::from_entropy(),
            };
            
            chunk.iter().map(|&power| {
                // Generate random noise
                let noise = generate_gaussian(&mut rng) * noise_std;
                
                // Add noise to signal
                power + noise
            }).collect::<Vec<Power>>()
        })
        .collect()
}

/// Add Gaussian noise to a trace (simple version, for backward compatibility)
pub fn add_noise(trace: &[Power], noise_std: Power, seed: Option<u64>) -> Vec<Power> {
    let chunk_size = 4096;
    
    trace.par_chunks(chunk_size)
        .enumerate()
        .flat_map(|(chunk_idx, chunk)| {
            let mut rng: ChaCha8Rng = match seed {
                Some(s) => {
                    let chunk_seed = s.wrapping_add((chunk_idx as u64).wrapping_mul(0x517cc1b727220a95));
                    ChaCha8Rng::seed_from_u64(chunk_seed)
                },
                None => ChaCha8Rng::from_entropy(),
            };
            
            chunk.iter().map(|&power| {
                let noise = generate_gaussian(&mut rng);
                power + noise * noise_std
            }).collect::<Vec<Power>>()
        })
        .collect()
}

/// Generate a single Gaussian random number using Box-Muller transform
fn generate_gaussian(rng: &mut ChaCha8Rng) -> f64 {
    let u1: f64 = rng.gen_range(0.0001..1.0); // Avoid exactly 0 for ln()
    let u2: f64 = rng.gen_range(0.0..1.0);
    
    (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos()
}

/// Calculate noise floor power from dBm value
pub fn noise_floor_power(noise_floor_dbm: f64) -> Power {
    let val = 10.0_f64.powf((noise_floor_dbm) / 10.0);
    Power::new::<milliwatt>(val)
}

/// Calculate noise standard deviation from noise floor and averaging
/// 
/// Returns: Noise standard deviation
pub fn noise_std_from_floor(noise_floor_dbm: f64, num_averages: f64) -> Power {
    let floor_power = noise_floor_power(noise_floor_dbm);
    
    // Noise variance reduces by sqrt(N) with averaging
    let reduction = num_averages.sqrt();
    
    floor_power / reduction
}

/// Calculate effective number of averages from averaging time
/// 
/// Typical OTDR acquisition rate is ~10,000 traces per second.
pub fn averages_from_time(averaging_time_s: f64, trace_rate_hz: f64) -> f64 {
    averaging_time_s * trace_rate_hz
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_has_correct_statistics() {
        // Generate a large trace with known noise level
        let p_base = Power::new::<watt>(1e-6);
        let trace = vec![p_base; 10000];
        let noise_std = Power::new::<watt>(1e-9);
        
        let noisy = add_noise(&trace, noise_std, Some(42));
        
        // Calculate actual standard deviation of noise
        let sum: f64 = noisy.iter().map(|p| p.get::<watt>()).sum();
        let mean = sum / noisy.len() as f64;
        
        let sum_sq: f64 = noisy.iter()
            .map(|p| (p.get::<watt>() - mean).powi(2))
            .sum();
        let variance = sum_sq / noisy.len() as f64;
        let actual_std = variance.sqrt();
        
        // Should be close to specified noise level (within 5%)
        let target_std = noise_std.get::<watt>();
        assert!((actual_std - target_std).abs() / target_std < 0.05);
    }

    #[test]
    fn test_noise_reproducibility() {
        let p = Power::new::<watt>(1e-6);
        let trace = vec![p; 100];
        let noise_std = Power::new::<watt>(1e-9);
        
        let noisy1 = add_noise(&trace, noise_std, Some(12345));
        let noisy2 = add_noise(&trace, noise_std, Some(12345));
        
        // Same seed should produce same noise
        for i in 0..trace.len() {
            assert_eq!(noisy1[i].get::<watt>(), noisy2[i].get::<watt>());
        }
        
        // Different seed should produce different noise
        let noisy3 = add_noise(&trace, noise_std, Some(54321));
        let mut any_different = false;
        for i in 0..trace.len() {
            if noisy1[i] != noisy3[i] {
                any_different = true;
                break;
            }
        }
        assert!(any_different);
    }

    #[test]
    fn test_noise_floor_power() {
        // -65 dBm should be ~3.16e-10 W
        let floor = noise_floor_power(-65.0);
        let floor_w = floor.get::<watt>();
        assert!((floor_w - 3.16e-10).abs() / floor_w < 0.01);
        
        // -30 dBm = 1 µW
        let floor_30 = noise_floor_power(-30.0);
        let floor_30_w = floor_30.get::<watt>();
        assert!((floor_30_w - 1e-6).abs() / floor_30_w < 0.01);
    }

    #[test]
    fn test_noise_std_from_floor_averaging() {
        // With many averages, noise reduction should scale with sqrt(N)
        let noise_1avg = noise_std_from_floor(-65.0, 1.0);
        let noise_1m_avg = noise_std_from_floor(-65.0, 1_000_000.0);
        
        let ratio = noise_1avg.get::<watt>() / noise_1m_avg.get::<watt>();
        // 1,000,000 averages should give 1000x reduction
        assert!((ratio - 1000.0).abs() < 1e-6);
    }

    #[test]
    fn test_averages_from_time() {
        // 30 seconds at 10,000 Hz = 300,000 averages
        let n = averages_from_time(30.0, 10000.0);
        assert_eq!(n, 300000.0);
    }

    #[test]
    fn test_noise_is_added_to_low_signal() {
        // Trace with very low power
        let small = Power::new::<watt>(1e-15);
        let trace = vec![small; 100];
        let noise_floor = Power::new::<watt>(3.16e-10); // -65 dBm
        let noise_std = noise_floor / 10.0;
        
        let noisy = add_noise_with_floor(&trace, noise_floor, noise_std, Some(42));
        
        // Output should have variance
        let sum_sq_diff: f64 = noisy.iter()
            .zip(trace.iter())
            .map(|(n, t)| (n.get::<watt>() - t.get::<watt>()).powi(2))
            .sum();
            
        // Variance should be roughly noise_std^2
        let variance = sum_sq_diff / trace.len() as f64;
        let expected_var = noise_std.get::<watt>().powi(2);
        
        // Check if variance is reasonable (within factor of 2)
        assert!(variance > expected_var * 0.5);
        assert!(variance < expected_var * 2.0);
    }

    #[test]
    fn test_noise_with_floor_preserves_signal() {
        // Trace with power well above noise floor
        let signal = Power::new::<watt>(1e-6);
        let trace = vec![signal; 100]; // Well above -65 dBm floor
        let noise_floor = Power::new::<watt>(3.16e-10);
        let noise_std = noise_floor / 10.0;
        
        let noisy = add_noise_with_floor(&trace, noise_floor, noise_std, Some(42));
        
        // Mean should still be close to original
        let mean = noisy.iter().map(|p| p.get::<watt>()).sum::<f64>() / noisy.len() as f64;
        assert!((mean - 1e-6).abs() / 1e-6 < 0.1); // Within 10% of original
    }
    #[test]
    fn test_averaging_recovers_signal_below_floor() {
        // Signal at -70 dBm (1e-7 mW)
        let signal_power = Power::new::<milliwatt>(1e-7);
        let trace = vec![signal_power; 1000];
        
        // Noise floor at -65 dBm (3.16e-7 mW)
        // Signal is ~5dB below noise floor
        let noise_floor_dbm = -65.0;
        let noise_floor = noise_floor_power(noise_floor_dbm);
        
        // High averaging: noise_std becomes small
        // Let's say we have enough averages to reduce noise_std to negligible
        // effectively 100x reduction (max limit in code)
        let num_averages = 10000.0;
        let noise_std = noise_std_from_floor(noise_floor_dbm, num_averages);
        
        // With current buggy implementation, the output will be clamped to noise_floor
        // With fix, it should be centered around signal_power
        let noisy = add_noise_with_floor(&trace, noise_floor, noise_std, Some(42));
        
        let mean: f64 = noisy.iter().map(|p| p.get::<milliwatt>()).sum::<f64>() / noisy.len() as f64;
        let target = signal_power.get::<milliwatt>();
        
        println!("Signal: {:.2e}, Floor: {:.2e}, Mean Output: {:.2e}", target, noise_floor.get::<milliwatt>(), mean);
        
        // Check if mean matches signal (within 20% tolerance due to remaining noise)
        // If it's clamped to floor, it will be around 3.16e-7
        // If correct, it should be around 1e-7
        assert!((mean - target).abs() < target * 0.2, 
            "Mean {:.2e} not close to signal {:.2e} (Floor is {:.2e})", mean, target, noise_floor.get::<milliwatt>());
    }
}
