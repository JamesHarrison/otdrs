use crate::{
    BlockInfo, DataPoints, DataPointsAtScaleFactor, FixedParametersBlock, GeneralParametersBlock,
    KeyEvent, KeyEvents, Landmark, LastKeyEvent, LinkParameters, MapBlock, ProprietaryBlock,
    SORFile, SupplierParametersBlock,
};
use nom::{
    bytes::complete::{tag, take, take_until},
    multi::count,
    number::complete::{le_i16, le_i32, le_u16, le_u32},
    sequence::terminated,
    IResult,
};
use std::str;

/// Block header string for the map block
const BLOCK_ID_MAP: &str = "Map";
/// Block header string for the general parameters block
const BLOCK_ID_GENPARAMS: &str = "GenParams";
/// Block header string for the supplier parameters block
const BLOCK_ID_SUPPARAMS: &str = "SupParams";
/// Block header string for the fixed parameters block
const BLOCK_ID_FXDPARAMS: &str = "FxdParams";
/// Block header string for the key events block
const BLOCK_ID_KEYEVENTS: &str = "KeyEvents";
/// Block header string for the link parameters block
const BLOCK_ID_LNKPARAMS: &str = "LnkParams";
/// Block header string for the data points block
const BLOCK_ID_DATAPTS: &str = "DataPts";
/// Block header string for the checksum block
const BLOCK_ID_CHECKSUM: &str = "Cksum";

/// Parses to look for a block header, null-terminated, and returns the bytes (sans null character)
fn block_header<'a>(i: &'a [u8], header: &str) -> IResult<&'a [u8], &'a [u8]> {
    terminated(tag(header), tag("\0"))(i)
}

/// Parse a block information sequence within the map block
fn map_block_info(i: &[u8]) -> IResult<&[u8], BlockInfo<'_>> {
    let (i, header) = null_terminated_str(i)?;
    let (i, revision_number) = le_u16(i)?;
    let (i, size) = le_i32(i)?;
    return Ok((
        i,
        BlockInfo {
            identifier: header,
            revision_number: revision_number,
            size: size,
        },
    ));
}

/// Parses the map block in a SOR file, which contains information about the location of all blocks in the file
pub fn map_block(i: &[u8]) -> IResult<&[u8], MapBlock> {
    let (i, _) = block_header(i, BLOCK_ID_MAP)?;
    let (i, revision_number) = le_u16(i)?;
    let (i, block_size) = le_i32(i)?;
    let (i, block_count) = le_i16(i)?;
    let blocks_to_read: usize = (block_count - 1) as usize;
    let (i, blocks) = count(map_block_info, blocks_to_read)(i)?;
    return Ok((
        i,
        MapBlock {
            revision_number: revision_number,
            block_count: block_count,
            block_size: block_size,
            block_info: blocks,
        },
    ));
}

/// Parse an incoming byte sequence until a null character is found and return the bytes to that point, consuming the null
fn null_terminated_chunk(i: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(take_until("\0"), tag("\0"))(i)
}

/// Convert a series of bytes into a string, handling zero-length input
fn parse_string(input: &[u8]) -> &str {
    if input.len() > 0 {
        return str::from_utf8(input).unwrap();
    } else {
        return "";
    }
}

/// Parse a null-terminated variable length string
pub fn null_terminated_str(i: &[u8]) -> IResult<&[u8], &str> {
    let (i, seg) = null_terminated_chunk(i)?;
    let string = parse_string(seg);
    return Ok((i, string));
}

/// Parse a fixed-length string of the given number of bytes
pub fn fixed_length_str(i: &[u8], n_bytes: usize) -> IResult<&[u8], &str> {
    let (i, seg) = take(n_bytes * (1u8 as usize))(i)?;
    let string = parse_string(seg);
    return Ok((i, string));
}

