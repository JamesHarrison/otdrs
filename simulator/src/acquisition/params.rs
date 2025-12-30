//! Acquisition parameter definitions

use serde::{Deserialize, Serialize};
use crate::SimulatorError;

/// Parameters for a single wavelength acquisition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WavelengthAcquisition {
    /// Wavelength in nanometers
    pub wavelength_nm: u16,
    /// Pulse width in nanoseconds
    pub pulse_width_ns: f64,
}

impl WavelengthAcquisition {
    pub fn new(wavelength_nm: u16, pulse_width_ns: f64) -> Self {
        Self {
            wavelength_nm,
            pulse_width_ns,
        }
    }
}

/// Pulse shape options
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PulseShape {
    /// Ideal rectangular pulse
    Rectangular,
    /// Hyperbolic secant (mode-locked laser approximation)
    HyperbolicSecant,
    /// Rectangular with finite rise/fall times
    RealRectangular {
        #[serde(default)]
        rise_time_ns: Option<f64>,
        #[serde(default)]
        fall_time_ns: Option<f64>,
    },
}

/// Optional launch cable configuration
/// 
/// A launch cable is a test fiber cable connected between the OTDR front panel
/// and the fiber under test. It's used to:
/// 1. Remove the front panel connector from measurements (it would otherwise cause dead zone)
/// 2. Allow measurement of the first connector on the fiber under test
/// 
/// The launch cable creates the "user offset" in the measurement.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LaunchCable {
    /// Length of the launch cable in meters
    /// Typical values: 50-500m for short range, 1000-2000m for long range
    pub length_m: f64,
    /// Connector type at the end of the launch cable (connects to fiber under test)
    /// The front panel end has a connector too but it's inside the dead zone
    #[serde(default)]
    pub connector_type: LaunchConnectorType,
    /// Insertion loss of the launch cable connector in dB
    #[serde(default = "default_launch_connector_loss")]
    pub connector_loss_db: f64,
}

fn default_launch_connector_loss() -> f64 {
    0.3 // Typical mated connector loss
}

/// Connector type for launch cable
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum LaunchConnectorType {
    #[default]
    UPC,
    APC,
}

impl LaunchCable {
    /// Create a new launch cable configuration
    pub fn new(length_m: f64) -> Self {
        Self {
            length_m,
            connector_type: LaunchConnectorType::UPC,
            connector_loss_db: 0.3,
        }
    }
    
    /// Set connector type
    pub fn with_connector_type(mut self, connector_type: LaunchConnectorType) -> Self {
        self.connector_type = connector_type;
        self
    }
    
    /// Set connector loss
    pub fn with_connector_loss(mut self, loss_db: f64) -> Self {
        self.connector_loss_db = loss_db;
        self
    }
}

/// Complete acquisition configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcquisitionConfig {
    /// List of wavelengths to simulate (all on same fiber)
    pub wavelengths: Vec<WavelengthAcquisition>,
    /// Acquisition range in meters (determines sampling interval)
    pub range_m: f64,
    /// Averaging time in seconds (affects noise level)
    pub averaging_time_s: f64,
    /// Number of averages (alternative to averaging_time)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_averages: Option<u32>,
    /// Pulse shape to use
    pub pulse_shape: PulseShape,
    /// Optional launch cable between OTDR and fiber under test
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_cable: Option<LaunchCable>,
}

impl AcquisitionConfig {
    /// Create a new acquisition configuration
    pub fn new(range_m: f64, averaging_time_s: f64, pulse_shape: PulseShape) -> Self {
        Self {
            wavelengths: Vec::new(),
            range_m,
            averaging_time_s,
            num_averages: None,
            pulse_shape,
            launch_cable: None,
        }
    }

    /// Add a wavelength acquisition
    pub fn add_wavelength(mut self, wavelength: WavelengthAcquisition) -> Self {
        self.wavelengths.push(wavelength);
        self
    }

    /// Set number of averages
    pub fn with_num_averages(mut self, num: u32) -> Self {
        self.num_averages = Some(num);
        self
    }
    
    /// Set launch cable configuration
    pub fn with_launch_cable(mut self, launch_cable: LaunchCable) -> Self {
        self.launch_cable = Some(launch_cable);
        self
    }

