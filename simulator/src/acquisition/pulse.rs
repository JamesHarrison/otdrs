//! Pulse shape generation
//!
//! This module provides functions to generate different OTDR pulse shapes.

use super::params::PulseShape;
use crate::units::{Length, Power, Time, Velocity, Ratio, SPEED_OF_LIGHT};
use crate::units::{meter, milliwatt, nanosecond, ratio};

/// Generate a pulse shape as a vector of power values
pub fn generate_pulse(
    pulse_shape: PulseShape,
    pulse_width: Length,
    delta_z: Length,
    peak_power: Power,
) -> Vec<Power> {
    match pulse_shape {
        PulseShape::Rectangular => generate_rectangular(pulse_width, delta_z, peak_power),
        PulseShape::HyperbolicSecant => generate_hyperbolic_secant(pulse_width, delta_z, peak_power),
        PulseShape::RealRectangular { rise_time_ns, fall_time_ns } => {
            generate_real_rectangular(
                pulse_width,
                delta_z,
                peak_power,
                Time::new::<nanosecond>(rise_time_ns.unwrap_or(1.0)),
                Time::new::<nanosecond>(fall_time_ns.unwrap_or(1.0))
            )
        }
    }
}

/// Generate an ideal rectangular pulse
fn generate_rectangular(pulse_width: Length, delta_z: Length, peak_power: Power) -> Vec<Power> {
    let num_samples = (pulse_width.get::<meter>() / delta_z.get::<meter>()).ceil() as usize;
    vec![peak_power; num_samples.max(1)]
}

/// Generate a hyperbolic secant pulse
/// Shape: P(z) = P0 * sech^2(z / (η*W))
/// where η ensures W is the FWHM
fn generate_hyperbolic_secant(pulse_width: Length, delta_z: Length, peak_power: Power) -> Vec<Power> {
    // η constant for FWHM
    // From paper: η = 2 * ln^(-1)((sqrt(6) + sqrt(32)) / (sqrt(6) - sqrt(32)))
    // Simplified: η ≈ 0.5672
    const ETA: f64 = 0.5672;

    // Generate pulse centered at 0, extending +/- 3*pulse_width
    let extent = 3.0 * pulse_width;
    let num_samples = ((2.0 * extent.get::<meter>()) / delta_z.get::<meter>()).ceil() as usize;
    let mut pulse = Vec::with_capacity(num_samples);

    for i in 0..num_samples {
        let z = -extent + delta_z * (i as f64);
        let x = z / (ETA * pulse_width); // Result is Ratio
        let sech = 1.0 / x.get::<ratio>().cosh();
        pulse.push(peak_power * sech * sech);
    }

    pulse
}

/// Generate a rectangular pulse with realistic smooth edges (tanh)
fn generate_real_rectangular(
    pulse_width: Length,
    delta_z: Length,
    peak_power: Power,
    rise_time: Time,
    fall_time: Time,
) -> Vec<Power> {
    // Convert rise/fall times to spatial lengths
    const GROUP_INDEX: f64 = 1.4682; // typical for SMF
    let group_velocity: Velocity = SPEED_OF_LIGHT / GROUP_INDEX;
    
    // Convert time to distance (single pass)
    // For Tanh: Rise time (10-90%) is approx 2.2 * tau (where tau is the denominator)
    // We want the user-specified time to be the 10-90% rise time.
    let rise_tau: Length = (rise_time * group_velocity) / 2.2;
    let fall_tau: Length = (fall_time * group_velocity) / 2.2;
    
    // We want the FWHM to be approximately pulse_width
    // The pulse is effectively centered at 0 in relative coords, 
    // extending from -width/2 to +width/2
    // Let's model it as a product of two sigmoids:
    // P(z) = P0 * 0.5*(1 + tanh((z + W/2)/tau_r)) * 0.5*(1 - tanh((z - W/2)/tau_f))
    
    // Calculate simulation extent
    // Ensure we capture the tails. 3*rise_time corresponds to ~99.5% transition
    let start_padding = 5.0 * rise_tau;
    let end_padding = 5.0 * fall_tau;
    let total_simulation_width = pulse_width + start_padding + end_padding;
    
    let num_samples = (total_simulation_width.get::<meter>() / delta_z.get::<meter>()).ceil() as usize;
    let mut pulse = Vec::with_capacity(num_samples);
    
    let half_width = pulse_width / 2.0;
    
    // Start z such that at sample 0 we are well before the pulse
    let z_start = -(half_width + start_padding);
    
    for i in 0..num_samples {
        let z = z_start + delta_z * (i as f64);
        
        let rise_arg = (z + half_width) / rise_tau;
        let fall_arg = (z - half_width) / fall_tau;
        
        let rise_factor = 0.5 * (1.0 + rise_arg.get::<ratio>().tanh());
        let fall_factor = 0.5 * (1.0 - fall_arg.get::<ratio>().tanh());
        
        pulse.push(peak_power * rise_factor * fall_factor);
    }
    
    pulse
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::units::{meter, milliwatt, nanosecond};

    #[test]
    fn test_rectangular_pulse() {
        let width = Length::new::<meter>(1.0);
        let dz = Length::new::<meter>(0.1);
        let power = Power::new::<milliwatt>(1.0);
        
        let pulse = generate_rectangular(width, dz, power);
        
        // Should have 10 samples
        assert_eq!(pulse.len(), 10);
        
        // All samples should be at peak power
        for &p in &pulse {
            assert!((p.get::<milliwatt>() - 1.0).abs() < 1e-6);
        }
    }

    #[test]
    fn test_hyperbolic_secant_pulse() {
        let width = Length::new::<meter>(1.0);
        let dz = Length::new::<meter>(0.05);
        let power = Power::new::<milliwatt>(1.0);
        
        let pulse = generate_hyperbolic_secant(width, dz, power);
        
        // Pulse should be non-empty
        assert!(!pulse.is_empty());
        
        // Peak should be near 1.0 mW at center
        let max_power = pulse.iter()
            .map(|p| p.get::<milliwatt>())
            .fold(f64::NEG_INFINITY, f64::max);
            
        assert!((max_power - 1.0).abs() < 0.01);
        
        // Pulse should decay towards edges
        assert!(pulse[0].get::<milliwatt>() < 0.5);
        assert!(pulse[pulse.len() - 1].get::<milliwatt>() < 0.5);
    }

    #[test]
    fn test_real_rectangular_pulse() {
        let width = Length::new::<meter>(1.0);
        let dz = Length::new::<meter>(0.05);
        let power = Power::new::<milliwatt>(1.0);
        let rise = Time::new::<nanosecond>(1.0);
        let fall = Time::new::<nanosecond>(1.0);

        let pulse = generate_real_rectangular(width, dz, power, rise, fall);
        
        // Should have rise, plateau, and fall regions
        assert!(pulse.len() > 20); // At least some samples
        
        // First sample should be low
        assert!(pulse[0].get::<milliwatt>() < 0.1);
        
        // Middle samples should be at peak
        let mid = pulse.len() / 2;
        assert!((pulse[mid].get::<milliwatt>() - 1.0).abs() < 0.1);
        
        // Last sample should be low
        assert!(pulse[pulse.len() - 1].get::<milliwatt>() < 0.1);
    }
}
