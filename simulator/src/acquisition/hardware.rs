//! Hardware characteristics configuration

use serde::{Deserialize, Serialize};

/// Models the characteristics of the simulated OTDR hardware
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HardwareConfig {
    /// Noise floor in dBm (RMS)
    pub noise_floor_dbm: f64,
    /// Dynamic range in dB
    pub dynamic_range_db: f64,
    /// Detector bandwidth in MHz
    pub detector_bandwidth_mhz: f64,
    /// Saturation power in mW
    pub saturation_power_mw: f64,
    /// Event dead zone in meters (at shortest pulse)
    pub event_dead_zone_m: f64,
    /// Attenuation dead zone in meters (at shortest pulse)
    pub attenuation_dead_zone_m: f64,
    /// Group index of refraction (default ~1.4682 for SMF)
    pub group_index: f64,
    /// Peak injection power in mW
    pub injection_power_mw: f64,
    /// Front panel offset in meters (internal fiber before front panel connector)
    /// Typically 20-100m in real OTDRs
    #[serde(default = "default_front_panel_offset")]
    pub front_panel_offset_m: f64,
}

fn default_front_panel_offset() -> f64 {
    50.0 // Typical value, 50m of internal fiber
}

impl Default for HardwareConfig {
    fn default() -> Self {
        // Reasonable defaults for a mid-range OTDR
        Self {
            noise_floor_dbm: -65.0,
            dynamic_range_db: 40.0,
            detector_bandwidth_mhz: 50.0,
            saturation_power_mw: 0.1,
            event_dead_zone_m: 0.8,
            attenuation_dead_zone_m: 3.0,
            group_index: 1.4682,
            injection_power_mw: 1.0,
            front_panel_offset_m: 50.0,
        }
    }
}

impl HardwareConfig {
    /// Convert noise floor from dBm to Watts
    pub fn noise_floor_watts(&self) -> f64 {
        // dBm to watts: P(W) = 10^((P(dBm)-30)/10)
        10.0_f64.powf((self.noise_floor_dbm - 30.0) / 10.0)
    }

    /// Calculate noise standard deviation based on averaging
    /// Noise reduces by sqrt(N) with N averages
    pub fn noise_std_watts(&self, num_averages: f64) -> f64 {
        self.noise_floor_watts() / num_averages.sqrt()
    }

    /// Get group velocity in m/s
    pub fn group_velocity(&self) -> f64 {
        const SPEED_OF_LIGHT: f64 = 299792458.0; // m/s
        SPEED_OF_LIGHT / self.group_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_noise_floor_conversion() {
        let hw = HardwareConfig::default();
        let noise_w = hw.noise_floor_watts();
        
        // -65 dBm should be about 3.16e-10 W
        assert!((noise_w - 3.16e-10).abs() / noise_w < 0.01);
    }

    #[test]
    fn test_noise_reduction_with_averaging() {
        let hw = HardwareConfig::default();
        let noise_1 = hw.noise_std_watts(1.0);
        let noise_100 = hw.noise_std_watts(100.0);
        
        // Noise should reduce by factor of 10 with 100 averages
        assert!((noise_1 / noise_100 - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_group_velocity() {
        let hw = HardwareConfig::default();
        let vg = hw.group_velocity();
        
        // Should be about 2.04e8 m/s for n_g = 1.4682
        assert!((vg - 2.04e8).abs() / vg < 0.01);
    }
}
