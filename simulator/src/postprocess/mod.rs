//! Post-processing chain for OTDR traces
//!
//! This module implements the post-processing operations applied after
//! the physics simulation to produce realistic OTDR traces:
//!
//! 1. Detector saturation (non-linear compression at high power)
//! 2. Detector bandwidth filtering (spatial resolution limits)
//! 3. Gaussian noise addition (thermal/shot noise)
//! 4. Power clipping (removing values below noise floor or above range)
//!
//! The processing order follows a real OTDR acquisition chain.

pub mod saturation;
pub mod noise;
pub mod filter; // Still included but we prefer physics::signal_processing for bandwidth
pub mod averaging;

pub use saturation::apply_saturation;
pub use noise::{add_noise, add_noise_with_floor, noise_std_from_floor, noise_floor_power, averages_from_time};
pub use filter::{apply_bandwidth_filter, spatial_resolution};
pub use averaging::{simulate_averaging, noise_after_averaging, dynamic_range_improvement};

use crate::acquisition::HardwareConfig;
use crate::units::{Power, Length, Time, Frequency, Ratio};
use crate::units::{milliwatt, watt, meter, second, megahertz, hertz};
use crate::physics::signal_processing::apply_receiver_bandwidth;

/// Configuration for the post-processing chain
#[derive(Debug, Clone)]
pub struct PostProcessConfig {
    /// Enable detector saturation modeling
    pub enable_saturation: bool,
    /// Enable bandwidth filtering
    pub enable_bandwidth_filter: bool,
    /// Enable noise addition
    pub enable_noise: bool,
    /// Enable power clipping
    pub enable_clipping: bool,
    /// Random seed for noise (None for random)
    pub seed: Option<u64>,
}

impl Default for PostProcessConfig {
    fn default() -> Self {
        Self {
            enable_saturation: true,
            enable_bandwidth_filter: true,
            enable_noise: true,
            enable_clipping: true,
            seed: None,
        }
    }
}

impl PostProcessConfig {
    /// Create config with reproducible noise
    pub fn with_seed(mut self, seed: u64) -> Self {
        self.seed = Some(seed);
        self
    }
    
    /// Create config without noise (for debugging)
    pub fn no_noise(mut self) -> Self {
        self.enable_noise = false;
        self
    }
}

/// Apply the complete post-processing chain to a physics trace
/// 
/// Args:
/// - trace: Raw physics trace (optical power)
/// - hardware: Hardware configuration
/// - delta_z: Spatial sampling interval
/// - averaging_time: Averaging time
/// - config: Post-processing configuration
/// 
/// Returns: Processed trace
pub fn apply_postprocessing(
    trace: &[Power],
    hardware: &HardwareConfig,
    delta_z: Length,
    averaging_time: Time,
    config: &PostProcessConfig,
) -> Vec<Power> {
    let mut result = trace.to_vec();
    
    // Step 1: Detector saturation
    if config.enable_saturation {
        let saturation_power = Power::new::<milliwatt>(hardware.saturation_power_mw);
        result = apply_saturation(&result, saturation_power, delta_z);
    }
    
    // Step 2: Detector bandwidth filtering
    if config.enable_bandwidth_filter {
        // Use the physics-based receiver model (TIA tail)
        let bandwidth = Frequency::new::<megahertz>(hardware.detector_bandwidth_mhz);
        result = apply_receiver_bandwidth(
            &result,
            bandwidth,
            delta_z,
            hardware.group_index,
        );
    }
    
    // Step 3: Noise addition
    if config.enable_noise {
        // Calculate effective number of averages
        const TRACE_RATE_HZ: f64 = 10000.0; // Typical OTDR acquisition rate
        let num_averages = averages_from_time(averaging_time.get::<second>(), TRACE_RATE_HZ);
        
        // Calculate noise parameters
        let floor_power = noise_floor_power(hardware.noise_floor_dbm);
        let noise_std = noise_std_from_floor(hardware.noise_floor_dbm, num_averages);
        
        // Use noise model with floor to create realistic noise region after fiber end
        result = add_noise_with_floor(&result, floor_power, noise_std, config.seed);
    }
    
    // Step 4: Power clipping
    if config.enable_clipping {
        result = apply_clipping(&result, hardware);
    }
    
    result
}

/// Apply power clipping to remove values outside detector range
/// 
/// Clips values:
/// - Below zero (can happen due to noise)
/// - Above saturation level (already handled by saturation model but clip anyway)
fn apply_clipping(trace: &[Power], hardware: &HardwareConfig) -> Vec<Power> {
    let max_power = Power::new::<milliwatt>(hardware.saturation_power_mw);
    let zero = Power::new::<watt>(0.0);
    
    trace.iter()
        .map(|&power| {
            if power < zero {
                zero // Can't have negative optical power
            } else if power > max_power {
                max_power
            } else {
                power
            }
        })
        .collect()
}

