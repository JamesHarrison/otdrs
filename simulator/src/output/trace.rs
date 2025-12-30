//! Simulated OTDR trace structure

use serde::{Deserialize, Serialize};

/// A single simulated OTDR trace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulatedTrace {
    /// Wavelength this trace was acquired at
    pub wavelength_nm: u16,
    /// Pulse width used
    pub pulse_width_ns: f64,
    /// Raw data points (power in Watts)
    pub data_points: Vec<f64>,
    /// Spatial sampling interval in meters
    pub delta_z_m: f64,
    /// Number of data points
    pub num_points: usize,
}

impl SimulatedTrace {
    /// Create a new simulated trace
    pub fn new(
        wavelength_nm: u16,
        pulse_width_ns: f64,
        data_points: Vec<f64>,
        delta_z_m: f64,
    ) -> Self {
        let num_points = data_points.len();
        Self {
            wavelength_nm,
            pulse_width_ns,
            data_points,
            delta_z_m,
            num_points,
        }
    }

    /// Get total trace length in meters
    pub fn total_length_m(&self) -> f64 {
        self.delta_z_m * self.num_points as f64
    }

    /// Convert power at index to dBm
    pub fn power_dbm(&self, index: usize) -> f64 {
        if index >= self.num_points {
            return f64::NEG_INFINITY;
        }

        let power_w = self.data_points[index];
        if power_w <= 0.0 {
            return f64::NEG_INFINITY;
        }

        // Convert to dBm: P(dBm) = 10*log10(P(mW)) = 10*log10(P(W)*1000)
        // For OTDR traces, use 5*log10 to account for two-way propagation
        5.0 * (power_w * 1000.0).log10()
    }
}
