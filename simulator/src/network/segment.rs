//! Fiber segment definitions

use serde::{Deserialize, Serialize};
use super::fiber_types::FiberType;

/// A single fiber segment with uniform properties
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FiberSegment {
    /// Length of this segment in meters
    pub length_m: f64,
    /// Fiber type (determines wavelength-dependent parameters)
    pub fiber_type: FiberType,
    /// Optional override for attenuation coefficient (dB/km)
    /// If None, uses fiber_type default for each wavelength
    pub attenuation_override: Option<f64>,
}

impl FiberSegment {
    /// Create a new fiber segment
    pub fn new(length_m: f64, fiber_type: FiberType) -> Self {
        Self {
            length_m,
            fiber_type,
            attenuation_override: None,
        }
    }

    /// Create a segment with custom attenuation
    pub fn with_attenuation(mut self, attenuation_db_km: f64) -> Self {
        self.attenuation_override = Some(attenuation_db_km);
        self
    }

    /// Get effective attenuation for this segment at given wavelength
    pub fn attenuation_db_km(&self, wavelength_nm: u16) -> f64 {
        self.attenuation_override
            .unwrap_or_else(|| self.fiber_type.characteristics(wavelength_nm).attenuation_db_km)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_segment_default_attenuation() {
        let segment = FiberSegment::new(1000.0, FiberType::G652D);
        let atten = segment.attenuation_db_km(1550);
        assert!((atten - 0.19).abs() < 0.01);
    }

    #[test]
    fn test_segment_override_attenuation() {
        let segment = FiberSegment::new(1000.0, FiberType::G652D)
            .with_attenuation(0.30);
        assert_eq!(segment.attenuation_db_km(1550), 0.30);
    }
}
