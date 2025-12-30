//! Cumulative loss function calculation
//!
//! This module computes the loss function f_l(z) which tracks the cumulative
//! insertion losses through the fiber network.

use crate::network::{NetworkDescription, NetworkEvent, FiberType};
use crate::units::{Length, Ratio, meter, kilometer, ratio};
use std::collections::BTreeMap;

/// Compute the cumulative loss function for a fiber network
/// 
/// Returns vector of loss multipliers (dimensionless ratios).
/// 1.0 = no loss, <1.0 = lossy.
pub fn compute_loss_function(
    network: &NetworkDescription,
    wavelength_nm: u16,
    delta_z: Length,
    num_points: usize,
) -> Vec<Ratio> {
    // Collect all loss events, sorted by position
    let mut loss_events: BTreeMap<usize, f64> = BTreeMap::new();
    
    // Build segment boundaries for macrobend fiber type lookup
    let segment_boundaries = build_segment_boundaries(network);
    
    for event in &network.events {
        if let Some((position, loss_db)) = get_loss_params(event, wavelength_nm, &segment_boundaries) {
            let index = (position.get::<meter>() / delta_z.get::<meter>()).round() as usize;
            if index < num_points {
                // Accumulate losses at the same position
                *loss_events.entry(index).or_insert(0.0) += loss_db;
            }
        }
    }
    
    // Build cumulative loss array
    let mut f_l = vec![Ratio::new::<ratio>(1.0); num_points];
    let mut cumulative_loss_db = 0.0;
    let mut last_event_index = 0;
    
    // Convert sorted events to vector for iteration
    let sorted_events: Vec<(usize, f64)> = loss_events.into_iter().collect();
    
    for (index, loss_db) in sorted_events {
        // Fill in the loss for all points up to this event
        let linear_loss = Ratio::new::<ratio>(10.0_f64.powf(-cumulative_loss_db / 10.0));
        for i in last_event_index..index {
            f_l[i] = linear_loss;
        }
        
        // Add this event's loss (doubled for round-trip)
        cumulative_loss_db += 2.0 * loss_db;
        last_event_index = index;
    }
    
    // Fill remaining points with final cumulative loss
    let final_loss = Ratio::new::<ratio>(10.0_f64.powf(-cumulative_loss_db / 10.0));
    for i in last_event_index..num_points {
        f_l[i] = final_loss;
    }
    
    f_l
}

/// Get loss parameters for an event
fn get_loss_params(
    event: &NetworkEvent,
    wavelength_nm: u16,
    segment_boundaries: &[(Length, Length, &FiberType)],
) -> Option<(Length, f64)> {
    match event {
        NetworkEvent::Splice { position_m, insertion_loss_db, .. } => {
            Some((Length::new::<meter>(*position_m), *insertion_loss_db))
        }
        NetworkEvent::Connector { position_m, insertion_loss_db, .. } => {
            Some((Length::new::<meter>(*position_m), *insertion_loss_db))
        }
        NetworkEvent::Macrobend { position_m, bend_radius_mm, turns, .. } => {
            // Get fiber type at this position for macrobend calculation
            let pos = Length::new::<meter>(*position_m);
            let fiber_type = find_fiber_type_at(pos, segment_boundaries)?;
            let loss_per_turn = fiber_type.macrobend_loss_per_turn(wavelength_nm, *bend_radius_mm);
            let total_loss = loss_per_turn * turns;
            Some((pos, total_loss))
        }
        NetworkEvent::Splitter { position_m, split_ratio, excess_loss_db } => {
            // Total splitter loss: ideal splitting loss + excess loss
            let ideal_loss = 10.0 * (*split_ratio as f64).log10();
            let total_loss = ideal_loss + excess_loss_db;
            Some((Length::new::<meter>(*position_m), total_loss))
        }
        NetworkEvent::End { .. } => {
            // End events don't cause insertion loss (only reflection)
            None
        }
    }
}

/// Find the fiber type at a given position
fn find_fiber_type_at<'a>(
    z: Length,
    boundaries: &'a [(Length, Length, &'a FiberType)],
) -> Option<&'a FiberType> {
    for (start, end, fiber_type) in boundaries {
        if z >= *start && z < *end {
            return Some(fiber_type);
        }
    }
    // Check if at the end
    if let Some((_, end, fiber_type)) = boundaries.last() {
        if (z - *end).abs() < Length::new::<meter>(1e-9) {
            return Some(fiber_type);
        }
    }
    None
}