/// Parse the general parameters block, which contains acquisition information as well as locations/identifiers.
pub fn general_parameters_block<'a>(i: &[u8]) -> IResult<&[u8], GeneralParametersBlock<'_>> {
    let (i, _) = block_header(i, BLOCK_ID_GENPARAMS)?;
    let (i, language_code) = fixed_length_str(i, 2)?;
    let (i, cable_id) = null_terminated_str(i)?;
    let (i, fiber_id) = null_terminated_str(i)?;
    let (i, fiber_type) = le_i16(i)?;
    let (i, nominal_wavelength) = le_i16(i)?;
    let (i, originating_location) = null_terminated_str(i)?;
    let (i, terminating_location) = null_terminated_str(i)?;
    let (i, cable_code) = null_terminated_str(i)?;
    let (i, current_data_flag) = fixed_length_str(i, 2)?;
    let (i, user_offset) = le_i32(i)?;
    let (i, user_offset_distance) = le_i32(i)?;
    let (i, operator) = null_terminated_str(i)?;
    let (i, comment) = null_terminated_str(i)?;
    return Ok((
        i,
        GeneralParametersBlock {
            language_code: language_code,
            cable_id: cable_id,
            fiber_id: fiber_id,
            fiber_type: fiber_type,
            nominal_wavelength: nominal_wavelength,
            originating_location: originating_location,
            terminating_location: terminating_location,
            cable_code: cable_code,
            current_data_flag: current_data_flag,
            user_offset: user_offset,
            user_offset_distance: user_offset_distance,
            operator: operator,
            comment: comment,
        },
    ));
}

/// Parse the supplier parameters block, which contains information about the OTDR equipment used.
pub fn supplier_parameters_block<'a>(i: &[u8]) -> IResult<&[u8], SupplierParametersBlock<'_>> {
    let (i, _) = block_header(i, BLOCK_ID_SUPPARAMS)?;
    let (i, supplier_name) = null_terminated_str(i)?;
    let (i, otdr_mainframe_id) = null_terminated_str(i)?;
    let (i, otdr_mainframe_sn) = null_terminated_str(i)?;
    let (i, optical_module_id) = null_terminated_str(i)?;
    let (i, optical_module_sn) = null_terminated_str(i)?;
    let (i, software_revision) = null_terminated_str(i)?;
    let (i, other) = null_terminated_str(i)?;
    return Ok((
        i,
        SupplierParametersBlock {
            supplier_name: supplier_name,
            otdr_mainframe_id: otdr_mainframe_id,
            otdr_mainframe_sn: otdr_mainframe_sn,
            optical_module_id: optical_module_id,
            optical_module_sn: optical_module_sn,
            software_revision: software_revision,
            other: other,
        },
    ));
}

/// Parse the fixed paramters block, which contains most of the information required to interpret the stored data.
pub fn fixed_parameters_block<'a>(i: &[u8]) -> IResult<&[u8], FixedParametersBlock<'_>> {
    let (i, _) = block_header(i, BLOCK_ID_FXDPARAMS)?;
    let (i, date_time_stamp) = le_u32(i)?;
    let (i, units_of_distance) = fixed_length_str(i, 2)?;
    let (i, actual_wavelength) = le_i16(i)?;
    let (i, acquisition_offset) = le_i32(i)?;
    let (i, acquisition_offset_distance) = le_i32(i)?;
    let (i, total_n_pulse_widths_used) = le_i16(i)?;
    let pulse_width_count: usize = total_n_pulse_widths_used as usize;
    let (i, pulse_widths_used) = count(le_i16, pulse_width_count)(i)?;
    //println!("{}, {:?}", pulse_width_count, pulse_widths_used);
    let (i, data_spacing) = count(le_i32, pulse_width_count)(i)?;
    let (i, n_data_points_for_pulse_widths_used) = count(le_i32, pulse_width_count)(i)?;
    let (i, group_index) = le_i32(i)?;
    let (i, backscatter_coefficient) = le_i16(i)?;
    let (i, number_of_averages) = le_i32(i)?;
    let (i, averaging_time) = le_u16(i)?;
    let (i, acquisition_range) = le_i32(i)?;
    let (i, acquisition_range_distance) = le_i32(i)?;
    let (i, front_panel_offset) = le_i32(i)?;
    let (i, noise_floor_level) = le_u16(i)?;
    let (i, noise_floor_scale_factor) = le_i16(i)?;
    let (i, power_offset_first_point) = le_u16(i)?;
    let (i, loss_threshold) = le_u16(i)?;
    let (i, reflectance_threshold) = le_u16(i)?;
    let (i, end_of_fibre_threshold) = le_u16(i)?;
    let (i, trace_type) = fixed_length_str(i, 2)?;
    let (i, window_coordinate_1) = le_i32(i)?;
    let (i, window_coordinate_2) = le_i32(i)?;
    let (i, window_coordinate_3) = le_i32(i)?;
    let (i, window_coordinate_4) = le_i32(i)?;
    return Ok((
        i,
        FixedParametersBlock {
            date_time_stamp: date_time_stamp,
            units_of_distance: units_of_distance,
            actual_wavelength: actual_wavelength,
            acquisition_offset: acquisition_offset,
            acquisition_offset_distance: acquisition_offset_distance,
            total_n_pulse_widths_used: total_n_pulse_widths_used,
            pulse_widths_used: pulse_widths_used,
            data_spacing: data_spacing,
            n_data_points_for_pulse_widths_used: n_data_points_for_pulse_widths_used,
            group_index: group_index,
            backscatter_coefficient: backscatter_coefficient,
            number_of_averages: number_of_averages,
            averaging_time: averaging_time,
            acquisition_range: acquisition_range,
            acquisition_range_distance: acquisition_range_distance,
            front_panel_offset: front_panel_offset,
            noise_floor_level: noise_floor_level,
            noise_floor_scale_factor: noise_floor_scale_factor,
            power_offset_first_point: power_offset_first_point,
            loss_threshold: loss_threshold,
            reflectance_threshold: reflectance_threshold,
            end_of_fibre_threshold: end_of_fibre_threshold,
            trace_type: trace_type,
            window_coordinate_1: window_coordinate_1,
            window_coordinate_2: window_coordinate_2,
            window_coordinate_3: window_coordinate_3,
            window_coordinate_4: window_coordinate_4,
        },
    ));
}

