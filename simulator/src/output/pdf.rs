//! PDF Report Generation
//!
//! This module generates PDF reports containing OTDR trace visualizations,
//! event tables, offset information, and configuration summaries.
//!
//! The implementation uses:
//! - `plotters` for chart generation (rendered to BMP)
//! - `printpdf` for PDF assembly with embedded images and text

use std::io::Cursor;
use std::path::Path;

use image::codecs::bmp::BmpDecoder;
use plotters::prelude::*;
use plotters::backend::BitMapBackend;
use printpdf::{
    BuiltinFont, ImageTransform, Mm, PdfDocument, PdfLayerReference,
    Image as PdfImage,
};

use crate::acquisition::pulse::generate_pulse;
use crate::config::SimulationConfig;
use crate::output::{GroundTruth, GroundTruthEvent, SimulationResult};
use crate::physics::PhysicsResult;
use crate::SimulatorError;
use crate::units::{Length, Power, meter, watt};

/// Configuration for PDF report generation
#[derive(Debug, Clone)]
pub struct PdfReportConfig {
    /// Chart width in pixels
    pub chart_width: u32,
    /// Chart height in pixels  
    pub chart_height: u32,
    /// Small chart width for event zooms
    pub small_chart_width: u32,
    /// Small chart height for event zooms
    pub small_chart_height: u32,
    /// Landscape page width (A4 landscape)
    pub landscape_width_mm: f32,
    /// Landscape page height (A4 landscape)
    pub landscape_height_mm: f32,
    /// Portrait page width (A4 portrait)
    pub portrait_width_mm: f32,
    /// Portrait page height (A4 portrait)
    pub portrait_height_mm: f32,
    /// Margins in mm
    pub margin_mm: f32,
    /// Zoom window size in meters (on each side of event)
    pub event_zoom_range_m: f64,
}

impl Default for PdfReportConfig {
    fn default() -> Self {
        Self {
            chart_width: 1800,
            chart_height: 800,
            small_chart_width: 800,
            small_chart_height: 400,
            landscape_width_mm: 297.0,  // A4 landscape
            landscape_height_mm: 210.0, // A4 landscape
            portrait_width_mm: 210.0,   // A4 portrait
            portrait_height_mm: 297.0,  // A4 portrait
            margin_mm: 15.0,
            event_zoom_range_m: 50.0,   // 50m on each side of event
        }
    }
}

/// Helper to convert trace from watts to dBm
fn trace_watts_to_dbm(trace: &[f64]) -> Vec<f64> {
    trace.iter()
        .map(|&power| {
            if power <= 0.0 {
                f64::NEG_INFINITY
            } else {
                10.0 * (power * 1000.0).log10() // Convert to mW and then dBm
            }
        })
        .collect()
}

/// Colors for different wavelengths
fn wavelength_color(wavelength_nm: u16) -> RGBColor {
    match wavelength_nm {
        1310 => RGBColor(0, 100, 200),   // Blue
        1550 => RGBColor(0, 150, 50),    // Green
        1625 => RGBColor(200, 50, 50),   // Red
        1490 => RGBColor(150, 0, 150),   // Purple
        _ => RGBColor(100, 100, 100),    // Gray for unknown
    }
}

/// Encode image buffer to BMP bytes
fn encode_to_bmp(buffer: Vec<u8>, width: u32, height: u32) -> Result<Vec<u8>, SimulatorError> {
    let img = image::RgbImage::from_raw(width, height, buffer)
        .ok_or_else(|| SimulatorError::OutputError("Failed to create image from buffer".to_string()))?;
    
    let mut bmp_bytes = Vec::new();
    {
        let mut cursor = Cursor::new(&mut bmp_bytes);
        let mut encoder = image::codecs::bmp::BmpEncoder::new(&mut cursor);
        encoder.encode(
            &img,
            width,
            height,
            image::ColorType::Rgb8,
        ).map_err(|e| SimulatorError::OutputError(format!("BMP encode error: {}", e)))?;
    }
    
    Ok(bmp_bytes)
}

