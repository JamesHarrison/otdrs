//! Ground truth JSON sidecar generation
//!
//! This module generates JSON sidecar files containing the ground truth
//! for simulated OTDR traces. This includes exact event positions and
//! parameters, which is useful for ML training and validation.

use std::collections::HashMap;
use std::path::Path;
use std::fs::File;
use std::io::Write;

use serde::{Serialize, Deserialize};

use crate::config::SimulationConfig;
use crate::network::{NetworkEvent, FiberType};
use crate::SimulatorError;

/// Complete ground truth for a simulation run
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruth {
    /// Unique simulation identifier
    pub simulation_id: String,
    /// Simulation seed (if any)
    pub seed: Option<u64>,
    /// Total fiber length in meters
    pub total_length_m: f64,
    /// Ground truth events
    pub events: Vec<GroundTruthEvent>,
    /// Fiber segments
    pub segments: Vec<GroundTruthSegment>,
    /// Wavelengths simulated
    pub wavelengths: Vec<u16>,
    /// Configuration summary
    pub config: GroundTruthConfig,
}

/// Ground truth for a single event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthEvent {
    /// Position along fiber in meters
    pub position_m: f64,
    /// Event type
    pub event_type: String,
    /// Event-specific parameters
    pub parameters: GroundTruthEventParams,
}

/// Event-specific parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum GroundTruthEventParams {
    Splice {
        insertion_loss_db: f64,
        splice_type: String,
    },
    Connector {
        insertion_loss_db: f64,
        return_loss_db: Option<f64>,
        connector_type: String,
        state: String,
    },
    Macrobend {
        bend_radius_mm: f64,
        turns: f64,
        span_m: f64,
        /// Loss at each wavelength
        loss_by_wavelength: HashMap<u16, f64>,
    },
    Splitter {
        split_ratio: u32,
        excess_loss_db: f64,
        total_loss_db: f64,
    },
    End {
        end_type: String,
        return_loss_db: f64,
    },
}

/// Ground truth for a fiber segment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthSegment {
    /// Start position in meters
    pub start_m: f64,
    /// End position in meters
    pub end_m: f64,
    /// Fiber type name
    pub fiber_type: String,
    /// Attenuation at each wavelength in dB/km
    pub attenuation_by_wavelength: HashMap<u16, f64>,
}

/// Configuration summary for sidecar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundTruthConfig {
    pub range_m: f64,
    pub averaging_time_s: f64,
    pub pulse_shape: String,
    /// Front panel offset in meters (internal OTDR fiber)
    pub front_panel_offset_m: f64,
    /// Launch cable configuration (if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub launch_cable: Option<LaunchCableInfo>,
}

/// Launch cable information for ground truth
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LaunchCableInfo {
    pub length_m: f64,
    pub connector_type: String,
    pub connector_loss_db: f64,
}

/// Generate ground truth from simulation configuration
pub fn generate_ground_truth(config: &SimulationConfig) -> GroundTruth {
    let wavelengths: Vec<u16> = config.acquisition.wavelengths.iter()
        .map(|w| w.wavelength_nm)
        .collect();
    
    // Generate events
    let events = config.network.events.iter()
        .map(|e| event_to_ground_truth(e, &wavelengths))
        .collect();
    
    // Generate segments
    let mut segments = Vec::new();
    let mut current_pos = 0.0;
    for segment in &config.network.segments {
        let start_m = current_pos;
        let end_m = current_pos + segment.length_m;
        
        let mut attenuation_by_wavelength = HashMap::new();
        for &wl in &wavelengths {
            let chars = segment.fiber_type.characteristics(wl);
            let attn = segment.attenuation_override.unwrap_or(chars.attenuation_db_km);
            attenuation_by_wavelength.insert(wl, attn);
        }
        
        segments.push(GroundTruthSegment {
            start_m,
            end_m,
            fiber_type: fiber_type_name(&segment.fiber_type),
            attenuation_by_wavelength,
        });
        
        current_pos = end_m;
    }
    
    GroundTruth {
        simulation_id: config.generate_id().to_string(),
        seed: config.seed,
        total_length_m: config.network.total_length(),
        events,
        segments,
        wavelengths,
        config: GroundTruthConfig {
            range_m: config.acquisition.range_m,
            averaging_time_s: config.acquisition.averaging_time_s,
            pulse_shape: format!("{:?}", config.acquisition.pulse_shape),
            front_panel_offset_m: config.hardware.front_panel_offset_m,
            launch_cable: config.acquisition.launch_cable.as_ref().map(|lc| LaunchCableInfo {
                length_m: lc.length_m,
                connector_type: format!("{:?}", lc.connector_type),
                connector_loss_db: lc.connector_loss_db,
            }),
        },
    }
}

