//! Network event definitions
//!
//! This module defines all types of events that can occur in a fiber network.

use serde::{Deserialize, Serialize};

/// Position marker for events along the fiber
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct EventPosition {
    /// Distance from network start in meters
    pub distance_m: f64,
}

impl EventPosition {
    pub fn new(distance_m: f64) -> Self {
        Self { distance_m }
    }
}

/// All possible event types in a fiber network
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum NetworkEvent {
    Splice {
        position_m: f64,
        insertion_loss_db: f64,
        splice_type: SpliceType,
    },
    Connector {
        position_m: f64,
        insertion_loss_db: f64,
        state: ConnectorState,
        connector_type: ConnectorType,
    },
    Macrobend {
        position_m: f64,
        bend_radius_mm: f64,
        turns: f64,
        span_m: f64,
    },
    Splitter {
        position_m: f64,
        split_ratio: u32,
        excess_loss_db: f64,
    },
    End {
        position_m: f64,
        end_type: EndType,
    },
}

impl NetworkEvent {
    /// Get the position of this event
    pub fn position_m(&self) -> f64 {
        match self {
            NetworkEvent::Splice { position_m, .. } => *position_m,
            NetworkEvent::Connector { position_m, .. } => *position_m,
            NetworkEvent::Macrobend { position_m, .. } => *position_m,
            NetworkEvent::Splitter { position_m, .. } => *position_m,
            NetworkEvent::End { position_m, .. } => *position_m,
        }
    }

    /// Calculate total loss for splitter event
    pub fn splitter_total_loss_db(split_ratio: u32, excess_loss_db: f64) -> f64 {
        let ideal_loss = 10.0 * (split_ratio as f64).powi(2).log10();
        ideal_loss + excess_loss_db
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpliceType {
    /// Fusion splice - very low loss, no reflection
    Fusion,
    /// Mechanical splice - slightly higher loss, may have small reflection
    Mechanical { reflectance_db: Option<i32> },
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectorState {
    /// Mated connector pair with specified return loss
    Mated { return_loss_db: f64 },
    /// Unmated/open connector - high Fresnel reflection
    Open,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum ConnectorType {
    /// Ultra Physical Contact - ~4% Fresnel reflection when open
    UPC,
    /// Angled Physical Contact - ~-65dB reflection when open
    APC,
}

impl ConnectorType {
    /// Get typical return loss for this connector type when open
    pub fn open_return_loss_db(&self) -> f64 {
        match self {
            ConnectorType::UPC => -14.0, // ~4% Fresnel reflection
            ConnectorType::APC => -65.0, // Angle reduces reflection significantly
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EndType {
    /// Clean cleaved end - high Fresnel reflection (~-14dB)
    Cleaved,
    /// Broken/shattered end - lower, broader reflection
    Broken,
    /// Terminated with index-matching - low/no reflection
    Terminated,
    /// Connected to device with specified return loss
    Connected { return_loss_db: f64 },
}

impl EndType {
    /// Get return loss for this end type
    pub fn return_loss_db(&self) -> f64 {
        match self {
            EndType::Cleaved => -14.0,    // ~4% Fresnel reflection
            EndType::Broken => -20.0,     // Scattered reflection
            EndType::Terminated => -60.0, // Index-matched
            EndType::Connected { return_loss_db } => *return_loss_db,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_splitter_loss_calculation() {
        let ideal_loss = 10.0 * 32.0_f64.powi(2).log10();
        let total_loss = NetworkEvent::splitter_total_loss_db(32, 0.5);
        assert!((total_loss - (ideal_loss + 0.5)).abs() < 0.1);
    }

    #[test]
    fn test_connector_return_loss() {
        assert_eq!(ConnectorType::UPC.open_return_loss_db(), -14.0);
        assert_eq!(ConnectorType::APC.open_return_loss_db(), -65.0);
    }

    #[test]
    fn test_end_type_return_loss() {
        assert_eq!(EndType::Cleaved.return_loss_db(), -14.0);
        assert_eq!(EndType::Terminated.return_loss_db(), -60.0);
        assert_eq!(
            EndType::Connected { return_loss_db: -30.0 }.return_loss_db(),
            -30.0
        );
    }
}