/// Add image to PDF layer
fn add_image_to_layer(
    layer: &PdfLayerReference,
    bmp_bytes: &[u8],
    x_mm: f32,
    y_mm: f32,
    width_mm: f32,
    pixel_width: u32,
) -> Result<(), SimulatorError> {
    let cursor = Cursor::new(bmp_bytes);
    let decoder = BmpDecoder::new(cursor)
        .map_err(|e| SimulatorError::OutputError(format!("BMP decode error: {}", e)))?;
    let pdf_image = PdfImage::try_from(decoder)
        .map_err(|e| SimulatorError::OutputError(format!("Image conversion error: {}", e)))?;
    
    let dpi = (pixel_width as f32) / (width_mm / 25.4);
    
    let transform = ImageTransform {
        translate_x: Some(Mm(x_mm)),
        translate_y: Some(Mm(y_mm)),
        dpi: Some(dpi),
        ..Default::default()
    };
    pdf_image.add_to_layer(layer.clone(), transform);
    
    Ok(())
}

/// Generate an OTDR trace chart as BMP bytes
pub fn generate_trace_chart(
    physics_results: &[PhysicsResult],
    processed_traces: &[Vec<f64>],
    ground_truth: &GroundTruth,
    config: &PdfReportConfig,
) -> Result<Vec<u8>, SimulatorError> {
    let mut buffer = vec![0u8; (config.chart_width * config.chart_height * 3) as usize];
    
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (config.chart_width, config.chart_height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| SimulatorError::OutputError(format!("Chart fill error: {}", e)))?;
        
        // Find data bounds
        let (x_min, x_max, y_min, y_max) = calculate_chart_bounds(physics_results, processed_traces);
        
        // Build chart
        let mut chart = ChartBuilder::on(&root)
            .caption("OTDR Trace - Full View", ("sans-serif", 28))
            .margin(15)
            .x_label_area_size(45)
            .y_label_area_size(55)
            .build_cartesian_2d(x_min..x_max, y_min..y_max)
            .map_err(|e| SimulatorError::OutputError(format!("Chart build error: {}", e)))?;
        
        // Configure mesh
        chart
            .configure_mesh()
            .x_desc("Distance (m)")
            .y_desc("Power (dB)")
            .x_label_formatter(&|x| {
                if *x >= 1000.0 {
                    format!("{:.1}km", x / 1000.0)
                } else {
                    format!("{:.0}m", x)
                }
            })
            .y_label_formatter(&|y| format!("{:.1}", y))
            .draw()
            .map_err(|e| SimulatorError::OutputError(format!("Mesh draw error: {}", e)))?;
        
        // Draw offset regions
        draw_offset_regions(&mut chart, ground_truth, y_min, y_max)?;
        
        // Draw event markers
        draw_event_markers(&mut chart, ground_truth, y_min, y_max)?;
        
        // Draw each wavelength trace
        for (physics, processed) in physics_results.iter().zip(processed_traces.iter()) {
            draw_trace_on_chart(&mut chart, physics, processed)?;
        }
        
        // Draw legend
        chart
            .configure_series_labels()
            .background_style(WHITE.mix(0.8))
            .border_style(BLACK)
            .position(SeriesLabelPosition::UpperRight)
            .draw()
            .map_err(|e| SimulatorError::OutputError(format!("Legend draw error: {}", e)))?;
        
        root.present().map_err(|e| SimulatorError::OutputError(format!("Present error: {}", e)))?;
    }
    
    encode_to_bmp(buffer, config.chart_width, config.chart_height)
}

/// Draw a trace on a chart 
fn draw_trace_on_chart<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<plotters::coord::types::RangedCoordf64, plotters::coord::types::RangedCoordf64>>,
    physics: &PhysicsResult,
    processed: &[f64],
) -> Result<(), SimulatorError>
where
    DB::ErrorType: std::fmt::Display,
{
    let trace_dbm = trace_watts_to_dbm(processed);
    let color = wavelength_color(physics.wavelength_nm);
    let delta_z_m = physics.delta_z.get::<meter>();
    
    let data_points: Vec<(f64, f64)> = trace_dbm
        .iter()
        .enumerate()
        .map(|(i, &db)| {
            let distance = i as f64 * delta_z_m;
            (distance, db)
        })
        .filter(|(_, db)| db.is_finite())
        .collect();
    
    // Downsample for performance if too many points
    let step = (data_points.len() / 5000).max(1);
    let downsampled: Vec<(f64, f64)> = data_points
        .iter()
        .step_by(step)
        .cloned()
        .collect();
    
    chart
        .draw_series(LineSeries::new(downsampled, color.stroke_width(2)))
        .map_err(|e| SimulatorError::OutputError(format!("Series draw error: {}", e)))?
        .label(format!("{}nm", physics.wavelength_nm))
        .legend(move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], color.stroke_width(2)));
    
    Ok(())
}

