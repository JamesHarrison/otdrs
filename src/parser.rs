use crate::types::{
    BlockInfo, DataPoints, DataPointsAtScaleFactor, FixedParametersBlock, GeneralParametersBlock,
    KeyEvent, KeyEvents, Landmark, LastKeyEvent, LinkParameters, MapBlock, ProprietaryBlock,
    SORFile, SupplierParametersBlock,
};
use nom::{
    bytes::complete::{tag, take, take_until},
    combinator::map_res,
    error::{Error, ErrorKind},
    multi::count,
    number::complete::{le_i16, le_i32, le_u16, le_u32},
    sequence::terminated,
    Err, IResult, Parser,
};
use std::str;

/// Block header string for the map block
pub const BLOCK_ID_MAP: &str = "Map";
/// Block header string for the general parameters block
pub const BLOCK_ID_GENPARAMS: &str = "GenParams";
/// Block header string for the supplier parameters block
pub const BLOCK_ID_SUPPARAMS: &str = "SupParams";
/// Block header string for the fixed parameters block
pub const BLOCK_ID_FXDPARAMS: &str = "FxdParams";
/// Block header string for the key events block
pub const BLOCK_ID_KEYEVENTS: &str = "KeyEvents";
/// Block header string for the link parameters block
pub const BLOCK_ID_LNKPARAMS: &str = "LnkParams";
/// Block header string for the data points block
pub const BLOCK_ID_DATAPTS: &str = "DataPts";
/// Block header string for the checksum block
pub const BLOCK_ID_CHECKSUM: &str = "Cksum";

/// Parses to look for a block header, null-terminated, and returns the bytes
/// (sans null character)
fn block_header<'a>(i: &'a [u8], header: &str) -> IResult<&'a [u8], &'a [u8]> {
    terminated(tag(header), tag("\0")).parse(i)
}

/// Parse a block information sequence within the map block
fn map_block_info(i: &[u8]) -> IResult<&[u8], BlockInfo> {
    let (i, header) = null_terminated_str(i)?;
    let (i, revision_number) = le_u16(i)?;
    let (i, size) = le_i32(i)?;
    Ok((
        i,
        BlockInfo {
            identifier: String::from(header),
            revision_number,
            size,
        },
    ))
}

/// Parses the map block in a SOR file, which contains information about the
/// location of all blocks in the file
pub fn map_block(i: &[u8]) -> IResult<&[u8], MapBlock> {
    let (i, _) = block_header(i, BLOCK_ID_MAP)?;
    let (i, revision_number) = le_u16(i)?;
    let (i, block_size) = le_i32(i)?;
    let (i, block_count) = le_i16(i)?;
    let blocks_to_read = block_count.checked_sub(1);
    if blocks_to_read == None {
        return Err(Err::Failure(Error {
            input: i,
            code: ErrorKind::Fix,
        }));
    }
    let (i, block_info) = count(map_block_info, blocks_to_read.unwrap() as usize).parse(i)?;
    Ok((
        i,
        MapBlock {
            revision_number,
            block_count,
            block_size,
            block_info,
        },
    ))
}

/// Parse an incoming byte sequence until a null character is found and return
/// the bytes to that point, consuming the null
fn null_terminated_chunk(i: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(take_until("\0"), tag("\0")).parse(i)
}

/// Parse a null-terminated variable length string
fn null_terminated_str(i: &[u8]) -> IResult<&[u8], &str> {
    map_res(null_terminated_chunk, str::from_utf8).parse(i)
}

/// Parse a fixed-length string of the given number of bytes
fn fixed_length_str(i: &[u8], n_bytes: usize) -> IResult<&[u8], &str> {
    map_res(take(n_bytes), str::from_utf8).parse(i)
}

