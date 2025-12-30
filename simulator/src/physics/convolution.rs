//! FFT-based convolution for OTDR trace generation
//!
//! This module provides efficient convolution of the fiber impulse response
//! with the OTDR pulse shape using FFT (Fast Fourier Transform).
//!
//! The output trace is computed as:
//! y(z) = h_f(z) * x(z)
//!
//! where * denotes convolution, h_f is the fiber impulse response, and
//! x is the pulse shape.

use rustfft::{FftPlanner, num_complex::Complex};
use crate::units::{Ratio, Power, ratio, milliwatt};

/// Typed wrapper for convolution of dimensionless impulse response with power pulse
pub fn convolve_power(signal: &[Ratio], kernel: &[Power]) -> Vec<Power> {
    let signal_f64: Vec<f64> = signal.iter().map(|x| x.get::<ratio>()).collect();
    let kernel_f64: Vec<f64> = kernel.iter().map(|x| x.get::<milliwatt>()).collect();
    
    let result_f64 = convolve(&signal_f64, &kernel_f64);
    
    result_f64.into_iter().map(|x| Power::new::<milliwatt>(x)).collect()
}

/// Typed wrapper for convolve_same
pub fn convolve_same_power(signal: &[Ratio], kernel: &[Power]) -> Vec<Power> {
    let signal_f64: Vec<f64> = signal.iter().map(|x| x.get::<ratio>()).collect();
    let kernel_f64: Vec<f64> = kernel.iter().map(|x| x.get::<milliwatt>()).collect();
    
    let result_f64 = convolve_same(&signal_f64, &kernel_f64);
    
    result_f64.into_iter().map(|x| Power::new::<milliwatt>(x)).collect()
}

/// Perform FFT-based convolution of two signals
/// 
/// Uses the convolution theorem: conv(a, b) = IFFT(FFT(a) × FFT(b))
/// 
/// Args:
/// - signal: The primary signal (impulse response)
/// - kernel: The kernel to convolve with (pulse shape)
/// 
/// Returns: Convolved output signal
pub fn convolve(signal: &[f64], kernel: &[f64]) -> Vec<f64> {
    if signal.is_empty() || kernel.is_empty() {
        return Vec::new();
    }
    
    // Output length for linear convolution
    let output_len = signal.len() + kernel.len() - 1;
    
    // Find next power of 2 for efficient FFT
    let fft_len = output_len.next_power_of_two();
    
    // Zero-pad inputs to FFT length
    let mut signal_padded: Vec<Complex<f64>> = signal
        .iter()
        .map(|&x| Complex::new(x, 0.0))
        .collect();
    signal_padded.resize(fft_len, Complex::new(0.0, 0.0));
    
    let mut kernel_padded: Vec<Complex<f64>> = kernel
        .iter()
        .map(|&x| Complex::new(x, 0.0))
        .collect();
    kernel_padded.resize(fft_len, Complex::new(0.0, 0.0));
    
    // Create FFT planners
    let mut planner = FftPlanner::new();
    let fft = planner.plan_fft_forward(fft_len);
    let ifft = planner.plan_fft_inverse(fft_len);
    
    // Forward FFT on both signals
    fft.process(&mut signal_padded);
    fft.process(&mut kernel_padded);
    
    // Element-wise multiplication in frequency domain
    let mut product: Vec<Complex<f64>> = signal_padded
        .iter()
        .zip(kernel_padded.iter())
        .map(|(a, b)| a * b)
        .collect();
    
    // Inverse FFT
    ifft.process(&mut product);
    
    // Normalize and extract real part
    let norm = 1.0 / fft_len as f64;
    product[..output_len]
        .iter()
        .map(|c| c.re * norm)
        .collect()
}

/// Perform convolution and return result truncated to input signal length
/// 
/// This is useful when you want the output to match the input trace length,
/// which is the typical case for OTDR simulation.
/// 
/// The output is aligned so that the pulse center corresponds to the
/// position in the impulse response (causal relationship).
pub fn convolve_same(signal: &[f64], kernel: &[f64]) -> Vec<f64> {
    let full_conv = convolve(signal, kernel);
    
    if full_conv.is_empty() || kernel.is_empty() {
        return signal.to_vec();
    }
    
    // Calculate offset to center the result
    // For a kernel of length K centered at K/2, we want output[i] to correspond
    // to input[i] convolved with the kernel centered at that position
    let kernel_center = kernel.len() / 2;
    
    // Extract the portion that aligns with the input
    let start = kernel_center;
    let end = (start + signal.len()).min(full_conv.len());
    
    if start >= full_conv.len() {
        return vec![0.0; signal.len()];
    }
    
    let mut result: Vec<f64> = full_conv[start..end].to_vec();
    
    // Pad with zeros if needed
    result.resize(signal.len(), 0.0);
    
    result
}

