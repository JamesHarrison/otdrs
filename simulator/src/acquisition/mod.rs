//! Acquisition parameters and configuration

pub mod params;
pub mod pulse;
pub mod hardware;

pub use params::{AcquisitionConfig, WavelengthAcquisition, PulseShape, LaunchCable, LaunchConnectorType};
pub use hardware::HardwareConfig;
