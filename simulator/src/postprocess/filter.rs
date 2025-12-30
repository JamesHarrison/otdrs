//! Detector bandwidth filtering
//!
//! This module models the finite bandwidth of the OTDR detector/electronics
//! using a low-pass filter. The bandwidth affects:
//! - Spatial resolution (ability to resolve closely spaced events)
//! - Rise/fall times of reflection peaks
//! - Overall noise characteristics
//!
//! The filter is implemented as a Gaussian filter which approximates
//! the typical response of electronic systems.

use std::f64::consts::PI;

/// Apply a Gaussian low-pass filter to simulate detector bandwidth
/// 
/// The filter is characterized by the -3dB bandwidth. A Gaussian filter
/// is used because it provides good pulse response without ringing.
/// 
/// Args:
/// - trace: Input trace in watts
/// - bandwidth_mhz: Detector bandwidth in MHz
/// - delta_z_m: Spatial sampling interval in meters
/// - group_velocity: Group velocity in m/s
/// 
/// Returns: Filtered trace
pub fn apply_bandwidth_filter(
    trace: &[f64],
    bandwidth_mhz: f64,
    delta_z_m: f64,
    group_velocity: f64,
) -> Vec<f64> {
    if trace.is_empty() || bandwidth_mhz <= 0.0 {
        return trace.to_vec();
    }
    
    // Convert bandwidth to spatial domain
    // Temporal bandwidth B_t (Hz) translates to spatial bandwidth B_z (1/m)
    // B_z = B_t / (v_g / 2), factor of 2 for OTDR double-pass
    let bandwidth_hz = bandwidth_mhz * 1e6;
    let spatial_bandwidth = bandwidth_hz / (group_velocity / 2.0);
    
    // Gaussian filter sigma in spatial samples
    // For a Gaussian, the -3dB bandwidth is at f_3dB = 0.8326 / (2π × σ)
    // So σ = 0.8326 / (2π × f_3dB)
    let sigma_m = 0.8326 / (2.0 * PI * spatial_bandwidth);
    let sigma_samples = sigma_m / delta_z_m;
    
    // If sigma is very small, no filtering needed
    if sigma_samples < 0.5 {
        return trace.to_vec();
    }
    
    // Generate Gaussian kernel
    let kernel = generate_gaussian_kernel(sigma_samples);
    
    // Apply filter using convolution
    convolve_1d(trace, &kernel)
}

/// Generate a normalized Gaussian kernel
fn generate_gaussian_kernel(sigma: f64) -> Vec<f64> {
    // Kernel extends to ±3σ (covers 99.7% of the distribution)
    let half_width = (3.0 * sigma).ceil() as usize;
    let width = 2 * half_width + 1;
    
    let mut kernel = vec![0.0; width];
    let mut sum = 0.0;
    
    for i in 0..width {
        let x = (i as f64) - (half_width as f64);
        let val = (-x * x / (2.0 * sigma * sigma)).exp();
        kernel[i] = val;
        sum += val;
    }
    
    // Normalize to sum to 1
    for val in &mut kernel {
        *val /= sum;
    }
    
    kernel
}

/// Simple 1D convolution (using direct method for small kernels)
fn convolve_1d(signal: &[f64], kernel: &[f64]) -> Vec<f64> {
    let n = signal.len();
    let k = kernel.len();
    let half_k = k / 2;
    
    let mut result = vec![0.0; n];
    
    for i in 0..n {
        let mut sum = 0.0;
        for j in 0..k {
            let signal_idx = (i as i64) + (j as i64) - (half_k as i64);
            if signal_idx >= 0 && signal_idx < n as i64 {
                sum += signal[signal_idx as usize] * kernel[j];
            }
        }
        result[i] = sum;
    }
    
    result
}

/// Calculate the spatial resolution (dead zone) from bandwidth
/// 
/// Args:
/// - bandwidth_mhz: Detector bandwidth in MHz
/// - group_velocity: Group velocity in m/s
/// 
/// Returns: Approximate spatial resolution in meters
pub fn spatial_resolution(bandwidth_mhz: f64, group_velocity: f64) -> f64 {
    // Rise time is approximately 0.35 / bandwidth (10-90% rise time)
    // Spatial resolution is then v_g × t_rise / 2
    let bandwidth_hz = bandwidth_mhz * 1e6;
    let rise_time_s = 0.35 / bandwidth_hz;
    
    group_velocity * rise_time_s / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_kernel_normalized() {
        let kernel = generate_gaussian_kernel(3.0);
        let sum: f64 = kernel.iter().sum();
        
        // Should sum to 1.0 (within numerical precision)
        assert!((sum - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_gaussian_kernel_symmetric() {
        let kernel = generate_gaussian_kernel(5.0);
        let n = kernel.len();
        
        for i in 0..n/2 {
            assert!((kernel[i] - kernel[n - 1 - i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_filter_smooths_impulse() {
        // Create an impulse
        let mut trace = vec![0.0; 100];
        trace[50] = 1.0;
        
        // Apply filter
        let group_velocity = 2.04e8; // m/s
        let filtered = apply_bandwidth_filter(&trace, 50.0, 0.1, group_velocity);
        
        // Impulse should spread out
        assert!(filtered[50] < 1.0);
        assert!(filtered[49] > 0.0);
        assert!(filtered[51] > 0.0);
        
        // Total energy should be preserved (approximately)
        let original_sum: f64 = trace.iter().sum();
        let filtered_sum: f64 = filtered.iter().sum();
        assert!((filtered_sum - original_sum).abs() / original_sum < 0.01);
    }

    #[test]
    fn test_filter_preserves_dc() {
        // Constant signal should be unchanged
        let trace = vec![1e-6; 100];
        let group_velocity = 2.04e8;
        let filtered = apply_bandwidth_filter(&trace, 50.0, 0.1, group_velocity);
        
        // Middle values should be unchanged
        for i in 10..90 {
            assert!((filtered[i] - 1e-6).abs() / 1e-6 < 0.01);
        }
    }

    #[test]
    fn test_spatial_resolution() {
        let group_velocity = 2.04e8; // m/s for n_g = 1.47
        
        // 50 MHz bandwidth should give ~0.7m resolution
        let res_50mhz = spatial_resolution(50.0, group_velocity);
        assert!((res_50mhz - 0.71).abs() < 0.1);
        
        // Higher bandwidth = better resolution
        let res_100mhz = spatial_resolution(100.0, group_velocity);
        assert!(res_100mhz < res_50mhz);
    }

    #[test]
    fn test_filter_empty_trace() {
        let trace: Vec<f64> = vec![];
        let filtered = apply_bandwidth_filter(&trace, 50.0, 0.1, 2.04e8);
        assert!(filtered.is_empty());
    }

    #[test]
    fn test_filter_zero_bandwidth() {
        let trace = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let filtered = apply_bandwidth_filter(&trace, 0.0, 0.1, 2.04e8);
        
        // Should return unchanged
        for i in 0..trace.len() {
            assert_eq!(filtered[i], trace[i]);
        }
    }
}