/// Generate pulse shape chart
fn generate_pulse_chart(
    config: &SimulationConfig,
    pdf_config: &PdfReportConfig,
) -> Result<Vec<u8>, SimulatorError> {
    let mut buffer = vec![0u8; (pdf_config.small_chart_width * pdf_config.small_chart_height * 3) as usize];
    
    // Generate pulse based on config
    let pulse_width_val = 1.0; // Normalized spatial pulse width
    let delta_z_val = 0.01;    // Fine sampling
    let peak_power_val = 1.0;
    
    let pulse_width = Length::new::<meter>(pulse_width_val);
    let delta_z = Length::new::<meter>(delta_z_val);
    let peak_power = Power::new::<watt>(peak_power_val);
    
    let pulse = generate_pulse(
        config.acquisition.pulse_shape.clone(),
        pulse_width,
        delta_z,
        peak_power,
    );
    
    // Convert to primitives for plotting
    let pulse_values: Vec<f64> = pulse.iter().map(|p| p.get::<watt>()).collect();
    
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (pdf_config.small_chart_width, pdf_config.small_chart_height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| SimulatorError::OutputError(format!("Chart fill error: {}", e)))?;
        
        let x_max = pulse_values.len() as f64 * delta_z_val;
        let y_max = pulse_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * 1.1;
        
        let mut chart = ChartBuilder::on(&root)
            .caption(format!("Pulse Shape: {:?}", config.acquisition.pulse_shape), ("sans-serif", 20))
            .margin(10)
            .x_label_area_size(35)
            .y_label_area_size(45)
            .build_cartesian_2d(0.0..x_max, 0.0..y_max)
            .map_err(|e| SimulatorError::OutputError(format!("Chart build error: {}", e)))?;
        
        chart
            .configure_mesh()
            .x_desc("Position (m)")
            .y_desc("Power (normalized)")
            .draw()
            .map_err(|e| SimulatorError::OutputError(format!("Mesh draw error: {}", e)))?;
        
        let data_points: Vec<(f64, f64)> = pulse_values
            .iter()
            .enumerate()
            .map(|(i, &p)| (i as f64 * delta_z_val, p))
            .collect();
        
        chart
            .draw_series(LineSeries::new(data_points, BLUE.stroke_width(2)))
            .map_err(|e| SimulatorError::OutputError(format!("Series draw error: {}", e)))?;
        
        root.present().map_err(|e| SimulatorError::OutputError(format!("Present error: {}", e)))?;
    }
    
    encode_to_bmp(buffer, pdf_config.small_chart_width, pdf_config.small_chart_height)
}

