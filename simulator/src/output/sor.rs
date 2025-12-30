//! SOR file generation from simulated traces
//!
//! This module creates compliant SOR (Standard OTDR Record) files from
//! simulated OTDR traces using the `otdrs` crate.

use std::path::Path;
use std::fs::File;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

use otdrs::types::{
    SORFile, MapBlock, BlockInfo, GeneralParametersBlock, SupplierParametersBlock,
    FixedParametersBlock, DataPoints, DataPointsAtScaleFactor, KeyEvents, KeyEvent,
    LastKeyEvent, ChecksumBlock,
};

use crate::config::SimulationConfig;
use crate::physics::PhysicsResult;
use crate::SimulatorError;
use crate::units::{nanosecond, meter};

/// Generate a SOR file from simulation results
/// 
/// Args:
/// - physics_result: The raw physics simulation output
/// - post_processed_trace: The post-processed trace in watts
/// - config: Simulation configuration
/// - output_path: Path to write the SOR file
/// 
/// Returns: Ok(()) on success, error otherwise
pub fn write_sor_file(
    physics_result: &PhysicsResult,
    post_processed_trace: &[f64],
    config: &SimulationConfig,
    output_path: &Path,
) -> Result<(), SimulatorError> {
    let sor = create_sor_file(physics_result, post_processed_trace, config)?;
    
    let bytes = sor.to_bytes()
        .map_err(|e| SimulatorError::OutputError(format!("SOR generation error: {:?}", e)))?;
    
    let mut file = File::create(output_path)
        .map_err(|e| SimulatorError::OutputError(format!("Failed to create file: {}", e)))?;
    
    file.write_all(&bytes)
        .map_err(|e| SimulatorError::OutputError(format!("Failed to write file: {}", e)))?;
    
    Ok(())
}

