//! Wavelength-dependent calculations
//!
//! This module provides utilities for wavelength-dependent fiber properties
//! and calculations that vary across the OTDR operating wavelengths.

use crate::network::FiberType;

/// Common OTDR wavelengths in nanometers
pub const WAVELENGTH_1310: u16 = 1310;
pub const WAVELENGTH_1550: u16 = 1550;
pub const WAVELENGTH_1625: u16 = 1625;

/// Calculate the relative Rayleigh scattering intensity at a given wavelength
/// 
/// Rayleigh scattering intensity scales as λ^(-4), so shorter wavelengths
/// have higher scattering (and thus slightly higher backscatter signal).
/// 
/// Args:
/// - wavelength_nm: Operating wavelength in nanometers
/// - reference_nm: Reference wavelength for normalization (default 1550nm)
/// 
/// Returns: Relative scattering intensity factor (1.0 at reference wavelength)
pub fn rayleigh_scattering_factor(wavelength_nm: u16, reference_nm: u16) -> f64 {
    let lambda = wavelength_nm as f64;
    let lambda_ref = reference_nm as f64;
    
    (lambda_ref / lambda).powi(4)
}

/// Calculate wavelength-dependent fiber attenuation using Rayleigh + UV + IR model
/// 
/// This is a simplified model that captures the main wavelength dependencies:
/// - Rayleigh scattering: A/λ^4
/// - UV absorption: B × exp(C/λ)
/// - IR absorption: D × exp(-E/λ)
/// 
/// For most OTDR work in the telecom window, this can be approximated
/// with a linear interpolation between measured values.
/// 
/// Args:
/// - wavelength_nm: Operating wavelength
/// - fiber_type: Type of fiber for base attenuation values
/// 
/// Returns: Attenuation in dB/km
pub fn wavelength_attenuation(wavelength_nm: u16, fiber_type: &FiberType) -> f64 {
    // Use the fiber type's built-in wavelength-dependent characteristics
    fiber_type.characteristics(wavelength_nm).attenuation_db_km
}

/// Calculate mode field diameter at arbitrary wavelength
/// 
/// MFD increases approximately linearly with wavelength in the telecom window.
/// 
/// Args:
/// - wavelength_nm: Operating wavelength
/// - fiber_type: Type of fiber
/// 
/// Returns: Mode field diameter in micrometers
pub fn wavelength_mfd(wavelength_nm: u16, fiber_type: &FiberType) -> f64 {
    fiber_type.characteristics(wavelength_nm).mfd_um
}

/// Calculate splice loss due to MFD mismatch
/// 
/// When two fibers with different MFDs are spliced, there is a loss
/// due to the mode mismatch. This can appear as a gain or loss on
/// the OTDR trace depending on the direction of measurement.
/// 
/// The loss is given by:
/// Loss (dB) = 10 × log10[(MFD1²/MFD2² + MFD2²/MFD1²) / 2]
/// 
/// Args:
/// - mfd1_um: Mode field diameter of first fiber in micrometers
/// - mfd2_um: Mode field diameter of second fiber in micrometers
/// 
/// Returns: Splice loss due to MFD mismatch in dB (always positive)
pub fn mfd_mismatch_loss(mfd1_um: f64, mfd2_um: f64) -> f64 {
    let ratio1 = (mfd1_um / mfd2_um).powi(2);
    let ratio2 = (mfd2_um / mfd1_um).powi(2);
    
    10.0 * ((ratio1 + ratio2) / 2.0).log10()
}

/// Calculate apparent OTDR splice gain/loss due to MFD mismatch
/// 
/// On an OTDR trace, MFD mismatch can appear as a gain when measuring
/// from a fiber with smaller MFD to one with larger MFD (the larger
/// capture fraction compensates for the coupling loss).
/// 
/// Args:
/// - mfd_before_um: MFD of fiber before splice
/// - mfd_after_um: MFD of fiber after splice
/// 
/// Returns: Apparent loss in dB (negative = apparent gain)
pub fn apparent_splice_loss(mfd_before_um: f64, mfd_after_um: f64) -> f64 {
    // The backscatter factor changes with MFD
    // Smaller MFD = higher capture fraction = more backscatter
    2.0 * 10.0 * (mfd_before_um / mfd_after_um).log10()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rayleigh_scattering_at_reference() {
        let factor = rayleigh_scattering_factor(1550, 1550);
        assert!((factor - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_rayleigh_scattering_shorter_wavelength() {
        // Shorter wavelength should have higher scattering
        let factor_1310 = rayleigh_scattering_factor(1310, 1550);
        let factor_1550 = rayleigh_scattering_factor(1550, 1550);
        
        assert!(factor_1310 > factor_1550);
        
        // λ^(-4) scaling: (1550/1310)^4 ≈ 1.96
        let expected = (1550.0_f64 / 1310.0).powi(4);
        assert!((factor_1310 - expected).abs() < 0.01);
    }

    #[test]
    fn test_rayleigh_scattering_longer_wavelength() {
        // Longer wavelength should have lower scattering
        let factor_1625 = rayleigh_scattering_factor(1625, 1550);
        
        assert!(factor_1625 < 1.0);
        
        // λ^(-4) scaling: (1550/1625)^4 ≈ 0.83
        let expected = (1550.0_f64 / 1625.0).powi(4);
        assert!((factor_1625 - expected).abs() < 0.01);
    }

    #[test]
    fn test_mfd_mismatch_loss_identical() {
        // No loss for identical MFDs
        let loss = mfd_mismatch_loss(10.4, 10.4);
        assert!(loss.abs() < 1e-10);
    }

    #[test]
    fn test_mfd_mismatch_loss_symmetric() {
        // Loss should be symmetric
        let loss_ab = mfd_mismatch_loss(9.2, 10.4);
        let loss_ba = mfd_mismatch_loss(10.4, 9.2);
        
        assert!((loss_ab - loss_ba).abs() < 1e-10);
    }

    #[test]
    fn test_mfd_mismatch_loss_typical() {
        // G.652 (10.4µm) to G.657.A1 (9.8µm) splice
        let loss = mfd_mismatch_loss(10.4, 9.8);
        
        // Should be a small positive loss (typically < 0.1 dB)
        assert!(loss > 0.0);
        assert!(loss < 0.2);
    }

    #[test]
    fn test_apparent_splice_gain() {
        // Small MFD to large MFD can show apparent gain
        let apparent = apparent_splice_loss(9.0, 11.0);
        
        // This should be negative (apparent gain)
        assert!(apparent < 0.0);
    }

    #[test]
    fn test_apparent_splice_loss() {
        // Large MFD to small MFD shows apparent loss
        let apparent = apparent_splice_loss(11.0, 9.0);
        
        // This should be positive (apparent loss)
        assert!(apparent > 0.0);
    }
}