/// Generate a zoomed event chart
fn generate_event_zoom_chart(
    event: &GroundTruthEvent,
    ground_truth: &GroundTruth,
    physics_results: &[PhysicsResult],
    processed_traces: &[Vec<f64>],
    pdf_config: &PdfReportConfig,
) -> Result<Vec<u8>, SimulatorError> {
    let mut buffer = vec![0u8; (pdf_config.small_chart_width * pdf_config.small_chart_height * 3) as usize];
    
    // Calculate total offset to convert FUT-relative positions to absolute trace positions
    let total_offset = calculate_total_offset(ground_truth);
    
    // Event position in absolute trace coordinates
    let event_pos_absolute = event.position_m + total_offset;
    
    // Center the zoom window on the event in absolute coordinates
    // Clamp x_min to 0 (can't go before front panel)
    let half_range = pdf_config.event_zoom_range_m;
    let x_min = (event_pos_absolute - half_range).max(0.0);
    let x_max = event_pos_absolute + half_range;
    
    {
        let root = BitMapBackend::with_buffer(&mut buffer, (pdf_config.small_chart_width, pdf_config.small_chart_height))
            .into_drawing_area();
        root.fill(&WHITE).map_err(|e| SimulatorError::OutputError(format!("Chart fill error: {}", e)))?;
        
        // Find y bounds for this region
        let (y_min, y_max) = calculate_y_bounds_for_range(physics_results, processed_traces, x_min, x_max);
        
        // Title shows FUT-relative position (user's perspective)
        let title = format!("{} @ {:.1}m (FUT)", event.event_type, event.position_m);
        let mut chart = ChartBuilder::on(&root)
            .caption(&title, ("sans-serif", 18))
            .margin(10)
            .x_label_area_size(35)
            .y_label_area_size(50)
            .build_cartesian_2d(x_min..x_max, y_min..y_max)
            .map_err(|e| SimulatorError::OutputError(format!("Chart build error: {}", e)))?;
        
        chart
            .configure_mesh()
            .x_desc("Distance (m)")
            .y_desc("Power (dB)")
            .x_label_formatter(&|x| format!("{:.1}", x))
            .y_label_formatter(&|y| format!("{:.1}", y))
            .draw()
            .map_err(|e| SimulatorError::OutputError(format!("Mesh draw error: {}", e)))?;
        
        // Draw event marker
        let color = match event.event_type.as_str() {
            "splice" => RGBAColor(255, 165, 0, 0.5),
            "connector" => RGBAColor(255, 0, 0, 0.5),
            "splitter" => RGBAColor(128, 0, 128, 0.5),
            "macrobend" => RGBAColor(255, 255, 0, 0.5),
            "end" => RGBAColor(0, 0, 0, 0.5),
            _ => RGBAColor(128, 128, 128, 0.5),
        };
        
        // Draw event marker at absolute position
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(event_pos_absolute, y_min), (event_pos_absolute, y_max)],
            color.stroke_width(2),
        )))
        .map_err(|e| SimulatorError::OutputError(format!("Event marker error: {}", e)))?;
        
        // Draw traces in this region
        for (physics, processed) in physics_results.iter().zip(processed_traces.iter()) {
            let trace_dbm = trace_watts_to_dbm(processed);
            let _color = wavelength_color(physics.wavelength_nm);
            
            let delta_z_m = physics.delta_z.get::<meter>();
            
            // Get only data points in range
            let data_points: Vec<(f64, f64)> = trace_dbm
                .iter()
                .enumerate()
                .map(|(i, &db)| {
                    let distance = i as f64 * delta_z_m;
                    (distance, db)
                })
                .filter(|(d, db)| db.is_finite() && *d >= x_min && *d <= x_max)
                .collect();
            
            let step = (data_points.len() / 1000).max(1);
            let downsampled: Vec<(f64, f64)> = data_points
                .iter()
                .step_by(step)
                .cloned()
                .collect();
            
            let line_color = wavelength_color(physics.wavelength_nm);
            chart
                .draw_series(LineSeries::new(downsampled, line_color.stroke_width(2)))
                .map_err(|e| SimulatorError::OutputError(format!("Series draw error: {}", e)))?;
        }
        
        root.present().map_err(|e| SimulatorError::OutputError(format!("Present error: {}", e)))?;
    }
    
    encode_to_bmp(buffer, pdf_config.small_chart_width, pdf_config.small_chart_height)
}

/// Calculate y bounds for a specific x range
fn calculate_y_bounds_for_range(
    physics_results: &[PhysicsResult],
    processed_traces: &[Vec<f64>],
    x_min: f64,
    x_max: f64,
) -> (f64, f64) {
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    
    for (physics, processed) in physics_results.iter().zip(processed_traces.iter()) {
        let trace_dbm = trace_watts_to_dbm(processed);
        let delta_z_m = physics.delta_z.get::<meter>();
        
        for (i, &db) in trace_dbm.iter().enumerate() {
            let distance = i as f64 * delta_z_m;
            if distance >= x_min && distance <= x_max && db.is_finite() {
                y_min = y_min.min(db);
                y_max = y_max.max(db);
            }
        }
    }
    
    // Add padding
    let y_range = y_max - y_min;
    y_min -= y_range * 0.1;
    y_max += y_range * 0.1;
    
    (y_min, y_max)
}

