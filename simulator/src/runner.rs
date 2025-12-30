//! Main simulation runner
//!
//! This module provides the high-level simulation API that ties together
//! all the components: network description, physics engine, post-processing,
//! and output generation.

use crate::config::SimulationConfig;
use crate::network::{NetworkDescription, FiberSegment, FiberType, NetworkEvent, ConnectorType, ConnectorState};
use crate::physics::simulate_physics;
use crate::postprocess::{apply_postprocessing, PostProcessConfig};
use crate::output::{SimulatedTrace, SimulationResult, generate_ground_truth};
use crate::SimulatorError;
use crate::units::{Length, Time, meter, second, nanosecond, watt};
use rayon::prelude::*;

/// Run a complete OTDR simulation
/// 
/// This is the main entry point for the simulation. It:
/// 1. Validates the configuration
/// 2. Builds effective network (with front panel + launch cable)
/// 3. Runs physics simulation for each wavelength
/// 4. Applies post-processing
/// 5. Generates output structures
/// 
/// Args:
/// - config: Complete simulation configuration
/// 
/// Returns: SimulationResult containing all traces and ground truth
pub fn run_simulation(config: &SimulationConfig) -> Result<SimulationResult, SimulatorError> {
    // Validate configuration
    config.validate()?;
    
    // Build effective network that includes front panel fiber and launch cable
    let effective_network = build_effective_network(config);
    
    let mut traces = Vec::new();
    let mut processed_traces = Vec::new();
    let mut physics_results = Vec::new();
    
    // Create post-processing config with simulation seed
    let pp_config = match config.seed {
        Some(seed) => PostProcessConfig::default().with_seed(seed),
        None => PostProcessConfig::default(),
    };
    
    // Run simulation for each wavelength
    // Use parallel iterator to run physics and post-processing for multiple wavelengths concurrently
    let results: Vec<_> = config.acquisition.wavelengths
        .par_iter()
        .map(|wl_acq| {
            // Run physics simulation on effective network
            // Convert configuration primitives to strong types
            let range = Length::new::<meter>(config.acquisition.range_m);
            
            let physics = simulate_physics(
                &effective_network,
                wl_acq,
                &config.hardware,
                config.acquisition.pulse_shape,
                range,
            );
            
            // Apply post-processing
            let averaging_time = Time::new::<second>(config.acquisition.averaging_time_s);
            
            let processed = apply_postprocessing(
                &physics.trace,
                &config.hardware,
                physics.delta_z,
                averaging_time,
                &pp_config,
            );
            
            // Convert results back to primitives for storage/output
            let processed_watts: Vec<f64> = processed.iter()
                .map(|p| p.get::<watt>())
                .collect();
            
            // Create trace structure
            let trace = SimulatedTrace {
                wavelength_nm: physics.wavelength_nm,
                pulse_width_ns: physics.pulse_width.get::<nanosecond>(),
                data_points: processed_watts.clone(),
                delta_z_m: physics.delta_z.get::<meter>(),
                num_points: physics.num_points,
            };
            
            (trace, processed_watts, physics)
        })
        .collect();

    // Store results
    for (trace, processed, physics) in results {
        traces.push(trace);
        processed_traces.push(processed);
        physics_results.push(physics);
    }
    
    // Generate ground truth (uses original network positions, relative to launch cable)
    let ground_truth = generate_ground_truth(config);
    
    Ok(SimulationResult {
        simulation_id: config.generate_id(),
        traces,
        ground_truth,
        processed_traces,
        physics_results,
    })
}