/// Parse the general parameters block, which contains acquisition information
/// as well as locations/identifiers.
pub fn general_parameters_block(i: &[u8]) -> IResult<&[u8], GeneralParametersBlock> {
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
    Ok((
        i,
        GeneralParametersBlock {
            language_code: String::from(language_code),
            cable_id: String::from(cable_id),
            fiber_id: String::from(fiber_id),
            fiber_type,
            nominal_wavelength,
            originating_location: String::from(originating_location),
            terminating_location: String::from(terminating_location),
            cable_code: String::from(cable_code),
            current_data_flag: String::from(current_data_flag),
            user_offset,
            user_offset_distance,
            operator: String::from(operator),
            comment: String::from(comment),
        },
    ))
}

/// Parse the supplier parameters block, which contains information about the
/// OTDR equipment used.
pub fn supplier_parameters_block(i: &[u8]) -> IResult<&[u8], SupplierParametersBlock> {
    let (i, _) = block_header(i, BLOCK_ID_SUPPARAMS)?;
    let (i, supplier_name) = null_terminated_str(i)?;
    let (i, otdr_mainframe_id) = null_terminated_str(i)?;
    let (i, otdr_mainframe_sn) = null_terminated_str(i)?;
    let (i, optical_module_id) = null_terminated_str(i)?;
    let (i, optical_module_sn) = null_terminated_str(i)?;
    let (i, software_revision) = null_terminated_str(i)?;
    let (i, other) = null_terminated_str(i)?;
    Ok((
        i,
        SupplierParametersBlock {
            supplier_name: String::from(supplier_name),
            otdr_mainframe_id: String::from(otdr_mainframe_id),
            otdr_mainframe_sn: String::from(otdr_mainframe_sn),
            optical_module_id: String::from(optical_module_id),
            optical_module_sn: String::from(optical_module_sn),
            software_revision: String::from(software_revision),
            other: String::from(other),
        },
    ))
}

/// Parse the fixed paramters block, which contains most of the information
/// required to interpret the stored data.
pub fn fixed_parameters_block(i: &[u8]) -> IResult<&[u8], FixedParametersBlock> {
    let (i, _) = block_header(i, BLOCK_ID_FXDPARAMS)?;
    let (i, date_time_stamp) = le_u32(i)?;
    let (i, units_of_distance) = fixed_length_str(i, 2)?;
    let (i, actual_wavelength) = le_i16(i)?;
    let (i, acquisition_offset) = le_i32(i)?;
    let (i, acquisition_offset_distance) = le_i32(i)?;
    let (i, total_n_pulse_widths_used) = le_i16(i)?;
    let pulse_width_count: usize = total_n_pulse_widths_used as usize;
    let (i, pulse_widths_used) = count(le_i16, pulse_width_count).parse(i)?;
    //println!("{}, {:?}", pulse_width_count, pulse_widths_used);
    let (i, data_spacing) = count(le_i32, pulse_width_count).parse(i)?;
    let (i, n_data_points_for_pulse_widths_used) = count(le_i32, pulse_width_count).parse(i)?;
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
    Ok((
        i,
        FixedParametersBlock {
            date_time_stamp,
            units_of_distance: String::from(units_of_distance),
            actual_wavelength,
            acquisition_offset,
            acquisition_offset_distance,
            total_n_pulse_widths_used,
            pulse_widths_used,
            data_spacing,
            n_data_points_for_pulse_widths_used,
            group_index,
            backscatter_coefficient,
            number_of_averages,
            averaging_time,
            acquisition_range,
            acquisition_range_distance,
            front_panel_offset,
            noise_floor_level,
            noise_floor_scale_factor,
            power_offset_first_point,
            loss_threshold,
            reflectance_threshold,
            end_of_fibre_threshold,
            trace_type: String::from(trace_type),
            window_coordinate_1,
            window_coordinate_2,
            window_coordinate_3,
            window_coordinate_4,
        },
    ))
}