/// Convert network event to ground truth event
fn event_to_ground_truth(event: &NetworkEvent, wavelengths: &[u16]) -> GroundTruthEvent {
    match event {
        NetworkEvent::Splice { position_m, insertion_loss_db, splice_type } => {
            GroundTruthEvent {
                position_m: *position_m,
                event_type: "splice".to_string(),
                parameters: GroundTruthEventParams::Splice {
                    insertion_loss_db: *insertion_loss_db,
                    splice_type: format!("{:?}", splice_type),
                },
            }
        }
        NetworkEvent::Connector { position_m, insertion_loss_db, state, connector_type } => {
            let (return_loss, state_str) = match state {
                crate::network::ConnectorState::Mated { return_loss_db } => {
                    (Some(*return_loss_db), "mated".to_string())
                }
                crate::network::ConnectorState::Open => {
                    (Some(connector_type.open_return_loss_db()), "open".to_string())
                }
            };
            
            GroundTruthEvent {
                position_m: *position_m,
                event_type: "connector".to_string(),
                parameters: GroundTruthEventParams::Connector {
                    insertion_loss_db: *insertion_loss_db,
                    return_loss_db: return_loss,
                    connector_type: format!("{:?}", connector_type),
                    state: state_str,
                },
            }
        }
        NetworkEvent::Macrobend { position_m, bend_radius_mm, turns, span_m } => {
            // Calculate wavelength-dependent loss
            // Note: We'd need the fiber type at this position for accurate calculation
            // For now, use a simplified model
            let mut loss_by_wavelength = HashMap::new();
            for &wl in wavelengths {
                // Simplified macrobend loss estimate
                // In a real implementation, we'd look up the fiber type at this position
                let base_loss = 0.1 * turns; // 0.1 dB/turn baseline
                let wavelength_factor = (wl as f64 / 1550.0).powi(3);
                let radius_factor = (15.0 / *bend_radius_mm).exp().min(100.0);
                let loss = base_loss * wavelength_factor * radius_factor;
                loss_by_wavelength.insert(wl, loss);
            }
            
            GroundTruthEvent {
                position_m: *position_m,
                event_type: "macrobend".to_string(),
                parameters: GroundTruthEventParams::Macrobend {
                    bend_radius_mm: *bend_radius_mm,
                    turns: *turns,
                    span_m: *span_m,
                    loss_by_wavelength,
                },
            }
        }
        NetworkEvent::Splitter { position_m, split_ratio, excess_loss_db } => {
            let ideal_loss = 10.0 * (*split_ratio as f64).log10();
            let total_loss = ideal_loss + excess_loss_db;
            
            GroundTruthEvent {
                position_m: *position_m,
                event_type: "splitter".to_string(),
                parameters: GroundTruthEventParams::Splitter {
                    split_ratio: *split_ratio,
                    excess_loss_db: *excess_loss_db,
                    total_loss_db: total_loss,
                },
            }
        }
        NetworkEvent::End { position_m, end_type } => {
            GroundTruthEvent {
                position_m: *position_m,
                event_type: "end".to_string(),
                parameters: GroundTruthEventParams::End {
                    end_type: format!("{:?}", end_type),
                    return_loss_db: end_type.return_loss_db(),
                },
            }
        }
    }
}