/// Build effective network that includes OTDR internal fiber and launch cable
/// 
/// The effective network prepends:
/// 1. Front panel offset (internal OTDR fiber, typically 20-100m)
/// 2. Launch cable (if configured)
/// 3. Launch cable end connector (at the interface to fiber under test)
/// 
/// All original network events are offset by the total prefix length.
fn build_effective_network(config: &SimulationConfig) -> NetworkDescription {
    let front_panel_offset = config.hardware.front_panel_offset_m;
    let launch_cable = config.acquisition.launch_cable.as_ref();
    
    let mut effective = NetworkDescription::new();
    
    // Add front panel fiber (internal to OTDR, uses typical G.652 fiber)
    if front_panel_offset > 0.0 {
        effective = effective.add_segment(FiberSegment::new(front_panel_offset, FiberType::G652D));
        // Front panel connector at end of internal fiber
        effective = effective.add_event(NetworkEvent::Connector {
            position_m: front_panel_offset,
            insertion_loss_db: 0.2, // Typical internal connector
            state: ConnectorState::Mated { return_loss_db: -50.0 }, // Good internal connection
            connector_type: ConnectorType::UPC,
        });
    }
    
    // Add launch cable if configured
    let mut total_offset = front_panel_offset;
    if let Some(lc) = launch_cable {
        effective = effective.add_segment(FiberSegment::new(lc.length_m, FiberType::G652D));
        total_offset += lc.length_m;
        
        // Connector at end of launch cable (interface to fiber under test)
        effective = effective.add_event(NetworkEvent::Connector {
            position_m: total_offset,
            insertion_loss_db: lc.connector_loss_db,
            state: ConnectorState::Mated { return_loss_db: -45.0 }, // Typical mated connector
            connector_type: match lc.connector_type {
                crate::acquisition::LaunchConnectorType::UPC => ConnectorType::UPC,
                crate::acquisition::LaunchConnectorType::APC => ConnectorType::APC,
            },
        });
    }
    
    // Add original network segments
    for segment in &config.network.segments {
        effective = effective.add_segment(segment.clone());
    }
    
    // Add original network events with offset positions
    for event in &config.network.events {
        let offset_event = offset_event(event, total_offset);
        effective = effective.add_event(offset_event);
    }
    
    // Set network name
    if let Some(ref name) = config.network.name {
        effective = effective.with_name(format!("Effective: {}", name));
    }
    
    effective
}

/// Offset an event's position by a given amount
fn offset_event(event: &NetworkEvent, offset_m: f64) -> NetworkEvent {
    match event {
        NetworkEvent::Splice { position_m, insertion_loss_db, splice_type } => {
            NetworkEvent::Splice {
                position_m: position_m + offset_m,
                insertion_loss_db: *insertion_loss_db,
                splice_type: splice_type.clone(),
            }
        }
        NetworkEvent::Connector { position_m, insertion_loss_db, state, connector_type } => {
            NetworkEvent::Connector {
                position_m: position_m + offset_m,
                insertion_loss_db: *insertion_loss_db,
                state: state.clone(),
                connector_type: connector_type.clone(),
            }
        }
        NetworkEvent::Macrobend { position_m, bend_radius_mm, turns, span_m } => {
            NetworkEvent::Macrobend {
                position_m: position_m + offset_m,
                bend_radius_mm: *bend_radius_mm,
                turns: *turns,
                span_m: *span_m,
            }
        }
        NetworkEvent::Splitter { position_m, split_ratio, excess_loss_db } => {
            NetworkEvent::Splitter {
                position_m: position_m + offset_m,
                split_ratio: *split_ratio,
                excess_loss_db: *excess_loss_db,
            }
        }
        NetworkEvent::End { position_m, end_type } => {
            NetworkEvent::End {
                position_m: position_m + offset_m,
                end_type: end_type.clone(),
            }
        }
    }
}