fn parse_key_event_common(i: &[u8]) -> IResult<&[u8], KeyEvent> {
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
    Ok((
        i,
        KeyEvent {
            event_number,
            event_propogation_time,
            attenuation_coefficient_lead_in_fiber,
            event_loss,
            event_reflectance,
            event_code: String::from(event_code),
            loss_measurement_technique: String::from(loss_measurement_technique),
            marker_location_1,
            marker_location_2,
            marker_location_3,
            marker_location_4,
            marker_location_5,
            comment: String::from(comment),
        },
    ))
}
/// Parse any key event, except for the final key event, which is parsed with
/// last_key_event as it differs structurally
pub fn key_event(i: &[u8]) -> IResult<&[u8], KeyEvent> {
    parse_key_event_common(i)
}

/// Parse the final key event in the key events block, which contains much of
/// the end-to-end loss definitions
pub fn last_key_event(i: &[u8]) -> IResult<&[u8], LastKeyEvent> {
    let (i, common) = parse_key_event_common(i)?;
    let (i, end_to_end_loss) = le_i32(i)?;
    let (i, end_to_end_marker_position_1) = le_i32(i)?;
    let (i, end_to_end_marker_position_2) = le_i32(i)?;
    let (i, optical_return_loss) = le_u16(i)?;
    let (i, optical_return_loss_marker_position_1) = le_i32(i)?;
    let (i, optical_return_loss_marker_position_2) = le_i32(i)?;

    Ok((
        i,
        LastKeyEvent {
            event_number: common.event_number,
            event_propogation_time: common.event_propogation_time,
            attenuation_coefficient_lead_in_fiber: common.attenuation_coefficient_lead_in_fiber,
            event_loss: common.event_loss,
            event_reflectance: common.event_reflectance,
            event_code: common.event_code,
            loss_measurement_technique: common.loss_measurement_technique,
            marker_location_1: common.marker_location_1,
            marker_location_2: common.marker_location_2,
            marker_location_3: common.marker_location_3,
            marker_location_4: common.marker_location_4,
            marker_location_5: common.marker_location_5,
            comment: common.comment,
            end_to_end_loss,
            end_to_end_marker_position_1,
            end_to_end_marker_position_2,
            optical_return_loss,
            optical_return_loss_marker_position_1,
            optical_return_loss_marker_position_2,
        },
    ))
}

/// Parse the key events block
pub fn key_events_block(i: &[u8]) -> IResult<&[u8], KeyEvents> {
    let (i, _) = block_header(i, BLOCK_ID_KEYEVENTS)?;
    let (i, number_of_key_events) = le_i16(i)?;
    let (n_key_events, overflowed) = number_of_key_events.overflowing_sub(1);
    if overflowed {
        return Err(Err::Failure(Error {
            input: i,
            code: ErrorKind::Fix,
        }));
    }
    let (i, key_events) = count(key_event, n_key_events as usize).parse(i)?;
    let (i, last_key_event) = last_key_event(i)?;
    Ok((
        i,
        KeyEvents {
            number_of_key_events,
            key_events,
            last_key_event,
        },
    ))
}

// TODO: Test this, no test data to hand so this is probably correct
/// Parse a landmark from the link parameters block
pub fn landmark(i: &[u8]) -> IResult<&[u8], Landmark> {
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
    Ok((
        i,
        Landmark {
            landmark_number,
            landmark_code: String::from(landmark_code),
            landmark_location,
            related_event_number,
            gps_longitude,
            gps_latitude,
            fiber_correction_factor_lead_in_fiber,
            sheath_marker_entering_landmark,
            sheath_marker_leaving_landmark,
            units_of_sheath_marks_leaving_landmark: String::from(
                units_of_sheath_marks_leaving_landmark,
            ),
            mode_field_diameter_leaving_landmark,
            comment: String::from(comment),
        },
    ))
}

// TODO: Test this, no test data to hand so this is probably correct
/// Extract link parameters and encoded landmarks from the LinkParams block.
pub fn link_parameters_block(i: &[u8]) -> IResult<&[u8], LinkParameters> {
    let (i, _) = block_header(i, BLOCK_ID_LNKPARAMS)?;
    let (i, number_of_landmarks) = le_i16(i)?;
    let (i, landmarks) = count(landmark, number_of_landmarks as usize).parse(i)?;
    Ok((
        i,
        LinkParameters {
            number_of_landmarks,
            landmarks,
        },
    ))
}

