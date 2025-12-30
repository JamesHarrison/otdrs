//! Physics simulation engine
//!
//! This module implements the core OTDR physics simulation using the LTI
//! (Linear Time-Invariant) system approach.
//!
//! The fiber network is modeled as a system with impulse response:
//! h_f(z) = f_l(z) × [h_b(z) + h_r(z)]
//! 
//! The output trace is computed as:
//! y(z) = h_f(z) * x(z)

pub mod backscatter;
pub mod reflection;
pub mod loss;
pub mod convolution;
pub mod wavelength;
pub mod signal_processing;

pub use backscatter::compute_backscatter_impulse;
pub use reflection::compute_reflection_impulse;
pub use loss::compute_loss_function;
pub use convolution::{convolve, convolve_same, convolve_power, convolve_same_power};
pub use signal_processing::apply_receiver_bandwidth;

use crate::network::NetworkDescription;
use crate::acquisition::{AcquisitionConfig, WavelengthAcquisition, HardwareConfig};
use crate::acquisition::pulse::generate_pulse;
use crate::units::{Length, Power, Time, Velocity, Frequency, Ratio, SPEED_OF_LIGHT};
use crate::units::{meter, kilometer, milliwatt, nanosecond, ratio, megahertz};

/// Result of a single-wavelength physics simulation
#[derive(Debug, Clone)]
pub struct PhysicsResult {
    /// The raw trace data (optical power)
    pub trace: Vec<Power>,
    /// Spatial sampling interval
    pub delta_z: Length,
    /// Number of data points
    pub num_points: usize,
    /// Wavelength in nm
    pub wavelength_nm: u16,
    /// Pulse width
    pub pulse_width: Time,
}

/// Simulate the OTDR physics for a single wavelength
/// 
/// Returns: PhysicsResult containing the raw trace
pub fn simulate_physics(
    network: &NetworkDescription,
    wavelength_acq: &WavelengthAcquisition,
    hardware: &HardwareConfig,
    pulse_shape: crate::acquisition::PulseShape,
    range: Length,
) -> PhysicsResult {
    // Calculate sampling parameters
    let delta_z = calculate_sampling_interval(range);
    let num_points = (range.get::<meter>() / delta_z.get::<meter>()).ceil() as usize + 1;
    
    let wavelength_nm = wavelength_acq.wavelength_nm;
    
    // Step 1: Compute backscattering impulse response h_b(z)
    let h_b = compute_backscatter_impulse(network, wavelength_nm, delta_z, num_points);
    
    // Step 2: Compute reflection impulse response h_r(z)
    let h_r = compute_reflection_impulse(network, wavelength_nm, delta_z, num_points);
    
    // Step 3: Compute loss function f_l(z)
    let f_l = compute_loss_function(network, wavelength_nm, delta_z, num_points);
    
    // Step 4: Combine impulse responses: h_f(z) = f_l(z) × [h_b(z) + h_r(z)]
    let h_f: Vec<Ratio> = h_b.iter()
        .zip(h_r.iter())
        .zip(f_l.iter())
        .map(|((b, r), l)| *l * (*b + *r))
        .collect();
    
    // Step 5: Generate pulse shape
    let pulse_width = Time::new::<nanosecond>(wavelength_acq.pulse_width_ns);
    let pulse_width_m = calculate_spatial_pulse_width(
        pulse_width,
        hardware.group_index,
    );
    
    let peak_power = Power::new::<milliwatt>(hardware.injection_power_mw);
    let pulse = generate_pulse(
        pulse_shape,
        pulse_width_m,
        delta_z,
        peak_power,
    );
    
    // Step 6: Convolve pulse with impulse response
    // h_f is Ratio (dimensionless), pulse is Power. Result is Power.
    let raw_trace = convolve_same_power(&h_f, &pulse);

    // Step 7: Output raw optical power
    PhysicsResult {
        trace: raw_trace,
        delta_z,
        num_points,
        wavelength_nm,
        pulse_width,
    }
}

/// Calculate spatial sampling interval based on range
fn calculate_sampling_interval(range: Length) -> Length {
    // Empirical formula matching typical OTDR behavior
    let base_sampling = 0.02; // 2cm minimum
    let range_km = range.get::<kilometer>();
    
    // Scale with sqrt of range for reasonable point counts
    let val = (base_sampling * range_km.sqrt()).max(0.02).min(2.0);
    Length::new::<meter>(val)
}

/// Calculate spatial pulse width from temporal width
/// 
/// W = (v_g × τ) / 2
fn calculate_spatial_pulse_width(pulse_width: Time, group_index: f64) -> Length {
    let group_velocity: Velocity = SPEED_OF_LIGHT / group_index;
    (group_velocity * pulse_width) / 2.0
}