/// Run simulation and write output files
/// 
/// Convenience function that runs the simulation and writes all output files.
/// 
/// Args:
/// - config: Complete simulation configuration
/// 
/// Returns: List of paths to generated files
pub fn run_and_save(config: &SimulationConfig) -> Result<Vec<std::path::PathBuf>, SimulatorError> {
    let result = run_simulation(config)?;
    
    let output_dir = std::path::Path::new(&config.output_dir);
    result.write_outputs(output_dir, &config.output_basename, config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{NetworkDescription, FiberSegment, FiberType, NetworkEvent, EndType, SpliceType, ConnectorType, ConnectorState};
    use crate::acquisition::{AcquisitionConfig, WavelengthAcquisition, PulseShape, HardwareConfig};
    use tempfile::TempDir;

    #[test]
    fn test_run_simulation_simple() {
        let config = SimulationConfig {
            seed: Some(42),
            output_dir: "./test".to_string(),
            output_basename: "test".to_string(),
            network: NetworkDescription::new()
                .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
                .add_event(NetworkEvent::End {
                    position_m: 1000.0,
                    end_type: EndType::Cleaved,
                }),
            acquisition: AcquisitionConfig::new(2000.0, 30.0, PulseShape::Rectangular)
                .add_wavelength(WavelengthAcquisition::new(1550, 10.0)),
            hardware: HardwareConfig::default(),
        };
        
        let result = run_simulation(&config).unwrap();
        
        assert_eq!(result.traces.len(), 1);
        assert_eq!(result.traces[0].wavelength_nm, 1550);
        assert!(result.traces[0].num_points > 0);
    }

    #[test]
    fn test_run_simulation_multi_wavelength() {
        let config = SimulationConfig {
            seed: Some(42),
            output_dir: "./test".to_string(),
            output_basename: "test".to_string(),
            network: NetworkDescription::new()
                .add_segment(FiberSegment::new(2000.0, FiberType::G652D))
                .add_event(NetworkEvent::Macrobend {
                    position_m: 1000.0,
                    bend_radius_mm: 10.0,
                    turns: 3.0,
                    span_m: 0.5,
                }),
            acquisition: AcquisitionConfig::new(5000.0, 30.0, PulseShape::HyperbolicSecant)
                .add_wavelength(WavelengthAcquisition::new(1310, 10.0))
                .add_wavelength(WavelengthAcquisition::new(1550, 10.0))
                .add_wavelength(WavelengthAcquisition::new(1625, 10.0)),
            hardware: HardwareConfig::default(),
        };
        
        let result = run_simulation(&config).unwrap();
        
        assert_eq!(result.traces.len(), 3);
        assert_eq!(result.ground_truth.wavelengths.len(), 3);
        
        // Check wavelength-dependent macrobend loss in ground truth
        let macrobend_event = result.ground_truth.events.iter()
            .find(|e| e.event_type == "macrobend")
            .expect("Should have macrobend event");
        
        if let crate::output::sidecar::GroundTruthEventParams::Macrobend { loss_by_wavelength, .. } = &macrobend_event.parameters {
            // Longer wavelengths should have higher macrobend loss
            let loss_1310 = loss_by_wavelength.get(&1310).unwrap();
            let loss_1550 = loss_by_wavelength.get(&1550).unwrap();
            let loss_1625 = loss_by_wavelength.get(&1625).unwrap();
            
            assert!(loss_1625 > loss_1550);
            assert!(loss_1550 > loss_1310);
        } else {
            panic!("Expected macrobend parameters");
        }
    }

    #[test]
    fn test_run_and_save() {
        let temp_dir = TempDir::new().unwrap();
        
        let config = SimulationConfig {
            seed: Some(42),
            output_dir: temp_dir.path().to_str().unwrap().to_string(),
            output_basename: "sim_test".to_string(),
            network: NetworkDescription::new()
                .add_segment(FiberSegment::new(500.0, FiberType::G652D))
                .add_event(NetworkEvent::Connector {
                    position_m: 500.0,
                    insertion_loss_db: 0.3,
                    state: ConnectorState::Open,
                    connector_type: ConnectorType::UPC,
                }),
            acquisition: AcquisitionConfig::new(1000.0, 10.0, PulseShape::Rectangular)
                .add_wavelength(WavelengthAcquisition::new(1550, 10.0)),
            hardware: HardwareConfig::default(),
        };
        
        let output_files = run_and_save(&config).unwrap();
        
        // Should have 2 files: 1 SOR + 1 JSON sidecar
        assert_eq!(output_files.len(), 2);
        
        // Check files exist
        for path in &output_files {
            assert!(path.exists());
        }
        
        // Check SOR file
        let sor_path = output_files.iter()
            .find(|p| p.extension().map_or(false, |e| e == "sor"))
            .expect("Should have SOR file");
        assert!(sor_path.exists());
        
        // Check JSON sidecar
        let json_path = output_files.iter()
            .find(|p| p.extension().map_or(false, |e| e == "json"))
            .expect("Should have JSON file");
        assert!(json_path.exists());
        
        // Verify JSON is valid
        let json_content = std::fs::read_to_string(json_path).unwrap();
        let _ground_truth: crate::output::GroundTruth = serde_json::from_str(&json_content).unwrap();
    }

    #[test]
    fn test_reproducibility_with_seed() {
        let config = SimulationConfig {
            seed: Some(12345),
            output_dir: "./test".to_string(),
            output_basename: "test".to_string(),
            network: NetworkDescription::new()
                .add_segment(FiberSegment::new(1000.0, FiberType::G652D)),
            acquisition: AcquisitionConfig::new(2000.0, 5.0, PulseShape::Rectangular)
                .add_wavelength(WavelengthAcquisition::new(1550, 10.0)),
            hardware: HardwareConfig::default(),
        };
        
        let result1 = run_simulation(&config).unwrap();
        let result2 = run_simulation(&config).unwrap();
        
        // With same seed, traces should be identical
        let trace1 = &result1.traces[0].data_points;
        let trace2 = &result2.traces[0].data_points;
        
        assert_eq!(trace1.len(), trace2.len());
        for i in 0..trace1.len() {
            assert_eq!(trace1[i], trace2[i]);
        }
    }
}
