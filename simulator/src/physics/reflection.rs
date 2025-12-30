//! Fresnel reflection impulse response calculation
//!
//! This module computes the reflection impulse response h_r(z) for discrete
//! reflective events in the fiber network.

use crate::network::{NetworkDescription, NetworkEvent, ConnectorState, FiberType};
use crate::units::{Length, Ratio, meter, kilometer, ratio};

/// Compute the reflection impulse response for a fiber network
/// 
/// Returns a vector of reflection coefficients (dimensionless ratios).
/// The actual power is determined when convolved with the pulse.
pub fn compute_reflection_impulse(
    network: &NetworkDescription,
    wavelength_nm: u16,
    delta_z: Length,
    num_points: usize,
) -> Vec<Ratio> {
    let mut h_r = vec![Ratio::new::<ratio>(0.0); num_points];
    
    // Build segment boundaries for attenuation calculation
    let segment_boundaries = build_segment_boundaries(network);
    
    // Process each event
    for event in &network.events {
        if let Some((return_loss_db, position)) = get_reflection_params(event) {
            // Find the sample index for this position
            let index = (position.get::<meter>() / delta_z.get::<meter>()).round() as usize;
            
            if index >= num_points {
                continue;
            }
            
            // Calculate cumulative attenuation to this point (in dB)
            let cumulative_attenuation_db = calculate_cumulative_attenuation(
                position,
                wavelength_nm,
                &segment_boundaries,
            );
            
            // Use the ReturnLoss type helper which handles both positive and negative input
            // by enforcing the Return Loss convention (positive dB = loss).
            // This robustly handles configuration like "45.0" (Return Loss) or "-45.0" (Reflectance dB).
            let reflection_fraction = crate::units::ReturnLoss::from_db(return_loss_db).as_ratio();
            let two_way_loss = Ratio::new::<ratio>(10.0_f64.powf(-2.0 * cumulative_attenuation_db / 10.0));
            
            h_r[index] += reflection_fraction * two_way_loss;
        }
    }
    
    h_r
}

/// Get reflection parameters for an event if it is reflective
fn get_reflection_params(event: &NetworkEvent) -> Option<(f64, Length)> {
    match event {
        NetworkEvent::Connector { position_m, state, connector_type, .. } => {
            let return_loss = match state {
                ConnectorState::Mated { return_loss_db } => *return_loss_db,
                ConnectorState::Open => connector_type.open_return_loss_db(),
            };
            Some((return_loss, Length::new::<meter>(*position_m)))
        }
        NetworkEvent::End { position_m, end_type } => {
            let return_loss = end_type.return_loss_db();
            Some((return_loss, Length::new::<meter>(*position_m)))
        }
        NetworkEvent::Splice { splice_type, position_m, .. } => {
            match splice_type {
                crate::network::SpliceType::Mechanical { reflectance_db: Some(rl) } => {
                    Some((*rl as f64, Length::new::<meter>(*position_m)))
                }
                _ => None,
            }
        }
        NetworkEvent::Macrobend { .. } | NetworkEvent::Splitter { .. } => None,
    }
}

/// Build segment boundary list for a network
fn build_segment_boundaries(network: &NetworkDescription) -> Vec<(Length, Length, &FiberType)> {
    let mut boundaries = Vec::new();
    let mut current_start = Length::new::<meter>(0.0);
    
    for segment in &network.segments {
        let end = current_start + Length::new::<meter>(segment.length_m);
        boundaries.push((current_start, end, &segment.fiber_type));
        current_start = end;
    }
    
    boundaries
}