/// Calculate chart bounds from all traces
fn calculate_chart_bounds(
    physics_results: &[PhysicsResult],
    processed_traces: &[Vec<f64>],
) -> (f64, f64, f64, f64) {
    let mut x_max = 0.0f64;
    let mut y_min = f64::MAX;
    let mut y_max = f64::MIN;
    
    for (physics, processed) in physics_results.iter().zip(processed_traces.iter()) {
        let trace_dbm = trace_watts_to_dbm(processed);
        x_max = x_max.max(physics.delta_z.get::<meter>() * physics.num_points as f64);
        
        for &db in &trace_dbm {
            if db.is_finite() {
                y_min = y_min.min(db);
                y_max = y_max.max(db);
            }
        }
    }
    
    // Add some padding
    let y_range = y_max - y_min;
    y_min -= y_range * 0.05;
    y_max += y_range * 0.1;
    
    (0.0, x_max, y_min, y_max)
}

/// Draw offset region shading
fn draw_offset_regions<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<plotters::coord::types::RangedCoordf64, plotters::coord::types::RangedCoordf64>>,
    ground_truth: &GroundTruth,
    y_min: f64,
    y_max: f64,
) -> Result<(), SimulatorError> 
where
    DB::ErrorType: std::fmt::Display,
{
    let front_panel_offset = ground_truth.config.front_panel_offset_m;
    let launch_cable_end = ground_truth.config.launch_cable
        .as_ref()
        .map(|lc| front_panel_offset + lc.length_m)
        .unwrap_or(front_panel_offset);
    
    // Front panel offset region (light blue)
    if front_panel_offset > 0.0 {
        chart.draw_series(std::iter::once(Rectangle::new(
            [(0.0, y_min), (front_panel_offset, y_max)],
            RGBAColor(100, 149, 237, 0.2).filled(),
        )))
        .map_err(|e| SimulatorError::OutputError(format!("Offset region error: {}", e)))?;
    }
    
    // Launch cable region (light green)
    if launch_cable_end > front_panel_offset {
        chart.draw_series(std::iter::once(Rectangle::new(
            [(front_panel_offset, y_min), (launch_cable_end, y_max)],
            RGBAColor(144, 238, 144, 0.2).filled(),
        )))
        .map_err(|e| SimulatorError::OutputError(format!("Launch cable region error: {}", e)))?;
    }
    
    Ok(())
}

/// Calculate total offset (front panel + launch cable) for converting FUT-relative positions to absolute
fn calculate_total_offset(ground_truth: &GroundTruth) -> f64 {
    let front_panel_offset = ground_truth.config.front_panel_offset_m;
    let launch_cable_length = ground_truth.config.launch_cable
        .as_ref()
        .map(|lc| lc.length_m)
        .unwrap_or(0.0);
    front_panel_offset + launch_cable_length
}

/// Draw vertical event markers
fn draw_event_markers<DB: DrawingBackend>(
    chart: &mut ChartContext<DB, Cartesian2d<plotters::coord::types::RangedCoordf64, plotters::coord::types::RangedCoordf64>>,
    ground_truth: &GroundTruth,
    y_min: f64,
    y_max: f64,
) -> Result<(), SimulatorError>
where
    DB::ErrorType: std::fmt::Display,
{
    // Calculate total offset to convert FUT-relative positions to absolute trace positions
    let total_offset = calculate_total_offset(ground_truth);
    
    for event in &ground_truth.events {
        // Event positions in ground_truth are relative to fiber under test (FUT)
        // Trace data positions start from front panel (0m)
        // So we need to add total_offset to align markers with trace data
        let pos = event.position_m + total_offset;
        let color = match event.event_type.as_str() {
            "splice" => RGBAColor(255, 165, 0, 0.7),      // Orange
            "connector" => RGBAColor(255, 0, 0, 0.7),     // Red
            "splitter" => RGBAColor(128, 0, 128, 0.7),    // Purple
            "macrobend" => RGBAColor(255, 255, 0, 0.7),   // Yellow
            "end" => RGBAColor(0, 0, 0, 0.7),             // Black
            _ => RGBAColor(128, 128, 128, 0.7),           // Gray
        };
        
        // Draw vertical line
        chart.draw_series(std::iter::once(PathElement::new(
            vec![(pos, y_min), (pos, y_max)],
            color.stroke_width(1),
        )))
        .map_err(|e| SimulatorError::OutputError(format!("Event marker error: {}", e)))?;
    }
    
    Ok(())
}