/// Parse the data points at a defined scale factor within the DataPoints block
pub fn data_points_at_scale_factor(i: &[u8]) -> IResult<&[u8], DataPointsAtScaleFactor> {
    let (i, n_points) = le_i32(i)?;
    let (i, scale_factor) = le_i16(i)?;
    let (i, data) = count(le_u16, n_points as usize).parse(i)?;
    Ok((
        i,
        DataPointsAtScaleFactor {
            n_points,
            scale_factor,
            data,
        },
    ))
}

/// Parse the DataPoints block and extract all the points for each scale factor
pub fn data_points_block(i: &[u8]) -> IResult<&[u8], DataPoints> {
    let (i, _) = block_header(i, BLOCK_ID_DATAPTS)?;
    let (i, number_of_data_points) = le_i32(i)?;
    let (i, total_number_scale_factors_used) = le_i16(i)?;
    let (i, scale_factors) = count(
        data_points_at_scale_factor,
        total_number_scale_factors_used as usize,
    )
    .parse(i)?;
    Ok((
        i,
        DataPoints {
            number_of_data_points,
            total_number_scale_factors_used,
            scale_factors,
        },
    ))
}
/// Parse the header string from a proprietary block, and return the remaining
/// data for external parsers.
pub fn proprietary_block(i: &[u8]) -> IResult<&[u8], ProprietaryBlock> {
    let (data, header) = null_terminated_str(i)?;
    Ok((
        &[],
        ProprietaryBlock {
            header: String::from(header),
            data: data.to_vec(),
        },
    ))
}

/// Parse a complete SOR file, extracting all known and proprietary blocks to a
/// SORFile struct.
pub fn parse_file<'a>(i: &'a [u8]) -> IResult<&'a [u8], SORFile> {
    let (mut rest, map) = map_block(i)?;

    let mut total_size: u64 = 0;
    for block in &map.block_info {
        if block.size < 0 {
            return Err(Err::Failure(Error { input: i, code: ErrorKind::Verify }));
        }
        total_size += block.size as u64;
    }

    if total_size > rest.len() as u64 {
        return Err(Err::Failure(Error { input: i, code: ErrorKind::Verify }));
    }

    let mut general_parameters: Option<GeneralParametersBlock> = None;
    let mut supplier_parameters: Option<SupplierParametersBlock> = None;
    let mut fixed_parameters: Option<FixedParametersBlock> = None;
    let mut key_events: Option<KeyEvents> = None;
    let link_parameters: Option<LinkParameters> = None;
    let mut data_points: Option<DataPoints> = None;
    let mut proprietary_blocks: Vec<ProprietaryBlock> = Vec::new();

    for block in &map.block_info {
        let (r, data) = take(block.size as usize)(rest)?;
        rest = r;

        match block.identifier.as_str() {
            BLOCK_ID_SUPPARAMS => {
                let (_, ret) = supplier_parameters_block(data)?;
                supplier_parameters = Some(ret);
            }
            BLOCK_ID_GENPARAMS => {
                let (_, ret) = general_parameters_block(data)?;
                general_parameters = Some(ret);
            }
            BLOCK_ID_FXDPARAMS => {
                let (_, ret) = fixed_parameters_block(data)?;
                fixed_parameters = Some(ret);
            }
            BLOCK_ID_KEYEVENTS => {
                let (_, ret) = key_events_block(data)?;
                key_events = Some(ret);
            }
            BLOCK_ID_LNKPARAMS => {
                // Unimplemented due to lack of test data
            }
            BLOCK_ID_DATAPTS => {
                let (_, ret) = data_points_block(data)?;
                data_points = Some(ret);
            }
            BLOCK_ID_CHECKSUM => {
                // TODO: Checksum checks should probably be handled elsewhere
            }
            _ => {
                // Handle proprietary blocks
                let (_, ret) = proprietary_block(data)?;
                proprietary_blocks.push(ret);
            }
        }
    }

    Ok((
        i,
        SORFile {
            map,
            general_parameters,
            supplier_parameters,
            fixed_parameters,
            key_events,
            link_parameters,
            data_points,
            proprietary_blocks,
        },
    ))
}

