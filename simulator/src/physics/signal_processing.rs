//! Signal processing for OTDR physics
//!
//! This module provides functions for signal processing operations
//! needed by the physics engine, such as bandwidth limiting.

use std::f64::consts::PI;
use crate::units::{Power, Length, Frequency, Time, Velocity, SPEED_OF_LIGHT};
use crate::units::{meter, second, hertz, megahertz, milliwatt, ratio};

/// Apply receiver bandwidth limiting to a trace
///
/// This simulates the low-pass filtering effect of the receiver electronics
/// (detector + amplifier + ADC anti-aliasing).
///
/// The filter is modeled as a causal filter (Double Exponential Decay) which provides
/// the characteristic "fast rise, slow tail" response of TIA circuits.
pub fn apply_receiver_bandwidth(
    trace: &[Power],
    bandwidth: Frequency,
    delta_z: Length,
    group_index: f64,
) -> Vec<Power> {
    if trace.is_empty() {
        return Vec::new();
    }

    // bandwidth in Hz
    let bandwidth_hz = bandwidth.get::<hertz>();
    
    // Calculate time constant tau = 1 / (2*pi*f_c)
    // This is for a single-pole low-pass filter (RC circuit approximation)
    let tau_val = 1.0 / (2.0 * PI * bandwidth_hz);
    let tau = Time::new::<second>(tau_val);
    
    // Convert to spatial decay constant
    // L_decay = v_g * tau / 2
    // Factor of 2 accounts for round-trip time scaling in OTDR
    let group_velocity: Velocity = SPEED_OF_LIGHT / group_index;
    let spatial_tau: Length = (group_velocity * tau) / 2.0;

    // Express tau in units of samples
    let tau_samples = spatial_tau.get::<meter>() / delta_z.get::<meter>();
    
    // If tau is very small compared to sampling, filtering has negligible effect
    if tau_samples < 0.1 {
        return trace.to_vec();
    }
    
    // Generate Double Exponential Decay kernel (Causal)
    // Primary component: Bandwidth-limited fast response
    // Secondary component: "Long tail" from TIA/thermal effects (approx 10% amplitude, 5x duration)
    
    let tau_slow_samples = tau_samples * 5.0;
    
    // We need the kernel to cover the tail decay
    // 5 * tau_slow captures >99% of the tail energy
    let length = (5.0 * tau_slow_samples).ceil() as usize;
    let kernel_len = length.max(1);
    let mut kernel = Vec::with_capacity(kernel_len);
    
    let mut sum = 0.0;
    for i in 0..kernel_len {
        let t = i as f64;
        
        // Sum of two exponentials
        let fast_component = (-t / tau_samples).exp();
        let slow_component = 0.1 * (-t / tau_slow_samples).exp();
        
        let val = fast_component + slow_component;
        kernel.push(val);
        sum += val;
    }
    
    // Normalize kernel to preserve total energy (unit gain at DC)
    for val in &mut kernel {
        *val /= sum;
    }
    
    // Convolve
    // We implementing a causal convolution: y[i] depends only on x[j] where j <= i
    
    let zero_power = Power::new::<milliwatt>(0.0);
    let mut output = vec![zero_power; trace.len()];
    
    for i in 0..trace.len() {
        let mut acc = zero_power;
        
        // y[i] = sum_{k=0}^{len-1} h[k] * x[i-k]
        for k in 0..kernel_len {
            if i >= k {
                acc += trace[i - k] * kernel[k];
            }
        }
        
        output[i] = acc;
    }
    
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bandwidth_limiting() {
        // Step function trace
        let zero = Power::new::<milliwatt>(0.0);
        let one = Power::new::<milliwatt>(1.0);
        
        let mut trace = vec![zero; 100];
        for i in 50..100 {
            trace[i] = one;
        }
        
        // Apply filter (bandwidth 50 MHz, typical)
        let applied_bandwidth = Frequency::new::<megahertz>(50.0);
        let delta_z = Length::new::<meter>(0.1);
        
        let filtered = apply_receiver_bandwidth(&trace, applied_bandwidth, delta_z, 1.4682);
        
        // Check causality: No leakage before the step
        assert!(filtered[49].get::<milliwatt>() < 1e-6);
        
        // Check rise behavior
        assert!(filtered[50].get::<milliwatt>() > 0.0);
        assert!(filtered[50].get::<milliwatt>() < 0.5);
        
        // Should approach 1.0 asymptotically
        assert!((filtered[95].get::<milliwatt>() - 1.0).abs() < 0.02);
        
        // Verify monotonicity in the rise region
        assert!(filtered[51] > filtered[50]);
        assert!(filtered[52] > filtered[51]);
    }

    #[test]
    fn test_high_bandwidth_no_change() {
        let zero = Power::new::<milliwatt>(0.0);
        let one = Power::new::<milliwatt>(1.0);
        let trace = vec![one, zero, one, zero];
        
        // Very high bandwidth -> effectively identity
        let filtered = apply_receiver_bandwidth(
            &trace, 
            Frequency::new::<megahertz>(100000.0), // 100 GHz
            Length::new::<meter>(1.0), 
            1.5
        );
        
        for i in 0..trace.len() {
            assert!((filtered[i] - trace[i]).get::<milliwatt>().abs() < 1e-4);
        }
    }
}