/// Write a complete PDF report
pub fn write_pdf_report(
    result: &SimulationResult,
    config: &SimulationConfig,
    output_path: &Path,
    pdf_config: &PdfReportConfig,
) -> Result<(), SimulatorError> {
    // Generate main trace chart
    let main_chart_bmp = generate_trace_chart(
        &result.physics_results,
        &result.processed_traces,
        &result.ground_truth,
        pdf_config,
    )?;
    
    // Generate pulse shape chart
    let pulse_chart_bmp = generate_pulse_chart(config, pdf_config)?;
    
    // Generate event zoom charts
    let event_charts: Vec<(String, Vec<u8>)> = result.ground_truth.events
        .iter()
        .map(|event| {
            let label = format!("{} @ {:.1}m", event.event_type, event.position_m);
            let chart = generate_event_zoom_chart(event, &result.ground_truth, &result.physics_results, &result.processed_traces, pdf_config)?;
            Ok((label, chart))
        })
        .collect::<Result<Vec<_>, SimulatorError>>()?;
    
    // Create PDF document - start with LANDSCAPE page for main chart
    let (doc, page1, layer1) = PdfDocument::new(
        format!("OTDR Simulation Report - {}", config.output_basename),
        Mm(pdf_config.landscape_width_mm),  // Landscape
        Mm(pdf_config.landscape_height_mm),
        "Main Chart",
    );
    
    // Fonts
    let font = doc.add_builtin_font(BuiltinFont::HelveticaBold)
        .map_err(|e| SimulatorError::OutputError(format!("Font error: {}", e)))?;
    let font_regular = doc.add_builtin_font(BuiltinFont::Helvetica)
        .map_err(|e| SimulatorError::OutputError(format!("Font error: {}", e)))?;
    
    // Page 1: Main trace chart (Landscape)
    let page1_layer = doc.get_page(page1).get_layer(layer1);
    
    // Title
    let title = format!("OTDR Simulation Report: {}", config.output_basename);
    page1_layer.use_text(
        &title,
        14.0,
        Mm(pdf_config.margin_mm),
        Mm(pdf_config.landscape_height_mm - pdf_config.margin_mm - 5.0),
        &font,
    );
    
    // Timestamp
    let timestamp = chrono_lite_timestamp();
    page1_layer.use_text(
        &format!("Generated: {}", timestamp),
        9.0,
        Mm(pdf_config.margin_mm),
        Mm(pdf_config.landscape_height_mm - pdf_config.margin_mm - 11.0),
        &font_regular,
    );
    
    // Main chart
    let chart_width_mm = pdf_config.landscape_width_mm - 2.0 * pdf_config.margin_mm;
    let chart_height_mm = chart_width_mm * (pdf_config.chart_height as f32 / pdf_config.chart_width as f32);
    let chart_y = pdf_config.landscape_height_mm - pdf_config.margin_mm - 16.0 - chart_height_mm;
    
    add_image_to_layer(
        &page1_layer,
        &main_chart_bmp,
        pdf_config.margin_mm,
        chart_y,
        chart_width_mm,
        pdf_config.chart_width,
    )?;
    
    // Page 2: Pulse shape and event details (Landscape)
    let (page2, layer2) = doc.add_page(
        Mm(pdf_config.landscape_width_mm),
        Mm(pdf_config.landscape_height_mm),
        "Pulse and Events",
    );
    let page2_layer = doc.get_page(page2).get_layer(layer2);
    
    // Section title
    page2_layer.use_text(
        "Pulse Shape & Event Details",
        14.0,
        Mm(pdf_config.margin_mm),
        Mm(pdf_config.landscape_height_mm - pdf_config.margin_mm - 5.0),
        &font,
    );
    
    // Pulse shape chart on the left
    let small_chart_width_mm = (pdf_config.landscape_width_mm - 3.0 * pdf_config.margin_mm) / 2.0;
    let small_chart_height_mm = small_chart_width_mm * (pdf_config.small_chart_height as f32 / pdf_config.small_chart_width as f32);
    
    add_image_to_layer(
        &page2_layer,
        &pulse_chart_bmp,
        pdf_config.margin_mm,
        pdf_config.landscape_height_mm - pdf_config.margin_mm - 15.0 - small_chart_height_mm,
        small_chart_width_mm,
        pdf_config.small_chart_width,
    )?;
    
    // Event zoom charts - 2 per row
    let mut event_x = pdf_config.margin_mm;
    let mut event_y = pdf_config.landscape_height_mm - pdf_config.margin_mm - 20.0 - small_chart_height_mm - 10.0 - small_chart_height_mm;
    let charts_per_row = 2;
    
    for (i, (_label, chart_bmp)) in event_charts.iter().enumerate() {
        // Check if we need a new page
        if event_y < pdf_config.margin_mm + small_chart_height_mm {
            // Add new page
            let (new_page, new_layer) = doc.add_page(
                Mm(pdf_config.landscape_width_mm),
                Mm(pdf_config.landscape_height_mm),
                "Event Details Continued",
            );
            let new_page_layer = doc.get_page(new_page).get_layer(new_layer);
            
            new_page_layer.use_text(
                "Event Details (continued)",
                14.0,
                Mm(pdf_config.margin_mm),
                Mm(pdf_config.landscape_height_mm - pdf_config.margin_mm - 5.0),
                &font,
            );
            
            event_x = pdf_config.margin_mm;
            event_y = pdf_config.landscape_height_mm - pdf_config.margin_mm - 15.0 - small_chart_height_mm;
            
            add_image_to_layer(
                &new_page_layer,
                chart_bmp,
                event_x,
                event_y,
                small_chart_width_mm,
                pdf_config.small_chart_width,
            )?;
        } else {
            add_image_to_layer(
                &page2_layer,
                chart_bmp,
                event_x,
                event_y,
                small_chart_width_mm,
                pdf_config.small_chart_width,
            )?;
        }
        
        // Move position for next chart
        if (i + 1) % charts_per_row == 0 {
            event_x = pdf_config.margin_mm;
            event_y -= small_chart_height_mm + 5.0;
        } else {
            event_x += small_chart_width_mm + pdf_config.margin_mm;
        }
    }
    
    // Page for text content (Portrait - more room for text)
    let (text_page, text_layer) = doc.add_page(
        Mm(pdf_config.portrait_width_mm),
        Mm(pdf_config.portrait_height_mm),
        "Text Content",
    );
    let text_page_layer = doc.get_page(text_page).get_layer(text_layer);
    
    add_text_content(&text_page_layer, result, config, pdf_config, &font, &font_regular)?;
    
    // Save PDF
    doc.save(&mut std::io::BufWriter::new(
        std::fs::File::create(output_path)
            .map_err(|e| SimulatorError::OutputError(format!("Failed to create PDF file: {}", e)))?
    ))
    .map_err(|e| SimulatorError::OutputError(format!("Failed to save PDF: {}", e)))?;
    
    Ok(())
}

