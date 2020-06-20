use crate::{BlockInfo, FixedParametersBlock, SupplierParametersBlock, GeneralParametersBlock, MapBlock, SORFile};
use nom::{
    bytes::complete::{take, tag, take_until},
    sequence::terminated,
    multi::{many_m_n},
    number::complete::{le_i16, le_i32, le_u16, le_u32},
    IResult,
};
use std::str;
const BLOCK_ID_MAP: &str = "Map\0";
const BLOCK_ID_GENPARAMS: &str = "GenParams\0";
const BLOCK_ID_SUPPARAMS: &str = "SupParams\0";
const BLOCK_ID_FXDPARAMS: &str = "FxdParams\0";
const BLOCK_ID_KEYEVENTS: &str = "KeyEvents\0";
const BLOCK_ID_LNKPARAMS: &str = "LnkParams\0";
const BLOCK_ID_DATAPTS: &str = "DataPts\0";
const BLOCK_ID_CHECKSUM: &str = "Cksum\0";
// const BLOCK_ID_LIST: &[&str] = &[BLOCK_ID_MAP, BLOCK_ID_GENPARAMS, BLOCK_ID_SUPPARAMS, BLOCK_ID_FXDPARAMS, BLOCK_ID_KEYEVENTS, BLOCK_ID_LNKPARAMS, BLOCK_ID_DATAPTS, BLOCK_ID_CHECKSUM];
// fn map_block_revision_no(i: &[u8]) -> IResult<&[u8], i16> {
//     le_i16(i)
// }

fn block_header(i: &[u8]) -> IResult<&[u8], &[u8]> {
    null_terminated_chunk(i)
    // alt((
    //     tag(BLOCK_ID_MAP),
    //     tag(BLOCK_ID_GENPARAMS),
    //     tag(BLOCK_ID_SUPPARAMS),
    //     tag(BLOCK_ID_FXDPARAMS),
    //     tag(BLOCK_ID_KEYEVENTS),
    //     tag(BLOCK_ID_LNKPARAMS),
    //     tag(BLOCK_ID_DATAPTS),
    //     tag(BLOCK_ID_CHECKSUM),
    // ))(i)
}

fn map_block_info(i: &[u8]) -> IResult<&[u8], BlockInfo<'_>> {
    let (i, header) = block_header(i)?;
    let header_str = parse_string(header);
    let (i, revision_number) = le_u16(i)?;
    let (i, size) = le_i32(i)?;
    return Ok((
        i,
        BlockInfo {
            identifier: header_str,
            revision_number: revision_number,
            size: size,
        },
    ));
}

