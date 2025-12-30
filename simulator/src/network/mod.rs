//! Network description module

use serde::{Deserialize, Serialize};
use crate::SimulatorError;

pub mod fiber_types;
pub mod event;
pub mod segment;

pub use fiber_types::{FiberType, FiberCharacteristics, CustomFiberParams};
pub use event::{NetworkEvent, EventPosition, SpliceType,
                ConnectorState, ConnectorType, EndType};
pub use segment::FiberSegment;

/// Complete description of a fiber network to simulate
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NetworkDescription {
    /// Ordered list of fiber segments making up the network
    pub segments: Vec<FiberSegment>,
    /// Events along the network (splices, connectors, etc.)
    pub events: Vec<NetworkEvent>,
    /// Optional description/name for the network
    pub name: Option<String>,
}

impl NetworkDescription {
    /// Create a new network description
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
            events: Vec::new(),
            name: None,
        }
    }

    /// Set the network name
    pub fn with_name(mut self, name: String) -> Self {
        self.name = Some(name);
        self
    }

    /// Add a fiber segment
    pub fn add_segment(mut self, segment: FiberSegment) -> Self {
        self.segments.push(segment);
        self
    }

    /// Add an event
    pub fn add_event(mut self, event: NetworkEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Get total network length in meters
    pub fn total_length(&self) -> f64 {
        self.segments.iter().map(|s| s.length_m).sum()
    }

    /// Validate the network description
    pub fn validate(&self) -> Result<(), SimulatorError> {
        // Check we have at least one segment
        if self.segments.is_empty() {
            return Err(SimulatorError::InvalidConfiguration(
                "Network must have at least one fiber segment".to_string()
            ));
        }

        // Check all segment lengths are positive
        for (i, segment) in self.segments.iter().enumerate() {
            if segment.length_m <= 0.0 {
                return Err(SimulatorError::InvalidConfiguration(
                    format!("Segment {} has non-positive length: {}", i, segment.length_m)
                ));
            }
        }

        let total_length = self.total_length();

        // Check all events are within the network length
        for event in &self.events {
            let pos = event.position_m();
            if pos < 0.0 || pos > total_length {
                return Err(SimulatorError::InvalidConfiguration(
                    format!("Event at position {} is outside network length {}", pos, total_length)
                ));
            }
        }

        // Check for duplicate event positions (within 1cm tolerance)
        let mut positions: Vec<f64> = self.events.iter()
            .map(|e| e.position_m())
            .collect();
        positions.sort_by(|a, b| a.partial_cmp(b).unwrap());

        for window in positions.windows(2) {
            if (window[1] - window[0]).abs() < 0.01 {
                return Err(SimulatorError::InvalidConfiguration(
                    format!("Duplicate events at position ~{:.2}m", window[0])
                ));
            }
        }

        Ok(())
    }

    /// Get events sorted by position
    pub fn events_sorted(&self) -> Vec<&NetworkEvent> {
        let mut events: Vec<&NetworkEvent> = self.events.iter().collect();
        events.sort_by(|a, b| {
            a.position_m()
                .partial_cmp(&b.position_m())
                .unwrap()
        });
        events
    }
}

impl Default for NetworkDescription {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_total_length() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_segment(FiberSegment::new(500.0, FiberType::G657A1));

        assert_eq!(network.total_length(), 1500.0);
    }

    #[test]
    fn test_network_validation_empty() {
        let network = NetworkDescription::new();
        assert!(network.validate().is_err());
    }

    #[test]
    fn test_network_validation_negative_length() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(-100.0, FiberType::G652D));

        assert!(network.validate().is_err());
    }

    #[test]
    fn test_network_validation_event_out_of_bounds() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_event(NetworkEvent::End {
                position_m: 2000.0,
                end_type: EndType::Cleaved,
            });

        assert!(network.validate().is_err());
    }

    #[test]
    fn test_network_validation_valid() {
        let network = NetworkDescription::new()
            .add_segment(FiberSegment::new(1000.0, FiberType::G652D))
            .add_event(NetworkEvent::End {
                position_m: 1000.0,
                end_type: EndType::Cleaved,
            });

        assert!(network.validate().is_ok());
    }
}
