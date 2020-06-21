/*

otdrs - a simple Rust tool to convert SOR files into an open format as transparently as possible to enable analysis without all the faff of dealing with a weird format

*/
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
// #[macro_use]
extern crate nom;
pub mod parser;
use serde::{Serialize};

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct BlockInfo<'a> {
    identifier: &'a str,
    revision_number: u16,
    size: i32
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct MapBlock<'a> {
    revision_number: u16,
    block_size: i32,
    block_count: i16,
    block_info: Vec<BlockInfo<'a>>
}

#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct GeneralParametersBlock<'a> {
    language_code: &'a str,
    cable_id: &'a str,
    fiber_id: &'a str,
    fiber_type: i16,
    nominal_wavelength: i16,
    originating_location: &'a str,
    terminating_location: &'a str,
    cable_code: &'a str,
    current_data_flag: &'a str,
    user_offset: i32,
    user_offset_distance: i32,
    operator: &'a str,
    comment: &'a str,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct SupplierParametersBlock<'a> {
    supplier_name: &'a str,
    otdr_mainframe_id: &'a str,
    otdr_mainframe_sn: &'a str,
    optical_module_id: &'a str,
    optical_module_sn: &'a str,
    software_revision: &'a str,
    other: &'a str,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct FixedParametersBlock<'a> {
    date_time_stamp: u32,
    units_of_distance: &'a str,
    actual_wavelength: i16,
    acquisition_offset: i32,
    acquisition_offset_distance: i32,
    total_n_pulse_widths_used: i16,
    pulse_widths_used: Vec<i16>, // TODO: More
    data_spacing: Vec<i32>,
    n_data_points_for_pulse_widths_used: Vec<i32>,
    group_index: i32,
    backscatter_coefficient: i16,
    number_of_averages: i32,
    averaging_time: u16,
    acquisition_range: i32,
    acquisition_range_distance: i32,
    front_panel_offset: i32,
    noise_floor_level: u16,
    noise_floor_scale_factor: i16,
    power_offset_first_point: u16,
    loss_threshold: u16,
    reflectance_threshold: u16,
    end_of_fibre_threshold: u16,
    trace_type: &'a str,
    window_coordinate_1: i32,
    window_coordinate_2: i32,
    window_coordinate_3: i32,
    window_coordinate_4: i32,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct KeyEvent<'a> {
    event_number: i16,
    event_propogation_time: i32,
    attenuation_coefficient_lead_in_fiber: i16,
    event_loss: i16,
    event_reflectance: i32,
    event_code: &'a str,
    loss_measurement_technique: &'a str,
    marker_location_1: i32,
    marker_location_2: i32,
    marker_location_3: i32,
    marker_location_4: i32,
    marker_location_5: i32,
    comment: &'a str,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct LastKeyEvent<'a> {
    event_number: i16,
    event_propogation_time: i32,
    attenuation_coefficient_lead_in_fiber: i16,
    event_loss: i16,
    event_reflectance: i32,
    event_code: &'a str,
    loss_measurement_technique: &'a str,
    marker_location_1: i32,
    marker_location_2: i32,
    marker_location_3: i32,
    marker_location_4: i32,
    marker_location_5: i32,
    comment: &'a str,
    end_to_end_loss: i32,
    end_to_end_marker_position_1: i32,
    end_to_end_marker_position_2: i32,
    optical_return_loss: u16,
    optical_return_loss_marker_position_1: i32,
    optical_return_loss_marker_position_2: i32,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct KeyEvents<'a> {
    number_of_key_events: i16,
    key_events: Vec<KeyEvent<'a>>,
    last_key_event: LastKeyEvent<'a>,
}


#[derive(Debug, PartialEq, Serialize)]
pub struct Landmark<'a> {
    landmark_number: i16,
    landmark_code: &'a str,
    landmark_location: i32,
    related_event_number: i16,
    gps_longitude: i32,
    gps_latitude: i32,
    fiber_correction_factor_lead_in_fiber: i16,
    sheath_marker_entering_landmark: i32,
    sheath_marker_leaving_landmark: i32,
    units_of_sheath_marks_leaving_landmark: &'a str,
    mode_field_diameter_leaving_landmark: i16,
    comment: &'a str,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct DataPointsAtScaleFactor {
    n_points: i32,
    scale_factor: i16,
    data: Vec<u16>,
}

#[derive(Debug, PartialEq, Serialize)]
pub struct DataPoints {
    number_of_data_points: i32,
    total_number_scale_factors_used: i16,
    scale_factors: Vec<DataPointsAtScaleFactor>,
}



#[derive(Debug, PartialEq, Serialize)]
pub struct LinkParameters<'a> {
    number_of_landmarks: i16,
    landmarks: Vec<Landmark<'a>>,
}


#[derive(Debug, PartialEq, Serialize)]
pub struct ProprietaryBlock<'a> {
    header: &'a str,
    data: &'a [u8],
}


#[derive(Debug, PartialEq, Serialize)]
pub struct SORFile<'b> {
    map: MapBlock<'b>,
    general_parameters: Option<GeneralParametersBlock<'b>>,
    supplier_parameters: Option<SupplierParametersBlock<'b>>,
    fixed_parameters: Option<FixedParametersBlock<'b>>,
    key_events: Option<KeyEvents<'b>>,
    link_parameters: Option<LinkParameters<'b>>,
    data_points: Option<DataPoints>,
    proprietary_blocks: Vec<ProprietaryBlock<'b>>,
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];

    let mut file = File::open(filename)?;
    let mut buffer = Vec::new();
    // println!("Hello, world! I have read {}", buffer.len());
    file.read_to_end(&mut buffer)?;
    let sor = parser::parse_file(buffer.as_slice()).unwrap().1;
    // println!("Test {:?}", sor.map.revision_number);
    let j = serde_json::to_string(&sor)?;
    print!("{}", j);
    Ok(())
}