/// Parse any key event, except for the final key event, which is parsed with last_key_event as it differs structurally
fn key_event<'a>(i: &[u8]) -> IResult<&[u8], KeyEvent<'_>> {
    let (i, event_number) = le_i16(i)?;
    let (i, event_propogation_time) = le_i32(i)?;
    let (i, attenuation_coefficient_lead_in_fiber) = le_i16(i)?;
    let (i, event_loss) = le_i16(i)?;
    let (i, event_reflectance) = le_i32(i)?;
    let (i, event_code) = fixed_length_str(i, 6)?;
    let (i, loss_measurement_technique) = fixed_length_str(i, 2)?;
    let (i, marker_location_1) = le_i32(i)?;
    let (i, marker_location_2) = le_i32(i)?;
    let (i, marker_location_3) = le_i32(i)?;
    let (i, marker_location_4) = le_i32(i)?;
    let (i, marker_location_5) = le_i32(i)?;
    let (i, comment) = null_terminated_str(i)?;
    return Ok((
        i,
        KeyEvent {
            event_number: event_number,
            event_propogation_time: event_propogation_time,
            attenuation_coefficient_lead_in_fiber: attenuation_coefficient_lead_in_fiber,
            event_loss: event_loss,
            event_reflectance: event_reflectance,
            event_code: event_code,
            loss_measurement_technique: loss_measurement_technique,
            marker_location_1: marker_location_1,
            marker_location_2: marker_location_2,
            marker_location_3: marker_location_3,
            marker_location_4: marker_location_4,
            marker_location_5: marker_location_5,
            comment: comment,
        },
    ));
}

/// Parse the final key event in the key events block, which contains much of the end-to-end loss definitions
fn last_key_event<'a>(i: &[u8]) -> IResult<&[u8], LastKeyEvent<'_>> {
    let (i, event_number) = le_i16(i)?;
    let (i, event_propogation_time) = le_i32(i)?;
    let (i, attenuation_coefficient_lead_in_fiber) = le_i16(i)?;
    let (i, event_loss) = le_i16(i)?;
    let (i, event_reflectance) = le_i32(i)?;
    let (i, event_code) = fixed_length_str(i, 6)?;
    let (i, loss_measurement_technique) = fixed_length_str(i, 2)?;
    let (i, marker_location_1) = le_i32(i)?;
    let (i, marker_location_2) = le_i32(i)?;
    let (i, marker_location_3) = le_i32(i)?;
    let (i, marker_location_4) = le_i32(i)?;
    let (i, marker_location_5) = le_i32(i)?;
    let (i, comment) = null_terminated_str(i)?;
    let (i, end_to_end_loss) = le_i32(i)?;
    let (i, end_to_end_marker_position_1) = le_i32(i)?;
    let (i, end_to_end_marker_position_2) = le_i32(i)?;
    let (i, optical_return_loss) = le_u16(i)?;
    let (i, optical_return_loss_marker_position_1) = le_i32(i)?;
    let (i, optical_return_loss_marker_position_2) = le_i32(i)?;

    return Ok((
        i,
        LastKeyEvent {
            event_number: event_number,
            event_propogation_time: event_propogation_time,
            attenuation_coefficient_lead_in_fiber: attenuation_coefficient_lead_in_fiber,
            event_loss: event_loss,
            event_reflectance: event_reflectance,
            event_code: event_code,
            loss_measurement_technique: loss_measurement_technique,
            marker_location_1: marker_location_1,
            marker_location_2: marker_location_2,
            marker_location_3: marker_location_3,
            marker_location_4: marker_location_4,
            marker_location_5: marker_location_5,
            comment: comment,
            end_to_end_loss: end_to_end_loss,
            end_to_end_marker_position_1: end_to_end_marker_position_1,
            end_to_end_marker_position_2: end_to_end_marker_position_2,
            optical_return_loss: optical_return_loss,
            optical_return_loss_marker_position_1: optical_return_loss_marker_position_1,
            optical_return_loss_marker_position_2: optical_return_loss_marker_position_2,
        },
    ));
}