/// Create a SORFile structure from simulation results
pub fn create_sor_file(
    physics_result: &PhysicsResult,
    post_processed_trace: &[f64],
    config: &SimulationConfig,
) -> Result<SORFile, SimulatorError> {
    // Convert trace to OTDR dB (5 log10) for SOR format
    // Using 5 log10 (one-way loss) prevents u16 overflow for high dynamic range
    let trace_otdr_db = trace_watts_to_otdr_db(post_processed_trace);
    
    // Find the reference power (maximum point) for absolute reference
    let reference_db = trace_otdr_db.iter()
        .filter(|v| v.is_finite())
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    
    // Convert to SOR data format (u16, dB*1000 relative to reference)
    let data_points = convert_to_sor_data(&trace_otdr_db);
    
    // Get current timestamp
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    
    // Create block info for map
    // Block identifiers must match what otdrs expects
    let block_info = vec![
        BlockInfo { identifier: "Map".to_string(), revision_number: 200, size: 0 },
        BlockInfo { identifier: "GenParams".to_string(), revision_number: 200, size: 0 },
        BlockInfo { identifier: "SupParams".to_string(), revision_number: 200, size: 0 },
        BlockInfo { identifier: "FxdParams".to_string(), revision_number: 200, size: 0 },
        BlockInfo { identifier: "KeyEvents".to_string(), revision_number: 200, size: 0 },
        BlockInfo { identifier: "DataPts".to_string(), revision_number: 200, size: 0 },
        BlockInfo { identifier: "Cksum".to_string(), revision_number: 200, size: 0 },
    ];
    
    let map = MapBlock {
        revision_number: 200,
        block_size: 0, // Will be calculated
        block_count: block_info.len() as i16,
        block_info,
    };
    
    // Calculate pulse width in ns and data spacing in 100ps units
    let pulse_width_ns = physics_result.pulse_width.get::<nanosecond>() as i16;
    let delta_z_m = physics_result.delta_z.get::<meter>();
    
    // delta_z (m) -> delta_t = delta_z * n_g / c
    // data_spacing is time for 10000 points in 100ps units
    // Note: SOR uses one-way propagation time, OTDR software handles display
    let c = 299792458.0; // m/s
    let delta_t_s = delta_z_m * config.hardware.group_index / c;
    // Each point is delta_t_s seconds apart, for 10000 points:
    let data_spacing_100ps = (delta_t_s * 1e10 * 10000.0).round() as i32;
    
    // Calculate total offset from OTDR to fiber under test (front panel + launch cable)
    let _launch_cable_length = config.acquisition.launch_cable.as_ref()
        .map(|lc| lc.length_m)
        .unwrap_or(0.0);
    
    // Acquisition range in 100ps units (one-way propagation time)
    let range_100ps = (config.acquisition.range_m * config.hardware.group_index / c * 1e10).round() as i32;
    
    // Calculate front panel offset in 100ps units (one-way propagation time)
    let front_panel_offset_100ps = (config.hardware.front_panel_offset_m * config.hardware.group_index / c * 1e10).round() as i32;
    
    // Calculate user offset (launch cable length) in 100ps units (one-way)
    let (user_offset_100ps, user_offset_distance_10m) = if let Some(ref launch) = config.acquisition.launch_cable {
        let offset_100ps = (launch.length_m * config.hardware.group_index / c * 1e10).round() as i32;
        let offset_dist = (launch.length_m * 10.0).round() as i32; // 10x distance units (meters)
        (offset_100ps, offset_dist)
    } else {
        (0, 0)
    };
    
    let general_parameters = GeneralParametersBlock {
        language_code: "EN".to_string(),
        cable_id: config.network.name.clone().unwrap_or_default(),
        fiber_id: format!("SIM-{}", physics_result.wavelength_nm),
        fiber_type: 652, // G.652 as default
        nominal_wavelength: physics_result.wavelength_nm as i16,
        originating_location: "SIMULATOR".to_string(),
        terminating_location: "END".to_string(),
        cable_code: "SIMULATED".to_string(),
        current_data_flag: "NC".to_string(),
        user_offset: user_offset_100ps,
        user_offset_distance: user_offset_distance_10m,
        operator: "otdrs-simulator".to_string(),
        comment: format!("Simulated trace - seed {:?}", config.seed),
    };
    
    let supplier_parameters = SupplierParametersBlock {
        supplier_name: "OTDRS Simulator".to_string(),
        otdr_mainframe_id: "OTDRS-SIM".to_string(),
        otdr_mainframe_sn: config.generate_id().to_string(),
        optical_module_id: format!("{}nm", physics_result.wavelength_nm),
        optical_module_sn: "SIM-001".to_string(),
        software_revision: env!("CARGO_PKG_VERSION").to_string(),
        other: "Simulated OTDR trace".to_string(),
    };
    
    let fixed_parameters = FixedParametersBlock {
        date_time_stamp: timestamp,
        units_of_distance: "mt".to_string(),
        actual_wavelength: physics_result.wavelength_nm as i16,
        acquisition_offset: 0,
        acquisition_offset_distance: 0,
        total_n_pulse_widths_used: 1,
        pulse_widths_used: vec![pulse_width_ns],
        data_spacing: vec![data_spacing_100ps],
        n_data_points_for_pulse_widths_used: vec![physics_result.num_points as i32],
        group_index: (config.hardware.group_index * 100000.0) as i32,
        backscatter_coefficient: -790, // -79.0 dB typical
        number_of_averages: config.acquisition.num_averages.unwrap_or(1000) as i32,
        averaging_time: (config.acquisition.averaging_time_s * 10.0) as u16,
        acquisition_range: range_100ps,
        acquisition_range_distance: (config.acquisition.range_m * 10.0) as i32,
        front_panel_offset: front_panel_offset_100ps,
        noise_floor_level: ((config.hardware.noise_floor_dbm.abs() * 100.0) as u16),
        noise_floor_scale_factor: 1,
        // Power offset stores the reference in dB*1000
        power_offset_first_point: ((reference_db.abs() * 1000.0) as u16).min(65535),
        loss_threshold: 200, // 0.2 dB
        reflectance_threshold: 55000, // -55 dB (as positive value)
        end_of_fibre_threshold: 3000, // 3.0 dB
        trace_type: "ST".to_string(), // Standard trace
        window_coordinate_1: 0,
        window_coordinate_2: 0,
        window_coordinate_3: 0,
        window_coordinate_4: 0,
    };
    
    // Create minimal key events (just end of fiber)
    let key_events = create_key_events(config, physics_result);
    
    let data_points_block = DataPoints {
        number_of_data_points: data_points.len() as i32,
        total_number_scale_factors_used: 1,
        scale_factors: vec![DataPointsAtScaleFactor {
            n_points: data_points.len() as i32,
            scale_factor: 1000, // 1.0 scale factor
            data: data_points,
        }],
    };
    
    Ok(SORFile {
        map,
        general_parameters: Some(general_parameters),
        supplier_parameters: Some(supplier_parameters),
        fixed_parameters: Some(fixed_parameters),
        key_events: Some(key_events),
        link_parameters: None,
        data_points: Some(data_points_block),
        proprietary_blocks: vec![],
        checksum: Some(ChecksumBlock { checksum: 0 }), // Will be calculated
    })
}