/// Build segment boundary list
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::{FiberSegment, FiberType, NetworkDescription, SpliceType, ConnectorState, ConnectorType};

    #[test]
    fn test_no_events_no_loss() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D));
        
        let f_l = compute_loss_function(&network, 1550, Length::new::<meter>(1.0), 1000);
        
        // No events means no discrete losses
        for &loss in &f_l {
            assert_eq!(loss.get::<ratio>(), 1.0);
        }
    }

    #[test]
    fn test_single_splice_loss() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_event(NetworkEvent::Splice {
                position_m: 500.0,
                insertion_loss_db: 0.1,
                splice_type: SpliceType::Fusion,
            });
        
        let f_l = compute_loss_function(&network, 1550, Length::new::<meter>(1.0), 1000);
        
        // Before splice: no loss
        assert_eq!(f_l[0].get::<ratio>(), 1.0);
        assert_eq!(f_l[499].get::<ratio>(), 1.0);
        
        // After splice: 0.2 dB round-trip loss
        let expected_loss = 10.0_f64.powf(-0.2 / 10.0);
        assert!((f_l[501].get::<ratio>() - expected_loss).abs() < 0.001);
        assert!((f_l[999].get::<ratio>() - expected_loss).abs() < 0.001);
    }

    #[test]
    fn test_cumulative_losses() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_event(NetworkEvent::Splice {
                position_m: 300.0,
                insertion_loss_db: 0.1,
                splice_type: SpliceType::Fusion,
            })
            .add_event(NetworkEvent::Splice {
                position_m: 600.0,
                insertion_loss_db: 0.15,
                splice_type: SpliceType::Fusion,
            });
        
        let f_l = compute_loss_function(&network, 1550, Length::new::<meter>(1.0), 1000);
        
        // Before first splice
        assert_eq!(f_l[0].get::<ratio>(), 1.0);
        
        // After first splice: 0.2 dB round-trip
        let loss_1 = 10.0_f64.powf(-0.2 / 10.0);
        assert!((f_l[400].get::<ratio>() - loss_1).abs() < 0.001);
        
        // After second splice: 0.2 + 0.3 = 0.5 dB round-trip
        let loss_2 = 10.0_f64.powf(-0.5 / 10.0);
        assert!((f_l[700].get::<ratio>() - loss_2).abs() < 0.001);
    }

    #[test]
    fn test_macrobend_wavelength_dependence() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_event(NetworkEvent::Macrobend {
                position_m: 500.0,
                bend_radius_mm: 10.0,
                turns: 2.0,
                span_m: 0.5,
            });
        
        let f_l_1310 = compute_loss_function(&network, 1310, Length::new::<meter>(1.0), 1000);
        let f_l_1550 = compute_loss_function(&network, 1550, Length::new::<meter>(1.0), 1000);
        let f_l_1625 = compute_loss_function(&network, 1625, Length::new::<meter>(1.0), 1000);
        
        assert!(f_l_1310[600] > f_l_1550[600]);
        assert!(f_l_1550[600] > f_l_1625[600]);
    }

    #[test]
    fn test_connector_loss() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_event(NetworkEvent::Connector {
                position_m: 500.0,
                insertion_loss_db: 0.5,
                state: ConnectorState::Mated { return_loss_db: -40.0 },
                connector_type: ConnectorType::UPC,
            });
        
        let f_l = compute_loss_function(&network, 1550, Length::new::<meter>(1.0), 1000);
        
        // After connector: 1.0 dB round-trip loss
        let expected_loss = 10.0_f64.powf(-1.0 / 10.0);
        assert!((f_l[600].get::<ratio>() - expected_loss).abs() < 0.001);
    }

    #[test]
    fn test_splitter_loss() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_event(NetworkEvent::Splitter {
                position_m: 500.0,
                split_ratio: 8, // 1:8 splitter
                excess_loss_db: 0.5,
            });
        
        let f_l = compute_loss_function(&network, 1550, Length::new::<meter>(1.0), 1000);
        
        // 1:8 splitter single-pass loss = 10×log10(8) ≈ 9.03 dB
        // Plus 0.5 dB excess = ~9.53 dB single-pass
        // Round-trip = ~19.06 dB
        let ideal_loss = 10.0 * 8.0_f64.log10() + 0.5;
        let expected_loss = 10.0_f64.powf(-2.0 * ideal_loss / 10.0);
        
        assert!((f_l[600].get::<ratio>() - expected_loss).abs() < 0.001);
    }
}