/// Add text content to a page (offset info, event table, config summary)
fn add_text_content(
    layer: &PdfLayerReference,
    result: &SimulationResult,
    _sim_config: &SimulationConfig,
    pdf_config: &PdfReportConfig,
    font: &printpdf::IndirectFontRef,
    font_regular: &printpdf::IndirectFontRef,
) -> Result<(), SimulatorError> {
    let mut y_pos = pdf_config.portrait_height_mm - pdf_config.margin_mm - 5.0;
    
    // Section: Offset Information
    layer.use_text("Offset Information", 14.0, Mm(pdf_config.margin_mm), Mm(y_pos), font);
    y_pos -= 8.0;
    
    let front_panel = result.ground_truth.config.front_panel_offset_m;
    layer.use_text(
        &format!("Front Panel Offset: {:.1} m", front_panel),
        10.0,
        Mm(pdf_config.margin_mm + 5.0),
        Mm(y_pos),
        font_regular,
    );
    y_pos -= 5.0;
    
    if let Some(lc) = &result.ground_truth.config.launch_cable {
        layer.use_text(
            &format!("Launch Cable: {:.1} m ({}, {:.2} dB loss)", lc.length_m, lc.connector_type, lc.connector_loss_db),
            10.0,
            Mm(pdf_config.margin_mm + 5.0),
            Mm(y_pos),
            font_regular,
        );
        y_pos -= 5.0;
        
        let total_offset = front_panel + lc.length_m;
        layer.use_text(
            &format!("Total User Offset: {:.1} m", total_offset),
            10.0,
            Mm(pdf_config.margin_mm + 5.0),
            Mm(y_pos),
            font_regular,
        );
        y_pos -= 5.0;
    }
    
    y_pos -= 10.0;
    
    // Section: Event Table
    layer.use_text("Event Table", 14.0, Mm(pdf_config.margin_mm), Mm(y_pos), font);
    y_pos -= 8.0;
    
    // Table header
    layer.use_text("Position", 9.0, Mm(pdf_config.margin_mm + 5.0), Mm(y_pos), font);
    layer.use_text("Type", 9.0, Mm(pdf_config.margin_mm + 35.0), Mm(y_pos), font);
    layer.use_text("Parameters", 9.0, Mm(pdf_config.margin_mm + 65.0), Mm(y_pos), font);
    y_pos -= 5.0;
    
    // Table rows
    for event in &result.ground_truth.events {
        if y_pos < pdf_config.margin_mm + 10.0 {
            break; // Avoid going off page
        }
        
        let pos_str = if event.position_m >= 1000.0 {
            format!("{:.2} km", event.position_m / 1000.0)
        } else {
            format!("{:.1} m", event.position_m)
        };
        
        let params_str = format_event_params(&event.parameters);
        
        layer.use_text(&pos_str, 8.0, Mm(pdf_config.margin_mm + 5.0), Mm(y_pos), font_regular);
        layer.use_text(&event.event_type, 8.0, Mm(pdf_config.margin_mm + 35.0), Mm(y_pos), font_regular);
        layer.use_text(&params_str, 8.0, Mm(pdf_config.margin_mm + 65.0), Mm(y_pos), font_regular);
        y_pos -= 4.5;
    }
    
    Ok(())
}