/// Convert trace from watts to OTDR dB (5 log10)
/// 
/// OTDR traces are typically stored using 5.0 * log10(P) scaling
/// so that the displayed slope matches the one-way fiber attenuation.
fn trace_watts_to_otdr_db(trace: &[f64]) -> Vec<f64> {
    trace.iter()
        .map(|&power| {
            if power <= 0.0 {
                f64::NEG_INFINITY
            } else {
                // Using 5.0 * log10(P_mw) for one-way equivalent dB
                5.0 * (power * 1000.0).log10() 
            }
        })
        .collect()
}

/// Convert OTDR dB trace to SOR u16 format
fn convert_to_sor_data(trace_db: &[f64]) -> Vec<u16> {
    // SOR uses dB*1000 as u16
    // Find reference (maximum) power for relative dB
    let max_db = trace_db.iter()
        .filter(|v| v.is_finite())
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    
    trace_db.iter()
        .map(|&db| {
            if !db.is_finite() || db < max_db - 60.0 {
                // Below dynamic range (clamping at 60dB loss which is 120dB power)
                65535
            } else {
                // Relative dB from max, scaled by 1000, as unsigned
                let relative_db = max_db - db;
                let scaled = (relative_db * 1000.0).round() as i32;
                scaled.clamp(0, 65534) as u16
            }
        })
        .collect()
}

