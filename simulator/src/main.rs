//! OTDR Simulator CLI Binary
//!
//! This binary provides a command-line interface for running OTDR simulations.
//!
//! # Usage
//!
//! ```bash
//! otdrs-sim config.yaml               # Run simulation from config file
//! otdrs-sim config.yaml -o output/    # Specify output directory
//! otdrs-sim config.yaml --pdf-report  # Generate PDF visualization report
//! otdrs-sim config.yaml --dry-run     # Validate config without running
//! ```

use std::path::PathBuf;
use clap::Parser;
use otdrs_simulator::{run_simulation, SimulationConfig};

/// OTDR Trace Simulator
#[derive(Parser, Debug)]
#[command(name = "otdrs-sim")]
#[command(author, version, about = "Generate simulated OTDR traces", long_about = None)]
struct Args {
    /// Path to YAML configuration file
    #[arg(value_name = "CONFIG")]
    config_file: PathBuf,
    
    /// Output directory (overrides config file setting)
    #[arg(short, long)]
    output_dir: Option<PathBuf>,
    
    /// Validate configuration without running simulation
    #[arg(long)]
    dry_run: bool,
    
    /// Generate PDF report with trace visualization, event table, and parameters
    #[arg(long)]
    pdf_report: bool,
    
    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();
    
    if let Err(e) = run(args) {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

fn run(args: Args) -> Result<(), Box<dyn std::error::Error>> {
    if args.verbose {
        println!("Loading configuration from: {:?}", args.config_file);
    }
    
    // Load configuration
    let mut config: SimulationConfig = SimulationConfig::from_yaml_file(&args.config_file)?;
    
    // Override output directory if specified
    if let Some(output_dir) = args.output_dir {
        config.output_dir = output_dir.to_string_lossy().to_string();
    }
    
    if args.verbose {
        println!("Configuration loaded:");
        println!("  Network: {} segments, {} events",
            config.network.segments.len(),
            config.network.events.len()
        );
        println!("  Total length: {:.1} m", config.network.total_length());
        println!("  Wavelengths: {:?}",
            config.acquisition.wavelengths.iter()
                .map(|w| format!("{}nm", w.wavelength_nm))
                .collect::<Vec<_>>()
        );
        println!("  Range: {:.0} m", config.acquisition.range_m);
        println!("  Output: {}/{}", config.output_dir, config.output_basename);
    }
    
    // Validate configuration
    config.validate()?;
    
    if args.verbose {
        println!("Configuration validated successfully.");
    }
    
    // Dry run - just validate and exit
    if args.dry_run {
        println!("Dry run complete - configuration is valid.");
        return Ok(());
    }
    
    // Run simulation
    if args.verbose {
        println!("Running simulation...");
    }
    
    let result = run_simulation(&config)?;
    
    if args.verbose {
        println!("Simulation complete:");
        println!("  Generated {} traces", result.traces.len());
        for trace in &result.traces {
            println!("    {}nm: {} points, {:.2}m resolution",
                trace.wavelength_nm,
                trace.num_points,
                trace.delta_z_m
            );
        }
    }
    
    // Write output files
    let output_dir = std::path::Path::new(&config.output_dir);
    let output_files = result.write_outputs_with_pdf(
        output_dir,
        &config.output_basename,
        &config,
        args.pdf_report,
    )?;
    
    println!("Generated {} files:", output_files.len());
    for file in &output_files {
        println!("  {}", file.display());
    }
    
    Ok(())
}
