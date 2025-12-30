//! YAML configuration file parsing

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use crate::SimulatorError;
use super::SimulationConfig;

/// Top-level YAML structure
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct YamlConfig {
    simulation: YamlSimulation,
    network: crate::NetworkDescription,
    acquisition: crate::AcquisitionConfig,
    #[serde(default)]
    hardware: crate::acquisition::HardwareConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct YamlSimulation {
    #[serde(skip_serializing_if = "Option::is_none")]
    seed: Option<u64>,
    output_dir: String,
    output_basename: String,
}

impl SimulationConfig {
    /// Load configuration from a YAML file
    pub fn from_yaml_file<P: AsRef<Path>>(path: P) -> crate::Result<Self> {
        let contents = fs::read_to_string(path.as_ref())
            .map_err(|e| SimulatorError::IoError(e))?;
        
        Self::from_yaml_str(&contents)
    }

    /// Parse configuration from a YAML string
    pub fn from_yaml_str(yaml: &str) -> crate::Result<Self> {
        let yaml_config: YamlConfig = serde_yaml::from_str(yaml)
            .map_err(|e| SimulatorError::InvalidConfiguration(
                format!("YAML parse error: {}", e)
            ))?;

        let config = SimulationConfig {
            seed: yaml_config.simulation.seed,
            output_dir: yaml_config.simulation.output_dir,
            output_basename: yaml_config.simulation.output_basename,
            network: yaml_config.network,
            acquisition: yaml_config.acquisition,
            hardware: yaml_config.hardware,
        };

        // Validate the parsed configuration
        config.validate()?;

        Ok(config)
    }

    /// Write configuration to a YAML file
    pub fn to_yaml_file<P: AsRef<Path>>(&self, path: P) -> crate::Result<()> {
        let yaml_config = YamlConfig {
            simulation: YamlSimulation {
                seed: self.seed,
                output_dir: self.output_dir.clone(),
                output_basename: self.output_basename.clone(),
            },
            network: self.network.clone(),
            acquisition: self.acquisition.clone(),
            hardware: self.hardware.clone(),
        };

        let yaml = serde_yaml::to_string(&yaml_config)
            .map_err(|e| SimulatorError::SerializationError(format!("YAML error: {}", e)))?;

        fs::write(path.as_ref(), yaml)
            .map_err(|e| SimulatorError::IoError(e))?;

        Ok(())
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
    fn test_yaml_roundtrip() {
        let config = SimulationConfig {
            seed: Some(12345),
            output_dir: "./output".to_string(),
            output_basename: "test_fiber".to_string(),
            network: NetworkDescription::new()
                .with_name("Test Network".to_string())
                .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
                .add_event(NetworkEvent::End {
                    position_m: 1000.0,
                    end_type: EndType::Cleaved,
                }),
            acquisition: AcquisitionConfig::new(5000.0, 30.0, PulseShape::HyperbolicSecant)
                .add_wavelength(WavelengthAcquisition::new(1550, 10.0))
                .add_wavelength(WavelengthAcquisition::new(1310, 10.0)),
            hardware: HardwareConfig::default(),
        };

        // Convert to YAML and back
        let yaml = serde_yaml::to_string(&YamlConfig {
            simulation: YamlSimulation {
                seed: config.seed,
                output_dir: config.output_dir.clone(),
                output_basename: config.output_basename.clone(),
            },
            network: config.network.clone(),
            acquisition: config.acquisition.clone(),
            hardware: config.hardware.clone(),
        }).unwrap();

        let parsed = SimulationConfig::from_yaml_str(&yaml).unwrap();

        assert_eq!(config.seed, parsed.seed);
        assert_eq!(config.output_basename, parsed.output_basename);
        assert_eq!(config.network.segments.len(), parsed.network.segments.len());
    }

    #[test]
    fn test_yaml_with_hardware_defaults() {
        let yaml = r#"
simulation:
  seed: 42
  output_dir: ./output
  output_basename: test

network:
  segments:
    - length_m: 1000
      fiber_type: G652D
  events: []

acquisition:
  wavelengths:
    - wavelength_nm: 1550
      pulse_width_ns: 10
  range_m: 5000
  averaging_time_s: 30
  pulse_shape: rectangular
"#;

        let config = SimulationConfig::from_yaml_str(yaml).unwrap();
        
        // Hardware should use defaults
        assert_eq!(config.hardware.noise_floor_dbm, -65.0);
    }
}