/// Convert trace from watts to dBm
pub fn trace_to_dbm(trace: &[Power]) -> Vec<f64> {
    trace.iter()
        .map(|&power| {
            let p_mw = power.get::<milliwatt>();
            if p_mw <= 0.0 {
                f64::NEG_INFINITY
            } else {
                10.0 * p_mw.log10()
            }
        })
        .collect()
}

/// Convert trace from watts to relative dB (for OTDR display)
/// 
/// OTDR traces are typically displayed as relative power in dB,
/// with higher (more positive) values at the start of the fiber.
pub fn trace_to_db_relative(trace: &[Power], reference_power: Power) -> Vec<f64> {
    let ref_w = reference_power.get::<watt>();
    trace.iter()
        .map(|&power| {
            let p_w = power.get::<watt>();
            if p_w <= 0.0 || ref_w <= 0.0 {
                f64::NEG_INFINITY
            } else {
                // OTDR uses 5*log10 due to double-pass (one-way equivalent)
                5.0 * (p_w / ref_w).log10()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_postprocessing_chain() {
        // Create a simple test trace
        let p_base = Power::new::<watt>(1e-6);
        let mut trace = vec![p_base; 1000];
        // Add a reflection peak
        trace[500] = Power::new::<watt>(1e-4);
        
        let hardware = HardwareConfig::default();
        let config = PostProcessConfig::default().with_seed(42);
        
        // 1m sampling, 30s averaging
        let result = apply_postprocessing(
            &trace, 
            &hardware, 
            Length::new::<meter>(0.1), 
            Time::new::<second>(30.0), 
            &config
        );
        
        // Result should have same length
        assert_eq!(result.len(), trace.len());
        
        // All values should be non-negative (due to clipping)
        for &val in &result {
            assert!(val.get::<watt>() >= 0.0);
        }
        
        // Peak should still be visible (higher than surroundings)
        let peak_region: f64 = result[490..510].iter().map(|p| p.get::<watt>()).sum::<f64>() / 20.0;
        let baseline: f64 = result[100..200].iter().map(|p| p.get::<watt>()).sum::<f64>() / 100.0;
        assert!(peak_region > baseline);
    }

    #[test]
    fn test_postprocessing_no_noise() {
        let p = Power::new::<watt>(1e-6);
        let trace = vec![p; 100];
        let hardware = HardwareConfig::default();
        let config = PostProcessConfig::default().no_noise();
        
        let result1 = apply_postprocessing(
            &trace, &hardware, Length::new::<meter>(0.1), Time::new::<second>(30.0), &config
        );
        let result2 = apply_postprocessing(
            &trace, &hardware, Length::new::<meter>(0.1), Time::new::<second>(30.0), &config
        );
        
        // Without noise, results should be identical
        for i in 0..trace.len() {
            assert_eq!(result1[i].get::<watt>(), result2[i].get::<watt>());
        }
    }

    #[test]
    fn test_clipping() {
        let zero = Power::new::<watt>(0.0);
        let neg = Power::new::<watt>(-1e-6);
        let small = Power::new::<watt>(1e-6);
        let large = Power::new::<watt>(1.0); // 1W > default saturation
        
        let trace = vec![neg, zero, small, large];
        // Contains negative and very large
        let hardware = HardwareConfig::default();
        
        let clipped = apply_clipping(&trace, &hardware);
        
        // Negative should become zero
        assert_eq!(clipped[0].get::<watt>(), 0.0);
        
        // Zero stays zero
        assert_eq!(clipped[1].get::<watt>(), 0.0);
        
        // Normal value unchanged  
        assert!((clipped[2].get::<watt>() - 1e-6).abs() < 1e-12);
        
        // Large value clipped to saturation
        assert!(clipped[3].get::<milliwatt>() <= hardware.saturation_power_mw);
    }

    #[test]
    fn test_trace_to_dbm() {
        let p0 = Power::new::<milliwatt>(1.0); // 0 dBm
        let p1 = Power::new::<milliwatt>(0.001); // -30 dBm
        let p2 = Power::new::<milliwatt>(1e-6); // -60 dBm
        
        let trace = vec![p0, p1, p2];
        let dbm = trace_to_dbm(&trace);
        
        assert!((dbm[0] - 0.0).abs() < 0.01);
        assert!((dbm[1] - (-30.0)).abs() < 0.01);
        assert!((dbm[2] - (-60.0)).abs() < 0.01);
    }

    #[test]
    fn test_trace_to_db_relative() {
        let p0 = Power::new::<milliwatt>(1.0);
        let p1 = Power::new::<milliwatt>(0.1);
        let p2 = Power::new::<milliwatt>(0.01);
        
        let trace = vec![p0, p1, p2];
        let db = trace_to_db_relative(&trace, p0);
        
        // Reference = 0 dB
        assert!((db[0] - 0.0).abs() < 0.01);
        
        // 0.1 ratio = -5 dB (using 5×log10 for OTDR)
        assert!((db[1] - (-5.0)).abs() < 0.01);
        
        // 0.01 ratio = -10 dB
        assert!((db[2] - (-10.0)).abs() < 0.01);
    }
}