#[test]
fn test_parse_file() {
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let res = parse_file(data);
    let sor = res.unwrap().1;
    let fp = sor.fixed_parameters.unwrap();
    assert_eq!(sor.map.revision_number, 200);
    assert_eq!(sor.general_parameters.unwrap().nominal_wavelength, 1550);
    assert_eq!(fp.pulse_widths_used, vec![30]);
    assert_eq!(sor.data_points.unwrap().number_of_data_points, 30000);
    assert_eq!(sor.key_events.unwrap().number_of_key_events, 3);
    assert_eq!(fp.date_time_stamp, 1569835674);
    assert_eq!(fp.averaging_time, 3000);
    assert_eq!(fp.number_of_averages, 2704);
}

#[test]
fn test_parse_anritsu_file() {
    let data = include_bytes!("../data/example3-anritsu-accessmastermt9085.sor");
    let res = parse_file(data);
    let sor = res.unwrap().1;
    let fp = sor.fixed_parameters.unwrap();
    assert_eq!(sor.map.revision_number, 200);
    assert_eq!(sor.general_parameters.unwrap().nominal_wavelength, 1310);
    assert_eq!(sor.data_points.unwrap().number_of_data_points, 20001);
    assert_eq!(sor.key_events.unwrap().number_of_key_events, 3);
    assert_eq!(fp.date_time_stamp, 1592094230);
    assert_eq!(fp.averaging_time, 30);
    assert_eq!(fp.number_of_averages, 15360);
}

#[test]
fn test_parse_exfo_ftb4_file() {
    let data = include_bytes!("../data/example4-exfo-ftb4ftbx730c-mfdgainer-1310nm.sor");
    let res = parse_file(data);
    let sor = res.unwrap().1;
    let fp = sor.fixed_parameters.unwrap();
    assert_eq!(sor.map.revision_number, 200);
    assert_eq!(sor.general_parameters.unwrap().nominal_wavelength, 1310);
    assert_eq!(sor.data_points.unwrap().number_of_data_points, 25903);
    assert_eq!(sor.key_events.unwrap().number_of_key_events, 9);
    assert_eq!(fp.date_time_stamp, 1593101318);
    assert_eq!(fp.averaging_time, 7);
    assert_eq!(fp.number_of_averages, 4563);
}