/// Create key events from network configuration
fn create_key_events(config: &SimulationConfig, _physics_result: &PhysicsResult) -> KeyEvents {
    let c = 299792458.0;
    let mut key_events = Vec::new();
    let mut event_number = 0;
    
    // Calculate offset from OTDR to fiber under test start
    // This includes front panel offset (internal OTDR fiber) and launch cable
    let launch_cable_length = config.acquisition.launch_cable.as_ref()
        .map(|lc| lc.length_m)
        .unwrap_or(0.0);
    let total_offset_m = config.hardware.front_panel_offset_m + launch_cable_length;
    
    // Add events from network
    for event in config.network.events_sorted() {
        // Event position relative to fiber under test
        let position_from_launch = event.position_m();
        // Absolute position from OTDR (add offset)
        let absolute_position_m = total_offset_m + position_from_launch;
        // Convert to propagation time (one-way, in 100ps units)
        let prop_time_100ps = (absolute_position_m * config.hardware.group_index / c * 1e10).round() as i32;
        
        let (loss, reflectance, code) = match event {
            crate::network::NetworkEvent::Splice { insertion_loss_db, .. } => {
                ((*insertion_loss_db * 1000.0) as i16, 0, "0F9999")
            }
            crate::network::NetworkEvent::Connector { insertion_loss_db, state, .. } => {
                let refl = match state {
                    crate::network::ConnectorState::Open => -14000, // -14 dB
                    crate::network::ConnectorState::Mated { return_loss_db } => {
                        (return_loss_db * 1000.0) as i32
                    }
                };
                ((*insertion_loss_db * 1000.0) as i16, refl, "1F9999")
            }
            crate::network::NetworkEvent::Macrobend { .. } => {
                // Macrobends appear as non-reflective loss
                (100, 0, "0F9999") // Placeholder loss
            }
            crate::network::NetworkEvent::Splitter { split_ratio, excess_loss_db, .. } => {
                let ideal_loss = 10.0 * (*split_ratio as f64).log10();
                let total = ideal_loss + excess_loss_db;
                ((total * 1000.0) as i16, 0, "0F9999")
            }
            crate::network::NetworkEvent::End { end_type, .. } => {
                let refl = match end_type {
                    crate::network::EndType::Cleaved => -14000,
                    crate::network::EndType::Broken => -20000,
                    crate::network::EndType::Terminated => -60000,
                    crate::network::EndType::Connected { return_loss_db } => {
                        (return_loss_db * 1000.0) as i32
                    }
                };
                (0, refl, "1E9999") // E = end of fiber
            }
        };
        
        key_events.push(KeyEvent {
            event_number,
            event_propogation_time: prop_time_100ps,
            attenuation_coefficient_lead_in_fiber: 220, // 0.22 dB/km typical
            event_loss: loss,
            event_reflectance: reflectance,
            event_code: code.to_string(),
            loss_measurement_technique: "2P".to_string(),
            marker_location_1: prop_time_100ps.saturating_sub(1000),
            marker_location_2: prop_time_100ps.saturating_add(1000),
            marker_location_3: 0,
            marker_location_4: 0,
            marker_location_5: prop_time_100ps,
            comment: String::new(),
        });
        
        event_number += 1;
    }
    
    // If no events, create a minimal end event
    if key_events.is_empty() {
        let total_length = config.network.total_length();
        let prop_time_100ps = (total_length * config.hardware.group_index / c * 1e10).round() as i32;
        
        key_events.push(KeyEvent {
            event_number: 0,
            event_propogation_time: prop_time_100ps,
            attenuation_coefficient_lead_in_fiber: 220,
            event_loss: 0,
            event_reflectance: -14000,
            event_code: "1E9999".to_string(),
            loss_measurement_technique: "2P".to_string(),
            marker_location_1: prop_time_100ps.saturating_sub(1000),
            marker_location_2: prop_time_100ps.saturating_add(1000),
            marker_location_3: 0,
            marker_location_4: 0,
            marker_location_5: prop_time_100ps,
            comment: "End of fiber".to_string(),
        });
    }
    
    // Create last key event from the actual last event
    let last = key_events.last().unwrap();
    let last_key_event = LastKeyEvent {
        event_number: last.event_number,
        event_propogation_time: last.event_propogation_time,
        attenuation_coefficient_lead_in_fiber: last.attenuation_coefficient_lead_in_fiber,
        event_loss: last.event_loss,
        event_reflectance: last.event_reflectance,
        event_code: last.event_code.clone(),
        loss_measurement_technique: last.loss_measurement_technique.clone(),
        marker_location_1: last.marker_location_1,
        marker_location_2: last.marker_location_2,
        marker_location_3: last.marker_location_3,
        marker_location_4: last.marker_location_4,
        marker_location_5: last.marker_location_5,
        comment: last.comment.clone(),
        end_to_end_loss: 0, // Could calculate
        end_to_end_marker_position_1: 0,
        end_to_end_marker_position_2: last.event_propogation_time,
        optical_return_loss: 0,
        optical_return_loss_marker_position_1: 0,
        optical_return_loss_marker_position_2: last.event_propogation_time,
    };
    
    // Remove the last event from key_events since it's now in last_key_event
    if !key_events.is_empty() {
        key_events.pop();
    }
    
    KeyEvents {
        number_of_key_events: (key_events.len() + 1) as i16,
        key_events,
        last_key_event,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{NetworkDescription, FiberSegment, FiberType, NetworkEvent, EndType};
    use crate::acquisition::{AcquisitionConfig, WavelengthAcquisition, PulseShape, HardwareConfig};
    use crate::units::{Power, Length, Time, milliwatt, meter, nanosecond};

    #[test]
    fn test_convert_to_sor_data() {
        let trace_dbm = vec![0.0, -5.0, -10.0, -20.0];
        let sor_data = convert_to_sor_data(&trace_dbm);
        
        // First point (max) should be 0
        assert_eq!(sor_data[0], 0);
        // -5 dB relative = 5000
        assert_eq!(sor_data[1], 5000);
        // -10 dB relative = 10000
        assert_eq!(sor_data[2], 10000);
    }

    #[test]
    fn test_create_sor_file() {
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
        
        let physics = PhysicsResult {
            trace: vec![Power::new::<milliwatt>(1e-6); 1000],
            delta_z: Length::new::<meter>(1.0),
            num_points: 1000,
            wavelength_nm: 1550,
            pulse_width: Time::new::<nanosecond>(10.0),
        };
        
        let trace = vec![1e-6; 1000];
        
        let sor = create_sor_file(&physics, &trace, &config).unwrap();
        
        assert!(sor.general_parameters.is_some());
        assert!(sor.fixed_parameters.is_some());
        assert!(sor.data_points.is_some());
        assert!(sor.key_events.is_some());
        
        // Verify wavelength is set
        assert_eq!(
            sor.general_parameters.unwrap().nominal_wavelength,
            1550
        );
    }
}
