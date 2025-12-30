//! Integration tests for the simulator

use otdrs_simulator::{SimulationConfig, run_simulation};
use tempfile::TempDir;

#[test]
fn test_load_example_ftth_config() {
    let config = SimulationConfig::from_yaml_file("tests/fixtures/example_ftth.yaml")
        .expect("Failed to load example config");
    
    // Verify basic properties
    assert_eq!(config.seed, Some(42));
    assert_eq!(config.output_basename, "ftth_drop_example");
    
    // Verify network
    assert_eq!(config.network.segments.len(), 3);
    assert_eq!(config.network.events.len(), 5); // CO connector, splice, macrobend, splice, customer connector
    assert_eq!(config.network.total_length(), 2200.0);
    
    // Verify multi-wavelength acquisition
    assert_eq!(config.acquisition.wavelengths.len(), 3);
    assert_eq!(config.acquisition.wavelengths[0].wavelength_nm, 1310);
    assert_eq!(config.acquisition.wavelengths[1].wavelength_nm, 1550);
    assert_eq!(config.acquisition.wavelengths[2].wavelength_nm, 1625);
    
    // Should validate successfully
    config.validate().expect("Config validation failed");
}

#[test]
fn test_end_to_end_simulation_and_sor_roundtrip() {
    // Load config
    let config = SimulationConfig::from_yaml_file("tests/fixtures/example_ftth.yaml")
        .expect("Failed to load config");
    
    // Run simulation
    let result = run_simulation(&config).expect("Simulation failed");
    
    // Write outputs to temp directory
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    let output_files = result.write_outputs(temp_dir.path(), "test_sim", &config)
        .expect("Failed to write outputs");
    
    // Should have 4 files: 3 SOR + 1 JSON
    assert_eq!(output_files.len(), 4);
    
    // Verify each SOR file can be parsed with otdrs
    for file in output_files.iter().filter(|f| f.extension().map_or(false, |e| e == "sor")) {
        let data = std::fs::read(file).expect("Failed to read SOR file");
        let (_, parsed) = otdrs::parser::parse_file(&data).expect("Failed to parse SOR file");
        
        // Verify parsed data matches what we generated
        let gen_params = parsed.general_parameters.expect("Missing general parameters");
        let fixed_params = parsed.fixed_parameters.expect("Missing fixed parameters");
        let data_points = parsed.data_points.expect("Missing data points");
        
        // Check wavelength matches
        assert!(gen_params.nominal_wavelength > 0);
        assert!(fixed_params.actual_wavelength > 0);
        
        // Check data points exist
        assert!(data_points.number_of_data_points > 10000); // Should have many points
        
        // Verify at least one scale factor with data
        assert!(!data_points.scale_factors.is_empty());
        assert!(!data_points.scale_factors[0].data.is_empty());
    }
    
    // Verify JSON sidecar
    let json_file = output_files.iter()
        .find(|f| f.extension().map_or(false, |e| e == "json"))
        .expect("Missing JSON sidecar");
    
    let json_content = std::fs::read_to_string(json_file).expect("Failed to read JSON");
    let ground_truth: serde_json::Value = serde_json::from_str(&json_content)
        .expect("Failed to parse JSON");
    
    // Verify ground truth contains events and segments
    assert!(ground_truth["events"].is_array());
    assert!(ground_truth["segments"].is_array());
    assert!(ground_truth["wavelengths"].is_array());
    
    // Verify wavelength-dependent macrobend loss
    let events = ground_truth["events"].as_array().unwrap();
    let macrobend = events.iter()
        .find(|e| e["event_type"] == "macrobend")
        .expect("Missing macrobend event");
    
    let loss_by_wavelength = &macrobend["parameters"]["loss_by_wavelength"];
    assert!(loss_by_wavelength["1310"].as_f64().is_some());
    assert!(loss_by_wavelength["1550"].as_f64().is_some());
    assert!(loss_by_wavelength["1625"].as_f64().is_some());
    
    // 1625nm should have higher macrobend loss than 1310nm
    let loss_1310 = loss_by_wavelength["1310"].as_f64().unwrap();
    let loss_1625 = loss_by_wavelength["1625"].as_f64().unwrap();
    assert!(loss_1625 > loss_1310, "Macrobend loss should be higher at 1625nm");
}

