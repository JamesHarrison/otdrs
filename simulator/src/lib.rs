//! OTDR Simulator Library
//!
//! This crate provides functionality to simulate OTDR (Optical Time-Domain Reflectometry)
//! traces using a Linear Time-Invariant (LTI) system approach based on the mathematical
//! formalism described by Feigel et al. (2014).
//!
//! # Quick Start
//!
//! ```rust,ignore
//! use otdrs_simulator::{run_simulation, SimulationConfig};
//! use otdrs_simulator::config::yaml::from_yaml_file;
//!
//! let config = from_yaml_file("simulation.yaml")?;
//! let result = run_simulation(&config)?;
//! result.write_outputs(output_dir, "trace", &config)?;
//! ```

pub mod network;
pub mod acquisition;
pub mod physics;
pub mod postprocess;
pub mod output;
pub mod config;
pub mod runner;
pub mod units;

pub use network::NetworkDescription;
pub use acquisition::AcquisitionConfig;
pub use config::SimulationConfig;
pub use output::SimulationResult;
pub use runner::{run_simulation, run_and_save};

use thiserror::Error;

/// Error types for the simulator
#[derive(Error, Debug)]
pub enum SimulatorError {
    #[error("Invalid configuration: {0}")]
    InvalidConfiguration(String),
    
    #[error("Physics calculation error: {0}")]
    PhysicsError(String),
    
    #[error("I/O error: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("Serialization error: {0}")]
    SerializationError(String),
    
    #[error("SOR file generation error: {0}")]
    SorError(String),
    
    #[error("Output error: {0}")]
    OutputError(String),
}

pub type Result<T> = std::result::Result<T, SimulatorError>;