/// Get fiber type name string
fn fiber_type_name(fiber_type: &FiberType) -> String {
    match fiber_type {
        FiberType::G652D => "G.652.D".to_string(),
        FiberType::G657A1 => "G.657.A1".to_string(),
        FiberType::G657A2 => "G.657.A2".to_string(),
        FiberType::G657B3 => "G.657.B3".to_string(),
        FiberType::Custom(_) => "Custom".to_string(),
    }
}

/// Write ground truth to JSON file
pub fn write_sidecar_file(
    ground_truth: &GroundTruth,
    output_path: &Path,
) -> Result<(), SimulatorError> {
    let json = serde_json::to_string_pretty(ground_truth)
        .map_err(|e| SimulatorError::OutputError(format!("JSON serialization error: {}", e)))?;
    
    let mut file = File::create(output_path)
        .map_err(|e| SimulatorError::OutputError(format!("Failed to create file: {}", e)))?;
    
    file.write_all(json.as_bytes())
        .map_err(|e| SimulatorError::OutputError(format!("Failed to write file: {}", e)))?;
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{NetworkDescription, FiberSegment, FiberType, NetworkEvent, EndType, SpliceType};
    use crate::acquisition::{AcquisitionConfig, WavelengthAcquisition, PulseShape, HardwareConfig};

    #[test]
    fn test_generate_ground_truth() {
        let config = SimulationConfig {
            seed: Some(42),
            output_dir: "./test".to_string(),
            output_basename: "test".to_string(),
            network: NetworkDescription::new()
                .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
                .add_segment(FiberSegment::new(500.0, FiberType::G657A1))
                .add_event(NetworkEvent::Splice {
                    position_m: 1000.0,
                    insertion_loss_db: 0.05,
                    splice_type: SpliceType::Fusion,
                })
                .add_event(NetworkEvent::End {
                    position_m: 1500.0,
                    end_type: EndType::Cleaved,
                }),
            acquisition: AcquisitionConfig::new(2000.0, 30.0, PulseShape::Rectangular)
                .add_wavelength(WavelengthAcquisition::new(1310, 10.0))
                .add_wavelength(WavelengthAcquisition::new(1550, 10.0)),
            hardware: HardwareConfig::default(),
        };
        
        let gt = generate_ground_truth(&config);
        
        assert_eq!(gt.total_length_m, 1500.0);
        assert_eq!(gt.events.len(), 2);
        assert_eq!(gt.segments.len(), 2);
        assert_eq!(gt.wavelengths.len(), 2);
    }

    #[test]
    fn test_ground_truth_serialization() {
        let config = SimulationConfig {
            seed: Some(42),
            output_dir: "./test".to_string(),
            output_basename: "test".to_string(),
            network: NetworkDescription::new()
                .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
                .add_event(NetworkEvent::Macrobend {
                    position_m: 500.0,
                    bend_radius_mm: 15.0,
                    turns: 2.0,
                    span_m: 0.5,
                }),
            acquisition: AcquisitionConfig::new(2000.0, 30.0, PulseShape::Rectangular)
                .add_wavelength(WavelengthAcquisition::new(1310, 10.0))
                .add_wavelength(WavelengthAcquisition::new(1550, 10.0))
                .add_wavelength(WavelengthAcquisition::new(1625, 10.0)),
            hardware: HardwareConfig::default(),
        };
        
        let gt = generate_ground_truth(&config);
        let json = serde_json::to_string_pretty(&gt).unwrap();
        
        // Check wavelength-dependent macrobend loss is present
        assert!(json.contains("loss_by_wavelength"));
        assert!(json.contains("1310"));
        assert!(json.contains("1550"));
        assert!(json.contains("1625"));
    }
}
