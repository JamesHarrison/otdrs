//! Rayleigh backscattering impulse response calculation
//!
//! This module computes the backscattering impulse response h_b(z) based on
//! the Feigel et al. formulation.

use crate::network::{NetworkDescription, FiberType, FiberSegment};
use crate::units::{Length, Ratio, Time, Velocity, AttenuationCoef, SPEED_OF_LIGHT};
use crate::units::{meter, kilometer, ratio, nanosecond, per_meter, watt, milliwatt};

/// Backscatter coefficient calculation from fiber datasheet value
/// 
/// Returns spatial backscatter coefficient K' in 1/m
pub fn backscatter_coeff_spatial(backscatter_coeff_db: f64, group_index: f64) -> AttenuationCoef {
    // Reference pulse width is 1ns
    let tau_ref = Time::new::<nanosecond>(1.0);
    
    // K_dB = 10 × log10(K × τ_ref)
    // K = 10^(K_dB/10) / τ_ref
    // K is in 1/s (reciprocal time? No, coefficient of scattering per time).
    // Let's do the math in base units.
    let k_temporal_val = 10.0_f64.powf(backscatter_coeff_db / 10.0) / tau_ref.value;
    
    // Group velocity
    let v_g = SPEED_OF_LIGHT / group_index;
    
    // Convert to spatial coefficient (1/m)
    // K' = 2K / v_g (Factor of 2 accounts for double-pass?)
    // Note: The formula often cited is P_bs = P_0 * 0.5 * pulse_width * alpha_s * capture_coeff.
    // Feigel says: h_b(z) = delta_z * K' * ...
    // And K' = (2K)/v_g.
    // Let's stick to the implementation: (2.0 * k_temporal * 1e3) / v_g (previous code).
    // Previous code had 1e3 to convert "1/km"??
    // Previous: `(2.0 * k_temporal * 1e3) / v_g` where v_g in m/s.
    // Result was "1/km".
    // If we want result in 1/m (uom base unit), we should drop the 1e3 if it was for km conversion.
    // WAIT. k_temporal is 1/s. v_g is m/s. (1/s) / (m/s) = 1/m.
    // So `2 * K / v_g` is naturally in 1/m.
    // If previous code multiplied by 1000, it was converting 1/m to 1/km?
    // "Returns: Spatial backscatter coefficient K' in 1/km".
    // Yes.
    // Here we want base units (1/m). So `2 * K / v_g` is correct.
    
    let k_prime_val = (2.0 * k_temporal_val) / v_g.value;
    
    AttenuationCoef::new::<per_meter>(k_prime_val)
}

/// Compute the backscattering impulse response for a fiber network
/// 
/// Returns vector of backscatter coefficients (dimensionless ratios).
pub fn compute_backscatter_impulse(
    network: &NetworkDescription,
    wavelength_nm: u16,
    delta_z: Length,
    num_points: usize,
) -> Vec<Ratio> {
    let mut h_b = vec![Ratio::new::<ratio>(0.0); num_points];
    
    // Build a map of segment boundaries for fast lookup
    let mut segment_boundaries: Vec<(Length, Length, &FiberType)> = Vec::new();
    let mut current_start = Length::new::<meter>(0.0);
    
    for segment in &network.segments {
        let end = current_start + Length::new::<meter>(segment.length_m);
        segment_boundaries.push((current_start, end, &segment.fiber_type));
        current_start = end;
    }
    
    // Calculate h_b(z) for each point
    for i in 0..num_points {
        let z = delta_z * (i as f64);
        
        // Find which segment this point is in
        let (fiber_type, _start) = match find_segment_at(z, &segment_boundaries) {
            Some((ft, start)) => (ft, start),
            None => continue, // Beyond network end
        };
        
        // Get fiber characteristics at this wavelength
        let chars = fiber_type.characteristics(wavelength_nm);
        
        // Calculate cumulative attenuation from start to this point (dB)
        let cumulative_attenuation_db = calculate_cumulative_attenuation(
            z,
            wavelength_nm,
            &segment_boundaries,
        );
        
        // Spatial backscatter coefficient
        let k_prime = backscatter_coeff_spatial(chars.backscatter_coeff_db, chars.group_index);
        
        // h_b(z) = Δz * K' * 10^(-2αz/10)
        // K' is 1/m. Δz is m. Result dimensionless.
        // Factor of 2 in exponent for round trip.
        let power_factor = Ratio::new::<ratio>(10.0_f64.powf(-2.0 * cumulative_attenuation_db / 10.0));
        
        h_b[i] = delta_z * k_prime * power_factor;
    }
    
    h_b
}