/// Parse the key events block
pub fn key_events_block<'a>(i: &[u8]) -> IResult<&[u8], KeyEvents<'_>> {
    let (i, _) = block_header(i, BLOCK_ID_KEYEVENTS)?;
    let (i, number_of_key_events) = le_i16(i)?;
    let (i, key_events) = count(key_event, (number_of_key_events - 1) as usize)(i)?;
    let (i, last_key_event) = last_key_event(i)?;
    return Ok((
        i,
        KeyEvents {
            number_of_key_events: number_of_key_events,
            key_events: key_events,
            last_key_event: last_key_event,
        },
    ));
}

// TODO: Test this, no test data to hand so this is probably correct
/// Parse a landmark from the link parameters block
pub fn landmark<'a>(i: &[u8]) -> IResult<&[u8], Landmark<'_>> {
    let (i, _) = block_header(i, BLOCK_ID_LNKPARAMS)?;
    let (i, landmark_number) = le_i16(i)?;
    let (i, landmark_code) = fixed_length_str(i, 2)?;
    let (i, landmark_location) = le_i32(i)?;
    let (i, related_event_number) = le_i16(i)?;
    let (i, gps_longitude) = le_i32(i)?;
    let (i, gps_latitude) = le_i32(i)?;
    let (i, fiber_correction_factor_lead_in_fiber) = le_i16(i)?;
    let (i, sheath_marker_entering_landmark) = le_i32(i)?;
    let (i, sheath_marker_leaving_landmark) = le_i32(i)?;
    let (i, units_of_sheath_marks_leaving_landmark) = fixed_length_str(i, 2)?;
    let (i, mode_field_diameter_leaving_landmark) = le_i16(i)?;
    let (i, comment) = null_terminated_str(i)?;
    return Ok((
        i,
        Landmark {
            landmark_number: landmark_number,
            landmark_code: landmark_code,
            landmark_location: landmark_location,
            related_event_number: related_event_number,
            gps_longitude: gps_longitude,
            gps_latitude: gps_latitude,
            fiber_correction_factor_lead_in_fiber: fiber_correction_factor_lead_in_fiber,
            sheath_marker_entering_landmark: sheath_marker_entering_landmark,
            sheath_marker_leaving_landmark: sheath_marker_leaving_landmark,
            units_of_sheath_marks_leaving_landmark: units_of_sheath_marks_leaving_landmark,
            mode_field_diameter_leaving_landmark: mode_field_diameter_leaving_landmark,
            comment: comment,
        },
    ));
}

// TODO: Test this, no test data to hand so this is probably correct
/// Extract link parameters and encoded landmarks from the LinkParams block.
pub fn link_parameters_block<'a>(i: &[u8]) -> IResult<&[u8], LinkParameters<'_>> {
    let (i, _) = block_header(i, BLOCK_ID_LNKPARAMS)?;
    let (i, number_of_landmarks) = le_i16(i)?;
    let (i, landmarks) = count(landmark, number_of_landmarks as usize)(i)?;
    return Ok((
        i,
        LinkParameters {
            number_of_landmarks: number_of_landmarks,
            landmarks: landmarks,
        },
    ));
}

