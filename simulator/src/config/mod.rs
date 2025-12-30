//! Configuration module for YAML parsing and simulation config

pub mod yaml;

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::{NetworkDescription, AcquisitionConfig};
use crate::acquisition::HardwareConfig;

/// Complete simulation configuration
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimulationConfig {
    /// Optional seed for RNG (for reproducibility)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    
    /// Output directory for generated files
    pub output_dir: String,
    
    /// Base filename for outputs (wavelength and extension will be appended)
    pub output_basename: String,
    
    /// Network to simulate
    pub network: NetworkDescription,
    
    /// Acquisition parameters
    pub acquisition: AcquisitionConfig,
    
    /// Hardware characteristics (uses defaults if not specified)
    #[serde(default)]
    pub hardware: HardwareConfig,
}

impl SimulationConfig {
    /// Validate the entire simulation configuration
    pub fn validate(&self) -> crate::Result<()> {
        self.network.validate()?;
        self.acquisition.validate()?;
        Ok(())
    }

    /// Generate a unique ID for this simulation run
    pub fn generate_id(&self) -> Uuid {
        if let Some(seed) = self.seed {
            // Deterministic UUID from seed for reproducibility
            Uuid::from_u128(seed as u128)
        } else {
            Uuid::new_v4()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{NetworkDescription, AcquisitionConfig};
    use crate::acquisition::HardwareConfig;
    use crate::network::{FiberSegment, FiberType, NetworkEvent, EndType};
    use crate::acquisition::{WavelengthAcquisition, PulseShape};

    #[test]
    fn test_simulation_config_validation() {
        let config = SimulationConfig {
            seed: Some(12345),
            output_dir: "./output".to_string(),
            output_basename: "test".to_string(),
            network: NetworkDescription::new()
                .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
                .add_event(NetworkEvent::End {
                    position_m: 1000.0,
                    end_type: EndType::Cleaved,
                }),
            acquisition: AcquisitionConfig::new(5000.0, 30.0, PulseShape::Rectangular)
                .add_wavelength(WavelengthAcquisition::new(1550, 10.0)),
            hardware: HardwareConfig::default(),
        };

        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_deterministic_id_from_seed() {
        let config = SimulationConfig {
            seed: Some(12345),
            output_dir: "./output".to_string(),
            output_basename: "test".to_string(),
            network: NetworkDescription::new()
                .add_segment(FiberSegment::new(1000.0, FiberType::G652D)),
            acquisition: AcquisitionConfig::new(5000.0, 30.0, PulseShape::Rectangular)
                .add_wavelength(WavelengthAcquisition::new(1550, 10.0)),
            hardware: HardwareConfig::default(),
        };

        let id1 = config.generate_id();
        let id2 = config.generate_id();
        
        // Same seed should produce same ID
        assert_eq!(id1, id2);
    }
}