pub fn map_block(i: &[u8]) -> IResult<&[u8], MapBlock> {
    let (i, _) = tag(BLOCK_ID_MAP)(i)?;
    let (i, revision_number) = le_u16(i)?;
    let (i, block_size) = le_i32(i)?;
    let (i, block_count) = le_i16(i)?;
    let blocks_to_read:usize = (block_count-1) as usize;
    let (i, blocks) = many_m_n(blocks_to_read, blocks_to_read, map_block_info)(i)?;
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

fn null_terminated_chunk(i: &[u8]) -> IResult<&[u8], &[u8]> {
    terminated(take_until("\0"), tag("\0"))(i)
}

fn parse_string(input: &[u8]) -> &str {
    if input.len() > 0 {
        return str::from_utf8(input).unwrap();
    } else {
        return "";
    }
}

pub fn null_terminated_str(i: &[u8]) -> IResult<&[u8], &str> {
    let (i, seg) = null_terminated_chunk(i)?;
    let string = parse_string(seg);
    return Ok((i, string));
}
pub fn fixed_length_str(i: &[u8], len: usize) -> IResult<&[u8], &str> {
    if len == 2 {
        let (i, seg) = take(2u8)(i)?;
        let string = parse_string(seg);
        return Ok((i, string));
    } else if len == 6 {
        let (i, seg) = take(6u8)(i)?;
        let string = parse_string(seg);
        return Ok((i, string));
    } else {
        // TODO: it would be nice to make this less crappy, this whole if-then-panic thing is a bit crap
        panic!("This length of fixed-length string is not implemented")
    }
}

pub fn general_parameters_block<'a>(i: &[u8]) -> IResult<&[u8], GeneralParametersBlock<'_>> {
    let (i, _) = tag(BLOCK_ID_GENPARAMS)(i)?;
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

pub fn supplier_parameters_block<'a>(i: &[u8]) -> IResult<&[u8], SupplierParametersBlock<'_>> {
    let (i, _) = tag(BLOCK_ID_SUPPARAMS)(i)?;
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

pub fn fixed_parameters_block<'a>(i: &[u8]) -> IResult<&[u8], FixedParametersBlock<'_>> {
    let (i, _) = tag(BLOCK_ID_FXDPARAMS)(i)?;
    let (i, date_time_stamp) = le_u32(i)?;
    let (i, units_of_distance) = fixed_length_str(i, 2)?;
    let (i, actual_wavelength) = le_i16(i)?;
    let (i, acquisition_offset) = le_i32(i)?;
    let (i, acquisition_offset_distance) = le_i32(i)?;
    let (i, total_n_pulse_widths_used) = le_i16(i)?;
    let (i, pulse_widths_used) = many_m_n(total_n_pulse_widths_used as usize, total_n_pulse_widths_used as usize, le_i16)(i)?;
    let (i, data_spacing) = le_i32(i)?;
    let (i, n_data_points_for_pulse_widths_used) = many_m_n(total_n_pulse_widths_used as usize, total_n_pulse_widths_used as usize, le_i32)(i)?;
    let (i, group_index) = le_i32(i)?;
    let (i, backscatter_coefficient) = le_i32(i)?;
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

pub fn parse_file<'a>(i: &[u8]) -> IResult<&[u8], SORFile<'_>> {
    let (i, map) = map_block(i)?;
    let (i, general_parameters) = general_parameters_block(i)?;
    
    let (i, supplier_parameters) = supplier_parameters_block(i)?;
    let (i, fixed_parameters) = fixed_parameters_block(i)?;
    
    return Ok((
        i,
        SORFile {
            map: map,
            general_parameters: general_parameters,
            supplier_parameters: supplier_parameters,
            fixed_parameters: fixed_parameters,
        },
    ));
}

#[cfg(test)]
fn test_load_file_section(header: &str) -> &[u8] {
    let data = include_bytes!("../data/example2-exfo-maxtester730c.sor");
    let res = map_block(data);
    let map = res.unwrap().1;
    let mut offset: usize = map.block_size as usize;
    let mut len: usize = 0;
    for block in map.block_info {
        len = block.size as usize;
        // Ignore the incoming \0 on the header definition to match the null-stripped line in the parsed block
        if block.identifier == &header[0..(header.len()-1)] {
            break;
        }
        offset += block.size as usize;
    }
    // println!("reading from {} to {}", offset, (offset+len));
    // println!("data: {:?}", &data[offset..(offset+len)]);
    return &data[offset..(offset+len)];
}

#[test]
fn test_parse_file() {
    let data = include_bytes!("../data/example2-exfo-maxtester730c.sor");
    let res = parse_file(data);
    let sor = res.unwrap().1;
    assert_eq!(sor.map.revision_number, 200);
}

#[test]
fn test_null_terminated_chunk() {
    let test_str = "abcdef\0";
    let res = null_terminated_chunk(test_str.as_bytes());
    let data = res.unwrap();
    assert_eq!(data.0, "".as_bytes()); // make sure we've consumed the null
    assert_eq!(data.1, "abcdef".as_bytes());
}

#[test]
fn test_fixparam_block() {
    let data = test_load_file_section(BLOCK_ID_FXDPARAMS);
    let res = fixed_parameters_block(data);
    assert_eq!(
        res.unwrap().1,
        FixedParametersBlock {
            date_time_stamp:0,
            units_of_distance: "",
            actual_wavelength: 0,
            acquisition_offset: 0,
            acquisition_offset_distance: 0,
            total_n_pulse_widths_used: 0,
            pulse_widths_used: vec![0],
            data_spacing: 0,
            n_data_points_for_pulse_widths_used: vec![0],
            group_index: 0,
            backscatter_coefficient: 0,
            number_of_averages: 0,
            averaging_time: 0,
            acquisition_range: 0,
            acquisition_range_distance: 0,
            front_panel_offset: 0,
            noise_floor_level: 0,
            noise_floor_scale_factor: 0,
            power_offset_first_point: 0,
            loss_threshold: 0,
            reflectance_threshold: 0,
            end_of_fibre_threshold: 0,
            trace_type: "",
            window_coordinate_1: 0,
            window_coordinate_2: 0,
            window_coordinate_3: 0,
            window_coordinate_4: 0,
        },
    );
}

#[test]
fn test_supparam_block() {
    let data = test_load_file_section(BLOCK_ID_SUPPARAMS);
    let res = supplier_parameters_block(data);
    assert_eq!(
        res.unwrap().1,
        SupplierParametersBlock { supplier_name: "Noyes", otdr_mainframe_id: "OFL280C-100", otdr_mainframe_sn: "2G14PT7552     ", optical_module_id: "0.0.43 ", optical_module_sn: " ", software_revision: "1.2.04b1011F ", other: "Last Calibration Date:  2019-03-25 " }
    );
}


#[test]
fn test_genparam_block() {
    let data = test_load_file_section(BLOCK_ID_GENPARAMS);
    let res = general_parameters_block(data);
    assert_eq!(
        res.unwrap().1,
        GeneralParametersBlock { language_code: "EN", cable_id: "C001 ", fiber_id: "009", fiber_type: 652, nominal_wavelength: 1550, originating_location: "CAB000 ", terminating_location: "CLS007 ", cable_code: " ", current_data_flag: "NC", user_offset: 24641, user_offset_distance: 503, operator: " ", comment: " " }
    );
}

#[test]
fn test_map_block() {
    let data = include_bytes!("../data/example1-noyes-ofl280.sor");
    let res = map_block(data);
    // println!("{:#?}", res.unwrap().1);
    assert_eq!(
        res.unwrap().1,
        MapBlock { revision_number: 200, block_size: 172, block_count: 11, block_info: vec![BlockInfo { identifier: "GenParams", revision_number: 200, size: 58 }, BlockInfo { identifier: "SupParams", revision_number: 200, size: 104 }, BlockInfo { identifier: "FxdParams", revision_number: 200, size: 92 }, BlockInfo { identifier: "FodParams", revision_number: 200, size: 266 }, BlockInfo { identifier: "KeyEvents", revision_number: 200, size: 166 }, BlockInfo { identifier: "Fod02Params", revision_number: 200, size: 38 }, BlockInfo { identifier: "Fod04Params", revision_number: 200, size: 166 }, BlockInfo { identifier: "Fod03Params", revision_number: 200, size: 26 }, BlockInfo { identifier: "DataPts", revision_number: 200, size: 60020 }, BlockInfo { identifier: "Cksum", revision_number: 200, size: 8 }] }
    );
}