/// Parse the data points at a defined scale factor within the DataPoints block
pub fn data_points_at_scale_factor(i: &[u8]) -> IResult<&[u8], DataPointsAtScaleFactor> {
    let (i, n_points) = le_i32(i)?;
    let (i, scale_factor) = le_i16(i)?;
    let (i, data) = count(le_u16, n_points as usize)(i)?;
    return Ok((
        i,
        DataPointsAtScaleFactor {
            n_points: n_points,
            scale_factor: scale_factor,
            data: data,
        },
    ));
}

/// Parse the DataPoints block and extract all the points for each scale factor
pub fn data_points_block<'a>(i: &[u8]) -> IResult<&[u8], DataPoints> {
    let (i, _) = block_header(i, BLOCK_ID_DATAPTS)?;
    let (i, number_of_data_points) = le_i32(i)?;
    let (i, total_number_scale_factors_used) = le_i16(i)?;
    let (i, scale_factors) = count(
        data_points_at_scale_factor,
        total_number_scale_factors_used as usize,
    )(i)?;
    return Ok((
        i,
        DataPoints {
            number_of_data_points: number_of_data_points,
            total_number_scale_factors_used: total_number_scale_factors_used,
            scale_factors: scale_factors,
        },
    ));
}
/// Parse the header string from a proprietary block, and return the remaining data for external parsers.
pub fn proprietary_block<'a>(i: &[u8]) -> IResult<&[u8], ProprietaryBlock> {
    let (i, header) = null_terminated_str(i)?;
    return Ok((
        &[],
        ProprietaryBlock {
            header: header,
            data: i,
        },
    ));
}

/// Parse a complete SOR file, extracting all known and proprietary blocks to a SORFile struct.
pub fn parse_file<'a>(i: &[u8]) -> IResult<&[u8], SORFile<'_>> {
    let mut general_parameters: Option<GeneralParametersBlock> = None;
    let mut supplier_parameters: Option<SupplierParametersBlock> = None;
    let mut fixed_parameters: Option<FixedParametersBlock> = None;
    let mut key_events: Option<KeyEvents> = None;
    let link_parameters: Option<LinkParameters> = None;
    let mut data_points: Option<DataPoints> = None;
    let mut proprietary_blocks: Vec<ProprietaryBlock> = Vec::new();
    let (_, map) = map_block(i)?;
    for block in &map.block_info {
        // Load the block's data
        let data = extract_block_data(i, block.identifier);
        // Parse it
        if block.identifier == BLOCK_ID_SUPPARAMS {
            let (_, ret) = supplier_parameters_block(data)?;
            supplier_parameters = Some(ret);
        } else if block.identifier == BLOCK_ID_GENPARAMS {
            let (_, ret) = general_parameters_block(data)?;
            general_parameters = Some(ret);
        } else if block.identifier == BLOCK_ID_FXDPARAMS {
            let (_, ret) = fixed_parameters_block(data)?;
            fixed_parameters = Some(ret);
        } else if block.identifier == BLOCK_ID_KEYEVENTS {
            let (_, ret) = key_events_block(data)?;
            key_events = Some(ret);
        } else if block.identifier == BLOCK_ID_LNKPARAMS {
            // Unimplemented due to lack of test data
        } else if block.identifier == BLOCK_ID_DATAPTS {
            let (_, ret) = data_points_block(data)?;
            data_points = Some(ret);
        } else if block.identifier == BLOCK_ID_CHECKSUM {
            // TODO: Checksum checks should probably be handled elsewhere
        } else {
            // Handle proprietary blocks
            let (_, ret) = proprietary_block(data)?;
            proprietary_blocks.push(ret);
        }
    }
    return Ok((
        i,
        SORFile {
            map: map,
            general_parameters: general_parameters,
            supplier_parameters: supplier_parameters,
            fixed_parameters: fixed_parameters,
            key_events: key_events,
            link_parameters: link_parameters,
            data_points: data_points,
            proprietary_blocks: proprietary_blocks,
        },
    ));
}