#[test]
fn test_data_points_block() {
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let sor = parse_file(data).unwrap().1;
    let parsed = sor.data_points.unwrap();
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
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let sor = parse_file(data).unwrap().1;
    assert_eq!(
        sor.key_events.unwrap(),
        KeyEvents {
            number_of_key_events: 3,
            key_events: vec![
                KeyEvent {
                    event_number: 1,
                    event_propogation_time: 0,
                    attenuation_coefficient_lead_in_fiber: 0,
                    event_loss: -215,
                    event_reflectance: -46671,
                    event_code: "1F9999".to_owned(),
                    loss_measurement_technique: "LS".to_owned(),
                    marker_location_1: 0,
                    marker_location_2: 0,
                    marker_location_3: 0,
                    marker_location_4: 0,
                    marker_location_5: 0,
                    comment: " ".to_owned()
                },
                KeyEvent {
                    event_number: 2,
                    event_propogation_time: 532,
                    attenuation_coefficient_lead_in_fiber: 0,
                    event_loss: 374,
                    event_reflectance: 0,
                    event_code: "0F9999".to_owned(),
                    loss_measurement_technique: "LS".to_owned(),
                    marker_location_1: 0,
                    marker_location_2: 0,
                    marker_location_3: 0,
                    marker_location_4: 0,
                    marker_location_5: 0,
                    comment: " ".to_owned()
                }
            ],
            last_key_event: LastKeyEvent {
                event_number: 3,
                event_propogation_time: 182802,
                attenuation_coefficient_lead_in_fiber: 185,
                event_loss: -950,
                event_reflectance: -23027,
                event_code: "2E9999".to_owned(),
                loss_measurement_technique: "LS".to_owned(),
                marker_location_1: 0,
                marker_location_2: 0,
                marker_location_3: 0,
                marker_location_4: 0,
                marker_location_5: 0,
                comment: " ".to_owned(),
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
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let sor = parse_file(data).unwrap().1;
    assert_eq!(
        sor.fixed_parameters.unwrap(),
        FixedParametersBlock {
            date_time_stamp: 1569835674,
            units_of_distance: "mt".to_owned(),
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
            trace_type: "ST".to_owned(),
            window_coordinate_1: 0,
            window_coordinate_2: 0,
            window_coordinate_3: 0,
            window_coordinate_4: 0
        },
    );
}

#[test]
fn test_supparam_block() {
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let sor = parse_file(data).unwrap().1;
    assert_eq!(
        sor.supplier_parameters.unwrap(),
        SupplierParametersBlock {
            supplier_name: "Noyes".to_owned(),
            otdr_mainframe_id: "OFL280C-100".to_owned(),
            otdr_mainframe_sn: "2G14PT7552     ".to_owned(),
            optical_module_id: "0.0.43 ".to_owned(),
            optical_module_sn: " ".to_owned(),
            software_revision: "1.2.04b1011F ".to_owned(),
            other: "Last Calibration Date:  2019-03-25 ".to_owned()
        }
    );
}

#[test]
fn test_genparam_block() {
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let sor = parse_file(data).unwrap().1;
    assert_eq!(
        sor.general_parameters.unwrap(),
        GeneralParametersBlock {
            language_code: "EN".to_owned(),
            cable_id: "C001 ".to_owned(),
            fiber_id: "009".to_owned(),
            fiber_type: 652,
            nominal_wavelength: 1550,
            originating_location: "CAB000 ".to_owned(),
            terminating_location: "CLS007 ".to_owned(),
            cable_code: " ".to_owned(),
            current_data_flag: "NC".to_owned(),
            user_offset: 24641,
            user_offset_distance: 503,
            operator: " ".to_owned(),
            comment: " ".to_owned()
        }
    );
}

#[test]
fn test_map_block() {
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let res = map_block(data);
    // println!("{:#?}".to_owned(), res.unwrap().1);
    assert_eq!(
        res.unwrap().1,
        MapBlock {
            revision_number: 200,
            block_size: 172,
            block_count: 11,
            block_info: vec![
                BlockInfo {
                    identifier: "GenParams".to_owned(),
                    revision_number: 200,
                    size: 58
                },
                BlockInfo {
                    identifier: "SupParams".to_owned(),
                    revision_number: 200,
                    size: 104
                },
                BlockInfo {
                    identifier: "FxdParams".to_owned(),
                    revision_number: 200,
                    size: 92
                },
                BlockInfo {
                    identifier: "FodParams".to_owned(),
                    revision_number: 200,
                    size: 266
                },
                BlockInfo {
                    identifier: "KeyEvents".to_owned(),
                    revision_number: 200,
                    size: 166
                },
                BlockInfo {
                    identifier: "Fod02Params".to_owned(),
                    revision_number: 200,
                    size: 38
                },
                BlockInfo {
                    identifier: "Fod04Params".to_owned(),
                    revision_number: 200,
                    size: 166
                },
                BlockInfo {
                    identifier: "Fod03Params".to_owned(),
                    revision_number: 200,
                    size: 26
                },
                BlockInfo {
                    identifier: "DataPts".to_owned(),
                    revision_number: 200,
                    size: 60020
                },
                BlockInfo {
                    identifier: "Cksum".to_owned(),
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