/// Simulate physics for all wavelengths in an acquisition config
pub fn simulate_all_wavelengths(
    network: &NetworkDescription,
    acquisition: &AcquisitionConfig,
    hardware: &HardwareConfig,
) -> Vec<PhysicsResult> {
    let range = Length::new::<meter>(acquisition.range_m);
    acquisition.wavelengths.iter()
        .map(|wl_acq| simulate_physics(network, wl_acq, hardware, acquisition.pulse_shape, range))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{FiberSegment, FiberType, NetworkEvent, ConnectorType, ConnectorState, SpliceType};
    use crate::acquisition::WavelengthAcquisition;

    #[test]
    fn test_simple_fiber_simulation() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D));
        
        let wl_acq = WavelengthAcquisition::new(1550, 10.0);
        let hardware = HardwareConfig::default();
        
        let result = simulate_physics(
            &network, 
            &wl_acq, 
            &hardware, 
            crate::acquisition::PulseShape::HyperbolicSecant,
            Length::new::<meter>(2000.0)
        );
        
        assert!(result.num_points > 0);
        assert!(result.delta_z.get::<meter>() > 0.0);
        
        // Trace should have positive values (backscatter power)
        let max_power = result.trace.iter()
            .map(|p| p.get::<milliwatt>())
            .fold(f64::NEG_INFINITY, f64::max);
        assert!(max_power > 0.0);
        
        // Power should generally decay with distance
        let first_tenth: f64 = result.trace[..result.num_points/10].iter()
            .map(|p| p.get::<milliwatt>()).sum();
        let last_tenth: f64 = result.trace[result.num_points*9/10..].iter()
            .map(|p| p.get::<milliwatt>()).sum();
        assert!(first_tenth > last_tenth);
    }

    #[test]
    fn test_reflection_creates_peak() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_event(NetworkEvent::Connector {
                position_m: 500.0,
                insertion_loss_db: 0.3,
                state: ConnectorState::Open,
                connector_type: ConnectorType::UPC,
            });
        
        let wl_acq = WavelengthAcquisition::new(1550, 5.0);
        let hardware = HardwareConfig::default();
        
        let result = simulate_physics(
            &network, 
            &wl_acq, 
            &hardware, 
            crate::acquisition::PulseShape::HyperbolicSecant,
            Length::new::<meter>(2000.0)
        );
        
        // Find the peak around position 500m
        let delta_z_m = result.delta_z.get::<meter>();
        let expected_index = (500.0 / delta_z_m) as usize;
        
        // Look in a window around the expected position
        let window_start = expected_index.saturating_sub(50);
        let window_end = (expected_index + 50).min(result.num_points);
        
        let peak_in_window = result.trace[window_start..window_end]
            .iter()
            .map(|p| p.get::<milliwatt>())
            .fold(f64::NEG_INFINITY, f64::max);
        
        // Peak should be higher than average background
        let total_power: f64 = result.trace.iter().map(|p| p.get::<milliwatt>()).sum();
        let avg_elsewhere = total_power / result.num_points as f64;
        assert!(peak_in_window > avg_elsewhere * 5.0);
    }

    #[test]
    fn test_splice_creates_loss_step() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(2000.0, FiberType::G652D))
            .add_event(NetworkEvent::Splice {
                position_m: 1000.0,
                insertion_loss_db: 0.5,
                splice_type: SpliceType::Fusion,
            });
        
        let wl_acq = WavelengthAcquisition::new(1550, 10.0);
        let hardware = HardwareConfig::default();
        
        let result = simulate_physics(
            &network, 
            &wl_acq, 
            &hardware, 
            crate::acquisition::PulseShape::HyperbolicSecant,
            Length::new::<meter>(3000.0)
        );
        
        let splice_index = (1000.0 / result.delta_z.get::<meter>()) as usize;
        
        // Average power before and after splice
        let avg_before: f64 = result.trace[splice_index.saturating_sub(100)..splice_index]
            .iter().map(|p| p.get::<milliwatt>()).sum::<f64>() / 100.0;
        let avg_after: f64 = result.trace[splice_index + 10..splice_index + 110]
            .iter().map(|p| p.get::<milliwatt>()).sum::<f64>() / 100.0;
        
        // Power after splice should be noticeably lower
        assert!(avg_after < avg_before * 0.95);
    }

    #[test]
    fn test_spatial_pulse_width() {
        // 10ns pulse with n_g = 1.4682 should be about 1.02m
        let time = Time::new::<nanosecond>(10.0);
        let width = calculate_spatial_pulse_width(time, 1.4682);
        assert!((width.get::<meter>() - 1.02).abs() < 0.01);
    }
}