/// Given an input file and a block header, extracts the bytes for that block only using the map's description of the length of the block.
/// This allows for the parsers in this file to work on a single block at a time without strict ordering, as the SOR file does not
/// require a specific sequence of blocks.
fn extract_block_data<'a>(data: &'a [u8], header: &str) -> &'a [u8] {
    let res = map_block(data);
    let map = res.unwrap().1;
    let mut offset: usize = map.block_size as usize;
    let mut len: usize = 0;
    for block in map.block_info {
        len = block.size as usize;
        // if header.ends_with("\0") {
        //     // Ignore the incoming \0 on the header definition to match the null-stripped line in the parsed block
        //     if block.identifier == &header[0..(header.len() - 1)] {
        //         break;
        //     }
        // } else {
        if block.identifier == header {
            break;
        }
        // }
        offset += block.size as usize;
    }
    // println!("reading from {} to {}", offset, (offset+len));
    // println!("data: {:?}", &data[offset..(offset+len)]);
    return &data[offset..(offset + len)];
}

#[cfg(test)]
fn test_load_file_section(header: &str) -> &[u8] {
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    return extract_block_data(data, header);
}

#[test]
fn test_parse_file() {
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let res = parse_file(data);
    let sor = res.unwrap().1;
    assert_eq!(sor.map.revision_number, 200);
    assert_eq!(sor.data_points.unwrap().number_of_data_points, 30000);
    assert_eq!(sor.key_events.unwrap().number_of_key_events, 3);
    assert_eq!(sor.fixed_parameters.unwrap().date_time_stamp, 1569835674);
}
#[test]
fn test_data_points_block() {
    let data = test_load_file_section(BLOCK_ID_DATAPTS);
    let res = data_points_block(data);
    let parsed = res.unwrap().1;
    assert_eq!(parsed.scale_factors[0].data.len(), 30000);
    assert_eq!(parsed.scale_factors[0].n_points, 30000);
    assert_eq!(parsed.total_number_scale_factors_used, 1);
    assert_eq!(parsed.number_of_data_points, 30000);
}
// This needs test data to actually run.
// #[test]
// fn test_link_parameters_block() {
//     let data = test_load_file_section(BLOCK_ID_LNKPARAMS);
//     let res = link_parameters_block(data);
//     assert_eq!(
//         res.unwrap().1,
//         LinkParameters {
//             number_of_landmarks: 1,
//             landmarks: vec![Landmark {
//                 landmark_number: 0,
//                 landmark_code: "",
//                 landmark_location: 0,
//                 related_event_number: 0,
//                 gps_longitude: 0,
//                 gps_latitude: 0,
//                 fiber_correction_factor_lead_in_fiber: 0,
//                 sheath_marker_entering_landmark: 0,
//                 sheath_marker_leaving_landmark: 0,
//                 units_of_sheath_marks_leaving_landmark: "",
//                 mode_field_diameter_leaving_landmark: 0,
//                 comment: "",
//             }]
//         },
//     );
// }

#[test]
fn test_key_events_block() {
    let data = test_load_file_section(BLOCK_ID_KEYEVENTS);
    let res = key_events_block(data);
    assert_eq!(
        res.unwrap().1,
        KeyEvents {
            number_of_key_events: 3,
            key_events: vec![
                KeyEvent {
                    event_number: 1,
                    event_propogation_time: 0,
                    attenuation_coefficient_lead_in_fiber: 0,
                    event_loss: -215,
                    event_reflectance: -46671,
                    event_code: "1F9999",
                    loss_measurement_technique: "LS",
                    marker_location_1: 0,
                    marker_location_2: 0,
                    marker_location_3: 0,
                    marker_location_4: 0,
                    marker_location_5: 0,
                    comment: " "
                },
                KeyEvent {
                    event_number: 2,
                    event_propogation_time: 532,
                    attenuation_coefficient_lead_in_fiber: 0,
                    event_loss: 374,
                    event_reflectance: 0,
                    event_code: "0F9999",
                    loss_measurement_technique: "LS",
                    marker_location_1: 0,
                    marker_location_2: 0,
                    marker_location_3: 0,
                    marker_location_4: 0,
                    marker_location_5: 0,
                    comment: " "
                }
            ],
            last_key_event: LastKeyEvent {
                event_number: 3,
                event_propogation_time: 182802,
                attenuation_coefficient_lead_in_fiber: 185,
                event_loss: -950,
                event_reflectance: -23027,
                event_code: "2E9999",
                loss_measurement_technique: "LS",
                marker_location_1: 0,
                marker_location_2: 0,
                marker_location_3: 0,
                marker_location_4: 0,
                marker_location_5: 0,
                comment: " ",
                end_to_end_loss: 576,
                end_to_end_marker_position_1: 0,
                end_to_end_marker_position_2: 182809,
                optical_return_loss: 24516,
                optical_return_loss_marker_position_1: 0,
                optical_return_loss_marker_position_2: 182809
            }
        }
    );
}

