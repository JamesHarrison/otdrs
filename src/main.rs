//!
//! # otdrs
//! 
//! otdrs is a tool for parsing Telcordia SOR files into a neutral, open format for further processing.
//! 
//! The serde library is used for serialisation, and currently only JSON output is supported.
//! 
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
// #[macro_use]
extern crate nom;
pub mod parser;
use serde::Serialize;

/// A BlockInfo struct contains information about a specific block later in the file, and appears in the MapBlock
#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct BlockInfo<'a> {
    /// Name of the block
    identifier: &'a str,
    /// Revision number - major (3 digits), minor, cosmetic
    revision_number: u16,
    /// Size in bytes of the block
    size: i32
    
}

/// Every SOR file has a MapBlock which acts as a map to the file's contents
#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct MapBlock<'a> {
    /// Revision number - major (3 digits), minor, cosmetic - for the file as a whole
    revision_number: u16, 
    /// Block size for the map block
    block_size: i32,
    /// Number of blocks in the file
    block_count: i16,
    /// Information on all the blocks in this file
    block_info: Vec<BlockInfo<'a>> 
}

/// The GeneralParametersBlock is mandatory for the format and contains test-identifying information as well as
/// generic information about the test being run such as the nominal wavelength
#[derive(Debug, PartialEq, Eq, Hash, Serialize)]
pub struct GeneralParametersBlock<'a> {
    /// Language code - EN, CN, JP, etc.
    language_code: &'a str, 
    /// Cable identifier
    cable_id: &'a str, 
    /// Fibre identifier
    fiber_id: &'a str, 
    /// Fibre type - this is generally coded as the ITU-T standard definition, sans letters, e.g. 657, 655.
    fiber_type: i16, 
    /// Nominal test wavelength in nm
    nominal_wavelength: i16, 
    /// Start location for the test
    originating_location: &'a str, 
    /// End location for the test
    terminating_location: &'a str, 
    /// Cable code - free field
    cable_code: &'a str, 
    ///  NC for new condition, RC for as-repaired, OT as something else
    current_data_flag: &'a str, 
    /// User offset - This is essentially the launch lead length from the front panel offset 
    /// (provided in the fixed parameters block), in 100ps increments
    user_offset: i32,
    /// This is the same as user_offset, but measured in 10x the distance units in FixedParametersBlock.units_of_distance
    user_offset_distance: i32,
    /// Operator of the unit for the test
    operator: &'a str,
    /// Free comment field
    comment: &'a str,
}

/// Supplier parameters describe the OTDR unit itself, such as the optical module ID/serial number
/// Often this block also contains information about calibration dates in the "other" field.
#[derive(Debug, PartialEq, Serialize)]
pub struct SupplierParametersBlock<'a> {
    /// Manufacturer of the OTDR
    supplier_name: &'a str,
    /// Mainframe model number
    otdr_mainframe_id: &'a str,
    /// Mainframe serial number
    otdr_mainframe_sn: &'a str,
    /// Optical module model number
    optical_module_id: &'a str,
    /// Optical module serial number
    optical_module_sn: &'a str,
    /// Software revision
    software_revision: &'a str,
    /// Free text
    other: &'a str,
}

