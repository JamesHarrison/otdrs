//! Detector saturation modeling
//!
//! This module models the non-linear response of the OTDR detector when
//! the detected power exceeds the linear range of the detector/electronics.
//!
//! The saturation is modeled using a hyperbolic tangent function:
//! y_sat(z) = P_sat × tanh(y_LTI(z) / P_sat)
//!
//! This ensures that:
//! - For small inputs, y_sat ≈ y_LTI (linear region)
//! - For large inputs, y_sat → P_sat (saturated region)

use crate::units::{Power, Length, milliwatt, meter, watt, ratio, Ratio};

/// Apply detector saturation with recovery tail (hysteresis)
/// 
/// Args:
/// - trace: Input trace (linear power)
/// - saturation_power: Power level at which detector saturates
/// - delta_z: Spatial sampling interval
/// 
/// Returns: Saturated trace
pub fn apply_saturation(
    trace: &[Power],
    saturation_power: Power,
    delta_z: Length,
) -> Vec<Power> {
    if saturation_power.get::<watt>() <= 0.0 {
        return trace.to_vec();
    }
    
    // Recovery time constant (in meters)
    const RECOVERY_TAU_M: f64 = 20.0;
    
    // Coupling coefficient: how much input power contributes to stored charge (per meter)
    // 0.1 means 1m of saturation stores equivalent of 10% of input power
    const COUPLING_COEFF_PER_M: f64 = 0.1;
    
    let dz_m = delta_z.get::<meter>();
    let decay_factor = (-dz_m / RECOVERY_TAU_M).exp();
    let accum_factor = COUPLING_COEFF_PER_M * dz_m;
    
    let mut result = Vec::with_capacity(trace.len());
    let mut stored_charge = Power::new::<watt>(0.0);
    
    for &input_power in trace {
        // Add decayed charge from previous overload to the effective signal seen
        let effective_input = input_power + stored_charge;
        
        // Output calculation
        let output = if effective_input > saturation_power {
            // Saturated region - soft clip
            // Ratio is dimensionless
            let r = effective_input / saturation_power;
            let tanh_val = r.get::<ratio>().tanh();
            let clipped = saturation_power * Ratio::new::<ratio>(tanh_val);
            
            // Accumulate charge from the INPUT power (forcing function)
            // We use input_power rather than excess to avoid runaway feedback loops
            // and scaling by delta_z ensures sample-rate independence.
            stored_charge += input_power * accum_factor;
            
            clipped
        } else {
            // Linear region
            effective_input
        };
        
        result.push(output);
        
        // Decay the stored charge
        stored_charge *= decay_factor;
    }
    
    result
}

/// Calculate the saturation factor at a given power level
/// 
/// This is useful for diagnostics to see how much compression is occurring.
/// 
/// Returns: Saturation factor (output/input), 1.0 = no saturation
pub fn saturation_factor(power: Power, saturation_power: Power) -> f64 {
    if power.get::<watt>() <= 0.0 || saturation_power.get::<watt>() <= 0.0 {
        return 1.0;
    }
    
    let x = power / saturation_power;
    let x_val = x.get::<ratio>();
    x_val.tanh() / x_val
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_saturation_small_signal() {
        // Small signals should pass through nearly unchanged
        let p1 = Power::new::<watt>(1e-9);
        let p2 = Power::new::<watt>(1e-10);
        let p3 = Power::new::<watt>(1e-11);
        
        let trace = vec![p1, p2, p3];
        let sat_pow = Power::new::<milliwatt>(0.1);
        
        let saturated = apply_saturation(&trace, sat_pow, Length::new::<meter>(1.0));
        
        for i in 0..trace.len() {
            let ratio_val = (saturated[i] / trace[i]).get::<ratio>();
            assert!((ratio_val - 1.0).abs() < 0.01);
        }
    }

    #[test]
    fn test_saturation_large_signal() {
        // Large signals should be compressed
        let p1 = Power::new::<milliwatt>(1.0);
        let p2 = Power::new::<milliwatt>(10.0);
        let p3 = Power::new::<milliwatt>(100.0);
        
        let trace = vec![p1, p2, p3];
        let sat_pow = Power::new::<milliwatt>(0.1);
        
        let saturated = apply_saturation(&trace, sat_pow, Length::new::<meter>(1.0));
        
        // All outputs should be near or below saturation level
        for &val in &saturated {
            assert!(val.get::<watt>() <= sat_pow.get::<watt>() * 1.01); // Allow small numerical error
        }
    }

    #[test]
    fn test_saturation_factor() {
        // At P_sat, saturation factor should be tanh(1)/1 ≈ 0.76
        let p_sat = Power::new::<milliwatt>(0.1);
        let factor = saturation_factor(p_sat, p_sat);
        let expected = 1.0_f64.tanh();
        assert!((factor - expected).abs() < 0.01);
        
        // At very low power, factor should be ~1.0
        let p_low = Power::new::<watt>(1e-9);
        let factor_low = saturation_factor(p_low, p_sat);
        assert!((factor_low - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_saturation_preserves_zero_input() {
        let zero = Power::new::<watt>(0.0);
        let small = Power::new::<watt>(1e-6);
        let trace = vec![zero, small, zero, small * 2.0, zero];
        
        let sat_pow = Power::new::<milliwatt>(0.1);
        let saturated = apply_saturation(&trace, sat_pow, Length::new::<meter>(1.0));
        
        assert_eq!(saturated[0].get::<watt>(), 0.0);
        assert!(saturated[2].get::<watt>() >= 0.0);
        assert!(saturated[4].get::<watt>() >= 0.0);
    }

    #[test]
    fn test_saturation_recovery_tail() {
        // Create a pulse that saturates
        let zero = Power::new::<watt>(0.0);
        let pulse_pow = Power::new::<milliwatt>(1000.0); // 1W
        
        let start = vec![zero; 10];
        let pulse = vec![pulse_pow; 10];
        let end = vec![zero; 400];
        
        let mut trace = start;
        trace.extend(pulse);
        trace.extend(end);
        
        let sat_pow = Power::new::<milliwatt>(0.1);
        let delta_z = Length::new::<meter>(1.0);
        
        let saturated = apply_saturation(&trace, sat_pow, delta_z);
        
        // Check saturation region (indices 10-19)
        for i in 10..20 {
            // Should be clipped to approximately p_sat
            let diff = (saturated[i] - sat_pow).get::<milliwatt>().abs();
            assert!(diff < sat_pow.get::<milliwatt>() * 0.1);
        }
        
        // Check tail region (immediately after pulse)
        // Index 20 is the first zero input after the pulse.
        assert!(saturated[20].get::<watt>() > 0.0);
        
        // The tail should decay
        assert!(saturated[30] <= saturated[20]);
        
        // Should eventually drop below saturation level
        let last_val = saturated[saturated.len() - 1];
        assert!(last_val < sat_pow);
        
        // And should be decaying
        assert!(last_val < saturated[30]);
        
        // But still be measurable for a while 
        assert!(saturated[50].get::<watt>() > 1e-12);
    }
}