fn format_event_params(params: &crate::output::GroundTruthEventParams) -> String {
    match params {
        crate::output::GroundTruthEventParams::Splice { insertion_loss_db, .. } => {
            format!("Loss: {:.2} dB", insertion_loss_db)
        }
        crate::output::GroundTruthEventParams::Connector { insertion_loss_db, return_loss_db, .. } => {
            if let Some(rl) = return_loss_db {
                format!("Loss: {:.2} dB, Refl: {:.2} dB", insertion_loss_db, rl)
            } else {
                format!("Loss: {:.2} dB", insertion_loss_db)
            }
        }
        crate::output::GroundTruthEventParams::Macrobend { loss_by_wavelength, .. } => {
            let mut details: Vec<String> = loss_by_wavelength.iter()
                .map(|(wl, loss)| format!("{}nm: {:.2}dB", wl, loss))
                .collect();
            details.sort(); // Stable output
            format!("Loss: {}", details.join(", "))
        }
        crate::output::GroundTruthEventParams::Splitter { split_ratio, total_loss_db, .. } => {
            format!("Split: {:.1}, Loss: {:.2} dB", split_ratio, total_loss_db)
        }
        crate::output::GroundTruthEventParams::End { return_loss_db, .. } => {
            format!("Refl: {:.2} dB", return_loss_db)
        }
    }
}

/// Helper to get simple timestamp
fn chrono_lite_timestamp() -> String {
    let now = std::time::SystemTime::now();
    let duration = now.duration_since(std::time::UNIX_EPOCH).unwrap();
    format!("UNIX {}", duration.as_secs())
}