/// Perform convolution returning the "valid" portion only
/// 
/// This returns only the portion where the kernel fully overlaps with the signal.
pub fn convolve_valid(signal: &[f64], kernel: &[f64]) -> Vec<f64> {
    let full_conv = convolve(signal, kernel);
    
    if full_conv.len() < kernel.len() {
        return Vec::new();
    }
    
    let valid_len = signal.len().saturating_sub(kernel.len()) + 1;
    let start = kernel.len() - 1;
    let end = (start + valid_len).min(full_conv.len());
    
    full_conv[start..end].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convolve_with_delta() {
        // Convolving with a delta function should return the original signal
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let delta = vec![1.0];
        
        let result = convolve(&signal, &delta);
        
        for i in 0..signal.len() {
            assert!((result[i] - signal[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_convolve_with_shift() {
        // Convolving with [0, 1] should shift signal by one position
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let shift = vec![0.0, 1.0];
        
        let result = convolve(&signal, &shift);
        
        // Result should be [0, 1, 2, 3, 4, 5]
        assert!((result[0] - 0.0).abs() < 1e-10);
        assert!((result[1] - 1.0).abs() < 1e-10);
        assert!((result[2] - 2.0).abs() < 1e-10);
        assert!((result[3] - 3.0).abs() < 1e-10);
        assert!((result[4] - 4.0).abs() < 1e-10);
        assert!((result[5] - 5.0).abs() < 1e-10);
    }

    #[test]
    fn test_convolve_rectangular_pulse() {
        // Convolving with rectangular pulse should produce a smoothed version
        let signal = vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0];
        let kernel = vec![1.0, 1.0, 1.0]; // Rectangular pulse
        
        let result = convolve(&signal, &kernel);
        
        // Delta at position 3 convolved with 3-point rect gives [0,0,0,1,1,1,0,0,0]
        assert!((result[3] - 1.0).abs() < 1e-10);
        assert!((result[4] - 1.0).abs() < 1e-10);
        assert!((result[5] - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_convolve_same_size() {
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        let kernel = vec![0.25, 0.5, 0.25];
        
        let result = convolve_same(&signal, &kernel);
        
        // Result should be same length as input
        assert_eq!(result.len(), signal.len());
        
        // Middle values should be weighted average
        // At position 5: 0.25*5 + 0.5*6 + 0.25*7 = 1.25 + 3 + 1.75 = 6
        assert!((result[5] - 6.0).abs() < 1e-10);
    }

    #[test]
    fn test_convolve_empty() {
        let empty: Vec<f64> = vec![];
        let signal = vec![1.0, 2.0, 3.0];
        
        assert!(convolve(&empty, &signal).is_empty());
        assert!(convolve(&signal, &empty).is_empty());
    }

    #[test]
    fn test_convolve_associativity_scale() {
        // conv(a, k) where k sums to S should have output sum = input sum × S
        let signal = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let kernel = vec![0.5, 1.0, 0.5]; // sums to 2.0
        
        let result = convolve(&signal, &kernel);
        
        let input_sum: f64 = signal.iter().sum();
        let output_sum: f64 = result.iter().sum();
        
        assert!((output_sum - input_sum * 2.0).abs() < 1e-10);
    }

    #[test]
    fn test_convolve_commutative() {
        let a = vec![1.0, 2.0, 3.0, 4.0];
        let b = vec![0.5, 1.0, 0.5];
        
        let result1 = convolve(&a, &b);
        let result2 = convolve(&b, &a);
        
        for i in 0..result1.len() {
            assert!((result1[i] - result2[i]).abs() < 1e-10);
        }
    }

    #[test]
    fn test_convolve_large() {
        // Test with larger arrays to ensure FFT efficiency
        let signal: Vec<f64> = (0..10000).map(|i| (i as f64 * 0.001).sin()).collect();
        let kernel: Vec<f64> = (0..100).map(|_| 0.01).collect();
        
        let result = convolve(&signal, &kernel);
        
        // Should complete and produce reasonable output
        assert_eq!(result.len(), signal.len() + kernel.len() - 1);
        
        // Result should be smoothed version (moving average)
        // Check that it's bounded reasonably
        for &val in &result {
            assert!(val.abs() < 2.0);
        }
    }
}
