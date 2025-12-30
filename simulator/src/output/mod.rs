//! Output module for simulation results
//!
//! This module provides functionality for:
//! - SOR file generation (via otdrs crate)
//! - JSON ground truth sidecar files
//! - PDF report generation
//! - Trace data structures

pub mod trace;
pub mod sor;
pub mod sidecar;
pub mod pdf;

pub use trace::SimulatedTrace;
pub use sor::{write_sor_file, create_sor_file};
pub use sidecar::{GroundTruth, GroundTruthEvent, GroundTruthEventParams, GroundTruthSegment, generate_ground_truth, write_sidecar_file};
pub use pdf::{write_pdf_report, PdfReportConfig};

use std::path::Path;
use uuid::Uuid;

use crate::config::SimulationConfig;
use crate::physics::PhysicsResult;
use crate::SimulatorError;

/// Complete simulation result for a multi-wavelength run
#[derive(Debug, Clone)]
pub struct SimulationResult {
    /// Unique identifier for this simulation run
    pub simulation_id: Uuid,
    /// Simulated traces, one per wavelength
    pub traces: Vec<SimulatedTrace>,
    /// Ground truth information
    pub ground_truth: GroundTruth,
    /// Post-processed traces in watts
    pub processed_traces: Vec<Vec<f64>>,
    /// Physics results
    pub physics_results: Vec<PhysicsResult>,
}

impl SimulationResult {
    /// Write all output files for this simulation
    /// 
    /// Generates:
    /// - One SOR file per wavelength: {basename}_{wavelength}nm.sor
    /// - One JSON sidecar: {basename}_ground_truth.json
    pub fn write_outputs(
        &self,
        output_dir: &Path,
        basename: &str,
        config: &SimulationConfig,
    ) -> Result<Vec<std::path::PathBuf>, SimulatorError> {
        std::fs::create_dir_all(output_dir)
            .map_err(|e| SimulatorError::OutputError(format!("Failed to create output directory: {}", e)))?;
        
        let mut output_files = Vec::new();
        
        // Write SOR files for each wavelength
        for (physics, processed) in self.physics_results.iter()
            .zip(self.processed_traces.iter())
        {
            let wavelength = physics.wavelength_nm;
            let sor_filename = format!("{}_{:04}nm.sor", basename, wavelength);
            let sor_path = output_dir.join(&sor_filename);
            
            write_sor_file(physics, processed, config, &sor_path)?;
            output_files.push(sor_path);
        }
        
        // Write ground truth sidecar
        let sidecar_filename = format!("{}_ground_truth.json", basename);
        let sidecar_path = output_dir.join(&sidecar_filename);
        write_sidecar_file(&self.ground_truth, &sidecar_path)?;
        output_files.push(sidecar_path);
        
        Ok(output_files)
    }
    
    /// Write all output files including optional PDF report
    /// 
    /// Generates:
    /// - One SOR file per wavelength: {basename}_{wavelength}nm.sor
    /// - One JSON sidecar: {basename}_ground_truth.json
    /// - Optionally, one PDF report: {basename}_report.pdf
    pub fn write_outputs_with_pdf(
        &self,
        output_dir: &Path,
        basename: &str,
        config: &SimulationConfig,
        generate_pdf: bool,
    ) -> Result<Vec<std::path::PathBuf>, SimulatorError> {
        let mut output_files = self.write_outputs(output_dir, basename, config)?;
        
        if generate_pdf {
            let pdf_filename = format!("{}_report.pdf", basename);
            let pdf_path = output_dir.join(&pdf_filename);
            write_pdf_report(self, config, &pdf_path, &PdfReportConfig::default())?;
            output_files.push(pdf_path);
        }
        
        Ok(output_files)
    }
}
