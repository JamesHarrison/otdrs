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
    data_spacing: i32,
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
pub struct SORFile<'b> {
    map: MapBlock<'b>,
    general_parameters: GeneralParametersBlock<'b>,
    supplier_parameters: SupplierParametersBlock<'b>,
    fixed_parameters: FixedParametersBlock<'b>,
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