/// Calculate cumulative attenuation from start to a given point
fn calculate_cumulative_attenuation(
    z: Length,
    wavelength_nm: u16,
    boundaries: &[(Length, Length, &FiberType)],
) -> f64 {
    let mut total_attenuation_db = 0.0;
    
    for (start, end, fiber_type) in boundaries {
        if z <= *start {
            break;
        }
        
        let chars = fiber_type.characteristics(wavelength_nm);
        
        // Calculate length of this segment traversed
        let segment_length = if z >= *end {
            *end - *start
        } else {
            z - *start
        };
        
        total_attenuation_db += chars.attenuation_db_km * segment_length.get::<kilometer>();
    }
    
    total_attenuation_db
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{FiberSegment, FiberType, ConnectorState, ConnectorType, EndType, NetworkDescription};

    #[test]
    fn test_reflection_from_open_upc() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_event(NetworkEvent::Connector {
                position_m: 500.0,
                insertion_loss_db: 0.3,
                state: ConnectorState::Open,
                connector_type: ConnectorType::UPC,
            });
        
        let h_r = compute_reflection_impulse(
            &network, 
            1550, 
            Length::new::<meter>(1.0), 
            1000
        );
        
        // UPC open has ~-14 dB return loss (4% reflection)
        // At 500m with ~0.22 dB/km, two-way loss is ~0.22 dB
        // So reflection FRACTION should be roughly 4% * 0.95 = 0.038
        assert!(h_r[500].get::<ratio>() > 1e-3);
        assert!(h_r[500].get::<ratio>() < 1e-1);
        
        // Other positions should have no reflection
        assert_eq!(h_r[0].get::<ratio>(), 0.0);
    }

    #[test]
    fn test_reflection_from_apc_vs_upc() {
        let create_network = |connector_type| {
            NetworkDescription::new()
                .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
                .add_event(NetworkEvent::Connector {
                    position_m: 500.0,
                    insertion_loss_db: 0.3,
                    state: ConnectorState::Open,
                    connector_type,
                })
        };
        
        let h_r_upc = compute_reflection_impulse(
            &create_network(ConnectorType::UPC), 
            1550, 
            Length::new::<meter>(1.0), 
            1000
        );
        let h_r_apc = compute_reflection_impulse(
            &create_network(ConnectorType::APC), 
            1550, 
            Length::new::<meter>(1.0), 
            1000
        );
        
        // APC should have much lower reflection than UPC
        assert!(h_r_upc[500] > h_r_apc[500] * 1000.0);
    }

    #[test]
    fn test_reflection_decay_with_distance() {
        let create_network = |pos: f64| {
            NetworkDescription::new()
                .add_segment(FiberSegment::new(10000.0, FiberType::G652D))
                .add_event(NetworkEvent::End {
                    position_m: pos,
                    end_type: EndType::Cleaved,
                })
        };
        
        let h_r_near = compute_reflection_impulse(&create_network(100.0), 1550, Length::new::<meter>(1.0), 10000);
        let h_r_far = compute_reflection_impulse(&create_network(5000.0), 1550, Length::new::<meter>(1.0), 10000);
        
        assert!(h_r_near[100] > h_r_far[5000]);
    }

    #[test]
    fn test_multiple_reflections() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(2000.0, FiberType::G652D))
            .add_event(NetworkEvent::Connector {
                position_m: 500.0,
                insertion_loss_db: 0.3,
                state: ConnectorState::Mated { return_loss_db: -40.0 },
                connector_type: ConnectorType::UPC,
            })
            .add_event(NetworkEvent::Connector {
                position_m: 1500.0,
                insertion_loss_db: 0.3,
                state: ConnectorState::Open,
                connector_type: ConnectorType::UPC,
            });
        
        let h_r = compute_reflection_impulse(&network, 1550, Length::new::<meter>(1.0), 2000);
        
        assert!(h_r[500].get::<ratio>() > 0.0);
        assert!(h_r[1500].get::<ratio>() > 0.0);
        
        // Open connector > mated connector
        assert!(h_r[1500] > h_r[500] * 100.0);
    }

    #[test]
    fn test_end_reflection() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(100.0, FiberType::G652D))
            .add_event(NetworkEvent::End {
                position_m: 100.0,
                end_type: EndType::Cleaved,
            });
        
        let h_r = compute_reflection_impulse(
            &network, 
            1550, 
            Length::new::<meter>(1.0), 
            150
        );
        
        // Cleaved end ~0.04
        assert!(h_r[100].get::<ratio>() > 1e-3);
    }

    #[test]
    fn test_terminated_end_low_reflection() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(100.0, FiberType::G652D))
            .add_event(NetworkEvent::End {
                position_m: 100.0,
                end_type: EndType::Terminated,
            });
        
        let h_r = compute_reflection_impulse(
            &network, 
            1550, 
            Length::new::<meter>(1.0), 
            150
        );
        
        // Terminated end < 1e-5
        assert!(h_r[100].get::<ratio>() > 0.0);
        assert!(h_r[100].get::<ratio>() < 1e-5);
    }
}