    /// Validate the acquisition configuration
    pub fn validate(&self) -> Result<(), SimulatorError> {
        // Check we have at least one wavelength
        if self.wavelengths.is_empty() {
            return Err(SimulatorError::InvalidConfiguration(
                "Acquisition must have at least one wavelength".to_string()
            ));
        }

        // Check range is positive
        if self.range_m <= 0.0 {
            return Err(SimulatorError::InvalidConfiguration(
                format!("Range must be positive, got {}", self.range_m)
            ));
        }

        // Check averaging time is positive
        if self.averaging_time_s <= 0.0 {
            return Err(SimulatorError::InvalidConfiguration(
                format!("Averaging time must be positive, got {}", self.averaging_time_s)
            ));
        }

        // Validate each wavelength
        for wl in &self.wavelengths {
            if wl.wavelength_nm < 1200 || wl.wavelength_nm > 1700 {
                return Err(SimulatorError::InvalidConfiguration(
                    format!("Wavelength {}nm is outside typical OTDR range (1200-1700nm)",
                            wl.wavelength_nm)
                ));
            }

            if wl.pulse_width_ns <= 0.0 {
                return Err(SimulatorError::InvalidConfiguration(
                    format!("Pulse width must be positive, got {}ns", wl.pulse_width_ns)
                ));
            }

            // Typical OTDR pulse widths range from 3ns to 20000ns
            if wl.pulse_width_ns < 1.0 || wl.pulse_width_ns > 50000.0 {
                return Err(SimulatorError::InvalidConfiguration(
                    format!("Pulse width {}ns is outside typical range (1-50000ns)",
                            wl.pulse_width_ns)
                ));
            }
        }

        Ok(())
    }

    /// Calculate spatial pulse width in meters for a given wavelength acquisition
    /// Using W = (v_g × τ) / 2 where v_g is group velocity
    pub fn spatial_pulse_width_m(&self, wl_acq: &WavelengthAcquisition, group_index: f64) -> f64 {
        const SPEED_OF_LIGHT: f64 = 299792458.0; // m/s
        let group_velocity = SPEED_OF_LIGHT / group_index;
        let pulse_width_s = wl_acq.pulse_width_ns * 1e-9;
        
        // Factor of 2 comes from the double-pass nature of OTDR
        // The displayed width is half the physical propagation width
        (group_velocity * pulse_width_s) / 2.0
    }

    /// Calculate sampling interval based on range
    /// Typical OTDRs use variable sampling based on range setting
    pub fn sampling_interval_m(&self, range_m: f64) -> f64 {
        // Empirical formula: sampling increases with range
        // Typical values: 0.04m for 2km range, 0.08m for 5km, 0.16m for 10km, etc.
        let base_sampling = 0.02; // 2cm at minimum range
        base_sampling * (range_m / 1000.0).sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_acquisition_validation_no_wavelengths() {
        let config = AcquisitionConfig::new(5000.0, 30.0, PulseShape::Rectangular);
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_acquisition_validation_invalid_range() {
        let config = AcquisitionConfig::new(-100.0, 30.0, PulseShape::Rectangular)
            .add_wavelength(WavelengthAcquisition::new(1550, 10.0));
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_acquisition_validation_invalid_wavelength() {
        let config = AcquisitionConfig::new(5000.0, 30.0, PulseShape::Rectangular)
            .add_wavelength(WavelengthAcquisition::new(800, 10.0)); // Too low
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_acquisition_validation_valid() {
        let config = AcquisitionConfig::new(5000.0, 30.0, PulseShape::Rectangular)
            .add_wavelength(WavelengthAcquisition::new(1550, 10.0))
            .add_wavelength(WavelengthAcquisition::new(1310, 10.0));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_spatial_pulse_width() {
        let config = AcquisitionConfig::new(5000.0, 30.0, PulseShape::Rectangular);
        let wl = WavelengthAcquisition::new(1550, 10.0);
        
        let width = config.spatial_pulse_width_m(&wl, 1.4682);
        // 10ns pulse should be about 1.02m spatial width
        assert!((width - 1.02).abs() < 0.01);
    }

    #[test]
    fn test_sampling_interval_scales_with_range() {
        let config = AcquisitionConfig::new(5000.0, 30.0, PulseShape::Rectangular);
        
        let sample_1km = config.sampling_interval_m(1000.0);
        let sample_10km = config.sampling_interval_m(10000.0);
        
        // Sampling should increase with range
        assert!(sample_10km > sample_1km);
    }
}