/// Fixed parameters block contains key information for interpreting the test data
#[derive(Debug, PartialEq, Serialize)]
pub struct FixedParametersBlock<'a> {
    /// Datestamp - unix epoch seconds, 32-bit. Remember not to do any OTDR tests after 2038.
    date_time_stamp: u32,
    /// Units of distance - km, mt, ft, kf, mi, etc. Typically mt (in civilised nations)
    units_of_distance: &'a str,
    /// Actual wavelength used - normally the factory-calibrated wavelength in nm, or nominal wavelength
    actual_wavelength: i16,
    /// Acquisition offset - this is the length of fibre from the OTDR port to the first data point in the DataPoints, in 100ps increments
    acquisition_offset: i32,
    /// As acquisition_offset, but as 10x units_of_distance
    acquisition_offset_distance: i32,
    /// The total number of pulse widths used, if more than one pulse width's results are stored in the file
    total_n_pulse_widths_used: i16,
    /// The list of pulse widths used, in nanoseconds
    pulse_widths_used: Vec<i16>,
    /// Data spacing, time taken to acquire 10,000 points in 100ps increments
    data_spacing: Vec<i32>,
    /// Number of points stored for each pulse width
    n_data_points_for_pulse_widths_used: Vec<i32>,
    /// Group index - the refractive index of the fibre, default to 146800 if nothing supplied
    group_index: i32,
    /// Backscatter coefficient - 
    backscatter_coefficient: i16,
    /// Number of averages - the number of samples that were averaged to generate the result - may be used instead of averaging time
    number_of_averages: i32,
    /// Averaging time - may be supplied instead of number of averages - in seconds x 10
    averaging_time: u16,
    /// Acquisition range set by the tester to reach the end of the fibre - as with other distance measurements, 100ps increments
    acquisition_range: i32,
    /// Acquisition range in 10x distance units, as an alternate to acquisition_range
    acquisition_range_distance: i32,
    /// Front panel offset is the time taken, in 100ps increments, between the front-end of the optical TRX and the front panel connector
    front_panel_offset: i32,
    /// Noise floor level - the lowest power level for which 98% of the noise data lies below; 5-digit -dB value (e.g. 10200 = -10.2dB)
    noise_floor_level: u16,
    /// Scale factor for the noise floor level - defaults to 1
    noise_floor_scale_factor: i16,
    /// Attenuation in dB*1000 applied by the instrument if done by the instrument
    power_offset_first_point: u16,
    /// The threshold in dB*1000 for a loss-type event; default 00200
    loss_threshold: u16,
    /// The threshold in -dB*1000 for reflectance events; default -55000
    reflectance_threshold: u16,
    /// The threshold in dB*1000 for the loss taken to detect the end of the fibre; default 03000
    end_of_fibre_threshold: u16,
    /// Trace type - identifies if this is a standard one-way trace, a bidirectional trace, reference trace, difference trace, or reversed trace
    trace_type: &'a str,
    /// Window coordinate for the upper right window corner
    window_coordinate_1: i32,
    /// Power coordinate for the upper right window corner
    window_coordinate_2: i32,
    /// Window coordinate for the lower left window corner
    window_coordinate_3: i32,
    /// Power coordinate for the lower left window corner
    window_coordinate_4: i32,
}

/// KeyEvents describe a single event along the fibre path detected by the OTDR
#[derive(Debug, PartialEq, Serialize)]
pub struct KeyEvent<'a> {
    /// Event number - this is from 0 to n
    event_number: i16,
    /// Event propogation time is the time in 100ps units from the front panel to the event
    event_propogation_time: i32,
    /// The span loss in db/km (as a 5-digit value, i.e. dB*1000) for the fibre entering the event
    attenuation_coefficient_lead_in_fiber: i16,
    /// Loss in dB*1000 for the event
    event_loss: i16,
    /// Reflectance in -dB*1000 for the event
    event_reflectance: i32,
    /// Code for the event is a 6-byte string:
    /// Byte 1:
    ///     0 = nonreflective, 1 = reflective, 2 = saturated reflective
    /// Byte 2:
    ///     A = added by user, M = moved by user, E = end of fibre, F = found by software, O = out of range, D = modified end of fibre
    /// Remaining bytes are the Landmark number if used - 9s otherwise
    event_code: &'a str,
    /// Loss measurement technique - 2P for two point, LS for least squares, OT for other
    loss_measurement_technique: &'a str,
    /// Marker location - ML1 is the OTDR side for 2P/LS/OT measurements
    marker_location_1: i32,
    /// Marker location - ML2 is the OTDR side for LS measurements, and bounds the event for 2P/OT
    marker_location_2: i32,
    /// Marker location - ML3 is on the far side for LS measurements, and empty for 2P/OT
    marker_location_3: i32,
    /// Marker location - ML4 is on the far side for LS measurements, and empty for 2P/OT
    marker_location_4: i32,
    /// Marker location - ML5 is the reflectance calculation position
    marker_location_5: i32,
    /// Free comment on the event
    comment: &'a str,
}