/// Find which segment a distance falls into
fn find_segment_at<'a>(
    z: Length,
    boundaries: &'a [(Length, Length, &'a FiberType)],
) -> Option<(&'a FiberType, Length)> {
    for (start, end, fiber_type) in boundaries {
        if z >= *start && z < *end {
            return Some((fiber_type, *start));
        }
    }
    // Check if exactly at the end of the last segment
    if let Some((start, end, fiber_type)) = boundaries.last() {
        if (z - *end).abs() < Length::new::<meter>(1e-9) {
            return Some((fiber_type, *start));
        }
    }
    None
}

/// Calculate cumulative attenuation from start to a given point
fn calculate_cumulative_attenuation(
    z: Length,
    wavelength_nm: u16,
    boundaries: &[(Length, Length, &FiberType)],
) -> f64 {
    let mut total_attenuation_db = 0.0;
    
    for (start, end, fiber_type) in boundaries {
        if z <= *start {
            break;
        }
        
        let chars = fiber_type.characteristics(wavelength_nm);
        let segment_length = if z >= *end {
            *end - *start
        } else {
            z - *start
        };
        
        // attenuation_db_km is dB/km
        total_attenuation_db += chars.attenuation_db_km * segment_length.get::<kilometer>();
    }
    
    total_attenuation_db
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{FiberSegment, NetworkDescription};

    #[test]
    fn test_backscatter_coeff_spatial() {
        // Typical G.652 backscatter coefficient: -79 dB @ 1ns reference
        let k_prime = backscatter_coeff_spatial(-79.0, 1.4682);
        
        // Value should be small positive 1/m
        assert!(k_prime.get::<per_meter>() > 0.0);
        assert!(k_prime.get::<per_meter>() < 1.0);
    }

    #[test]
    fn test_backscatter_impulse_simple_fiber() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D));
        
        let h_b = compute_backscatter_impulse(
            &network, 
            1550, 
            Length::new::<meter>(1.0), 
            1000
        );
        
        assert_eq!(h_b.len(), 1000);
        
        // First point should have highest backscatter
        assert!(h_b[0].get::<ratio>() > 0.0);
        
        // Decay
        assert!(h_b[0] > h_b[500]);
        assert!(h_b[500] > h_b[999]);
    }

    #[test]
    fn test_backscatter_decay_rate() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(10000.0, FiberType::G652D));
        
        let h_b = compute_backscatter_impulse(
            &network, 
            1550, 
            Length::new::<meter>(10.0), 
            1000
        );
        
        // 0.22 dB/km
        // 1km (100 pts @ 10m)
        let ratio_val = h_b[100] / h_b[0];
        let expected = 10.0_f64.powf(-2.0 * 0.22 / 10.0);
        
        assert!((ratio_val.get::<ratio>() - expected).abs() < 0.05);
    }

    #[test]
    fn test_backscatter_wavelength_dependence() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(5000.0, FiberType::G652D));
        
        let h_b_1310 = compute_backscatter_impulse(&network, 1310, Length::new::<meter>(5.0), 1000);
        let h_b_1550 = compute_backscatter_impulse(&network, 1550, Length::new::<meter>(5.0), 1000);
        
        let decay_1310 = h_b_1310[999] / h_b_1310[0];
        let decay_1550 = h_b_1550[999] / h_b_1550[0];
        
        assert!(decay_1310 < decay_1550);
    }

    #[test]
    fn test_backscatter_multi_segment() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(500.0, FiberType::G652D))
            .add_segment(FiberSegment::new(500.0, FiberType::G657A1));
        
        let h_b = compute_backscatter_impulse(&network, 1550, Length::new::<meter>(1.0), 1000);
        
        assert!(h_b[0] > Ratio::new::<ratio>(0.0));
        assert!(h_b[400] > Ratio::new::<ratio>(0.0));
        assert!(h_b[600] > Ratio::new::<ratio>(0.0));
        
        assert!(h_b[0] > h_b[999]);
    }
}