#[test]
fn test_fixparam_block() {
    let data = test_load_file_section(BLOCK_ID_FXDPARAMS);
    let res = fixed_parameters_block(data);
    assert_eq!(
        res.unwrap().1,
        FixedParametersBlock {
            date_time_stamp: 1569835674,
            units_of_distance: "mt",
            actual_wavelength: 1550,
            acquisition_offset: -2147,
            acquisition_offset_distance: -42,
            total_n_pulse_widths_used: 1,
            pulse_widths_used: vec![30],
            data_spacing: vec![100000],
            n_data_points_for_pulse_widths_used: vec![30000],
            group_index: 146750,
            backscatter_coefficient: 802,
            number_of_averages: 2704,
            averaging_time: 3000,
            acquisition_range: 300000,
            acquisition_range_distance: 6000,
            front_panel_offset: 2147,
            noise_floor_level: 30342,
            noise_floor_scale_factor: 1000,
            power_offset_first_point: 0,
            loss_threshold: 50,
            reflectance_threshold: 65000,
            end_of_fibre_threshold: 3000,
            trace_type: "ST",
            window_coordinate_1: 0,
            window_coordinate_2: 0,
            window_coordinate_3: 0,
            window_coordinate_4: 0
        },
    );
}

#[test]
fn test_supparam_block() {
    let data = test_load_file_section(BLOCK_ID_SUPPARAMS);
    let res = supplier_parameters_block(data);
    assert_eq!(
        res.unwrap().1,
        SupplierParametersBlock {
            supplier_name: "Noyes",
            otdr_mainframe_id: "OFL280C-100",
            otdr_mainframe_sn: "2G14PT7552     ",
            optical_module_id: "0.0.43 ",
            optical_module_sn: " ",
            software_revision: "1.2.04b1011F ",
            other: "Last Calibration Date:  2019-03-25 "
        }
    );
}

#[test]
fn test_genparam_block() {
    let data = test_load_file_section(BLOCK_ID_GENPARAMS);
    let res = general_parameters_block(data);
    assert_eq!(
        res.unwrap().1,
        GeneralParametersBlock {
            language_code: "EN",
            cable_id: "C001 ",
            fiber_id: "009",
            fiber_type: 652,
            nominal_wavelength: 1550,
            originating_location: "CAB000 ",
            terminating_location: "CLS007 ",
            cable_code: " ",
            current_data_flag: "NC",
            user_offset: 24641,
            user_offset_distance: 503,
            operator: " ",
            comment: " "
        }
    );
}

#[test]
fn test_map_block() {
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let res = map_block(data);
    // println!("{:#?}", res.unwrap().1);
    assert_eq!(
        res.unwrap().1,
        MapBlock {
            revision_number: 200,
            block_size: 172,
            block_count: 11,
            block_info: vec![
                BlockInfo {
                    identifier: "GenParams",
                    revision_number: 200,
                    size: 58
                },
                BlockInfo {
                    identifier: "SupParams",
                    revision_number: 200,
                    size: 104
                },
                BlockInfo {
                    identifier: "FxdParams",
                    revision_number: 200,
                    size: 92
                },
                BlockInfo {
                    identifier: "FodParams",
                    revision_number: 200,
                    size: 266
                },
                BlockInfo {
                    identifier: "KeyEvents",
                    revision_number: 200,
                    size: 166
                },
                BlockInfo {
                    identifier: "Fod02Params",
                    revision_number: 200,
                    size: 38
                },
                BlockInfo {
                    identifier: "Fod04Params",
                    revision_number: 200,
                    size: 166
                },
                BlockInfo {
                    identifier: "Fod03Params",
                    revision_number: 200,
                    size: 26
                },
                BlockInfo {
                    identifier: "DataPts",
                    revision_number: 200,
                    size: 60020
                },
                BlockInfo {
                    identifier: "Cksum",
                    revision_number: 200,
                    size: 8
                }
            ]
        }
    );
}

#[test]
fn test_null_terminated_chunk() {
    let test_str = "abcdef\0";
    let res = null_terminated_chunk(test_str.as_bytes());
    let data = res.unwrap();
    assert_eq!(data.0, "".as_bytes()); // make sure we've consumed the null
    assert_eq!(data.1, "abcdef".as_bytes());
}