/// The last key event is as the KeyEvent, with some additional fields; see KeyEvent for the documentation of other fields
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
    /// End to end loss is in dB*1000 and measures the loss between the two markers defined below
    end_to_end_loss: i32,
    /// Start of the measurement span - typically user offset
    end_to_end_marker_position_1: i32,
    /// End of the measurement span - typically end of fibre event position
    end_to_end_marker_position_2: i32,
    /// Return loss in dB*1000 for the markers defined below
    optical_return_loss: u16,
    /// Start of the measurement span - typically user offset
    optical_return_loss_marker_position_1: i32,
    /// End of the measurement span - typically end of fibre event position
    optical_return_loss_marker_position_2: i32,
}

/// List of key events and a pointer to the last key event
#[derive(Debug, PartialEq, Serialize)]
pub struct KeyEvents<'a> {
    number_of_key_events: i16,
    key_events: Vec<KeyEvent<'a>>,
    last_key_event: LastKeyEvent<'a>,
}

/// Landmarks are a slightly esoteric feature not often used in SOR files for field test equipment.
/// They act to relate OTDR events to real-world information such as WGS84 GPS data, known fibre MFDs, metre markers, etc
#[derive(Debug, PartialEq, Serialize)]
pub struct Landmark<'a> {
    landmark_number: i16,
    /// Landmark code identifies the landmark - see page 27 of the standard for the list
    landmark_code: &'a str,
    /// Location in 100ps from user offset to the landmark
    landmark_location: i32,
    related_event_number: i16,
    gps_longitude: i32,
    gps_latitude: i32,
    /// Fibre correction factor is the difference in 100*% between the optical path and the cable length; otherwise known as heliax correction
    fiber_correction_factor_lead_in_fiber: i16,
    sheath_marker_entering_landmark: i32,
    sheath_marker_leaving_landmark: i32,
    units_of_sheath_marks_leaving_landmark: &'a str,
    mode_field_diameter_leaving_landmark: i16,
    comment: &'a str,
}

/// DataPointsAtScaleFactor is the struct that actually contains the data points of the measurements for a given scale factor
#[derive(Debug, PartialEq, Serialize)]
pub struct DataPointsAtScaleFactor {
    /// Number of points in this block
    n_points: i32,
    /// Scale factor for the data, as 1000*SF
    scale_factor: i16,
    /// Data points as dB*1000
    data: Vec<u16>,
}

/// DataPoints holds all the different datasets in this file - one per scale factor
#[derive(Debug, PartialEq, Serialize)]
pub struct DataPoints {
    number_of_data_points: i32,
    total_number_scale_factors_used: i16,
    scale_factors: Vec<DataPointsAtScaleFactor>,
}

/// LinkParameters are a bit esoteric and not often found in test equipment, more the likes of network management systems.
/// Contains a set of landmarks which describe the physical fibre path and may relate this to described KeyEvents
#[derive(Debug, PartialEq, Serialize)]
pub struct LinkParameters<'a> {
    number_of_landmarks: i16,
    landmarks: Vec<Landmark<'a>>,
}

/// ProprietaryBlock is a struct to contain third-party proprietary information.
/// This is mostly used for vendor-specific special sauce, extra data, extra analysis, etc.
/// otdrs extracts the header, and stores the data as an array of bytes.
#[derive(Debug, PartialEq, Serialize)]
pub struct ProprietaryBlock<'a> {
    header: &'a str,
    data: &'a [u8],
}

/// SORFile describes a full SOR file. All blocks except MapBlock are Option types as we cannot
/// guarantee the parser will find them, but many blocks are in fact mandatory in the specification
/// so compliant files will provide them.
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

/// By default we simply read the file provided as the first argument, and print the parsed file as JSON to stdout
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
