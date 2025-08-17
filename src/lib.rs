pub mod parser;
/// Base library for otdrs
pub mod types;
use crate::types::{BlockInfo, MapBlock, ProprietaryBlock, SORFile};
use crc::{Crc, CRC_16_IBM_3740};
use std::fmt;

// Include Python if feature enabled
#[cfg(feature = "python")]
pub mod python;

#[derive(Debug, PartialEq, Eq)]
pub enum WriteError {
    MissingMandatoryBlock(String),
    MissingBlockInfo(String),
    Utf8EncodingError,
    FixedLengthStringMismatchError,
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            WriteError::MissingMandatoryBlock(block) => {
                write!(f, "Missing mandatory block: {}", block)
            }
            WriteError::MissingBlockInfo(block) => {
                write!(f, "BlockInfo block is missing for one of your blocks in the Map!: {}", block)
            }
            WriteError::Utf8EncodingError => write!(f, "A character in a fixed-length string appears to be UTF-8 and require more than one byte to encode, which is not permitted in the standard."),
            WriteError::FixedLengthStringMismatchError => write!(f, "Fixed-length string exceeds the length specified for the string")
        }
    }
}

impl std::error::Error for WriteError {}

fn null_terminated_str(b: &mut Vec<u8>, s: &str) {
    b.extend(s.as_bytes());
    b.push(0x0);
}

fn fixed_length_str(b: &mut Vec<u8>, s: &str, len: usize) -> Result<(), WriteError> {
    if s.len() > len {
        return Err(WriteError::FixedLengthStringMismatchError);
    }
    let mut bytes: Vec<u8> = Vec::with_capacity(len);
    for c in s.chars() {
        let mut byte = [0; 1];
        if c.len_utf8() > 1 {
            return Err(WriteError::Utf8EncodingError);
        }
        c.encode_utf8(&mut byte);
        bytes.push(byte[0]);
    }
    // Handle scenarios in which the provided string is smaller than the specified length by null padding
    bytes.resize(len, 0);
    b.extend(bytes);
    Ok(())
}

// Write any integer (e.g., i16, u16, i32, u32, etc.) in little-endian form.
fn le_integer<T>(b: &mut Vec<u8>, i: T)
where
    T: Copy + Into<i64> + 'static,
{
    let mut v = i.into() as u64;
    let n = core::mem::size_of::<T>();
    // Efficiency: allocate the capacity we need
    b.reserve(n);
    // Write out bytes in LE order
    for _ in 0..n {
        b.push((v & 0xFF) as u8);
        v >>= 8;
    }
}


impl SORFile {
    pub fn to_bytes(&self) -> Result<Vec<u8>, WriteError> {
        let mut bytes: Vec<u8> = Vec::new();
        let mut new_map = MapBlock {
            revision_number: self.map.revision_number,
            block_count: 0,
            block_size: 0,
            block_info: Vec::new(),
        };

        // Mandatory blocks
        for block_id in [
            parser::BLOCK_ID_GENPARAMS,
            parser::BLOCK_ID_FXDPARAMS,
            parser::BLOCK_ID_KEYEVENTS,
            parser::BLOCK_ID_DATAPTS,
        ] {
            let block_bytes = match block_id {
                parser::BLOCK_ID_GENPARAMS => self.gen_general_parameters(),
                parser::BLOCK_ID_FXDPARAMS => self.gen_fixed_parameters(),
                parser::BLOCK_ID_KEYEVENTS => self.gen_key_events(),
                parser::BLOCK_ID_DATAPTS => self.gen_data_points(),
                _ => unreachable!(),
            }?;
            let block_info = self
                .map
                .block_info
                .iter()
                .find(|&x| x.identifier == block_id);
            if block_info.is_none() {
                return Err(WriteError::MissingBlockInfo(block_id.to_string()));
            }
            let new_block_info = BlockInfo {
                identifier: block_id.to_string(),
                revision_number: block_info.unwrap().revision_number,
                size: block_bytes.len() as i32,
            };
            new_map.block_info.push(new_block_info);
            new_map.block_count += 1;
            new_map.block_size += (block_id.len() + 1 + 2 + 4) as i32;
            bytes.extend(block_bytes);
        }

        // Optional blocks
        if self.supplier_parameters.is_some() {
            let block_bytes = self.gen_supplier_parameters()?;
            let block_id = parser::BLOCK_ID_SUPPARAMS;
            let block_info = self
                .map
                .block_info
                .iter()
                .find(|&x| x.identifier == block_id);
            if block_info.is_none() {
                return Err(WriteError::MissingBlockInfo(block_id.to_string()));
            }
            let new_block_info = BlockInfo {
                identifier: block_id.to_string(),
                revision_number: block_info.unwrap().revision_number,
                size: block_bytes.len() as i32,
            };
            new_map.block_info.push(new_block_info);
            new_map.block_count += 1;
            new_map.block_size += (block_id.len() + 1 + 2 + 4) as i32;
            bytes.extend(block_bytes);
        }

        // For each proprietary block, just write it out
        for pb in &self.proprietary_blocks {
            let block_bytes = self.gen_proprietary_block(pb)?;
            let block_info = self
                .map
                .block_info
                .iter()
                .find(|&x| x.identifier == pb.header);
            if block_info.is_none() {
                return Err(WriteError::MissingBlockInfo(pb.header.clone()));
            }
            let new_block_info = BlockInfo {
                identifier: pb.header.clone(),
                revision_number: block_info.unwrap().revision_number,
                size: block_bytes.len() as i32,
            };
            new_map.block_info.push(new_block_info);
            new_map.block_count += 1;
            new_map.block_size += (pb.header.len() + 1 + 2 + 4) as i32;
            bytes.extend(block_bytes);
        }

        let new_block_info = BlockInfo {
            identifier: parser::BLOCK_ID_CHECKSUM.to_string(),
            revision_number: 200,
            size: (parser::BLOCK_ID_CHECKSUM.len() + 1 + 2) as i32,
        };
        new_map.block_info.push(new_block_info);
        new_map.block_count += 1;
        new_map.block_size += (parser::BLOCK_ID_CHECKSUM.len() + 1 + 2 + 4) as i32;

        let mut map_bytes = self.gen_map(new_map)?;
        // Now construct the final file: map, then block data, then the checksum block
        map_bytes.extend(bytes);
        // Compute the checksum over the assembled map and data bytes
        let cs_block = self.gen_checksum_block(&map_bytes)?;
        map_bytes.extend(cs_block);

        Ok(map_bytes)
    }

    fn gen_map(&self, map: MapBlock) -> Result<Vec<u8>, WriteError> {
        let mut bytes: Vec<u8> = Vec::new();
        null_terminated_str(&mut bytes, parser::BLOCK_ID_MAP);
        le_integer(&mut bytes, map.revision_number);
        le_integer(&mut
            bytes,
            map.block_size + (parser::BLOCK_ID_MAP.len() as i32) + 1 + 2 + 4 + 2
        );
        le_integer(&mut bytes, map.block_count + 1);
        for bi in map.block_info {
            null_terminated_str(&mut bytes, &bi.identifier);
            le_integer(&mut bytes, bi.revision_number);
            le_integer(&mut bytes, bi.size);
        }
        Ok(bytes)
    }

    fn gen_general_parameters(&self) -> Result<Vec<u8>, WriteError> {
        let mut bytes: Vec<u8> = Vec::new();
        let gp = self.general_parameters.as_ref().ok_or_else(|| {
            WriteError::MissingMandatoryBlock(parser::BLOCK_ID_GENPARAMS.to_string())
        })?;
        null_terminated_str(&mut bytes, parser::BLOCK_ID_GENPARAMS);
        fixed_length_str(&mut bytes, &gp.language_code, 2)?;
        null_terminated_str(&mut bytes, &gp.cable_id);
        null_terminated_str(&mut bytes, &gp.fiber_id);
        le_integer(&mut bytes, gp.fiber_type);
        le_integer(&mut bytes, gp.nominal_wavelength);
        null_terminated_str(&mut bytes, &gp.originating_location);
        null_terminated_str(&mut bytes, &gp.terminating_location);
        null_terminated_str(&mut bytes, &gp.cable_code);
        fixed_length_str(&mut bytes, &gp.current_data_flag, 2)?;
        le_integer(&mut bytes, gp.user_offset);
        le_integer(&mut bytes, gp.user_offset_distance);
        null_terminated_str(&mut bytes, &gp.operator);
        null_terminated_str(&mut bytes, &gp.comment);
        Ok(bytes)
    }

    fn gen_supplier_parameters(&self) -> Result<Vec<u8>, WriteError> {
        let mut bytes: Vec<u8> = Vec::new();
        let sp = self.supplier_parameters.as_ref().unwrap();
        null_terminated_str(&mut bytes, parser::BLOCK_ID_SUPPARAMS);
        null_terminated_str(&mut bytes, &sp.supplier_name);
        null_terminated_str(&mut bytes, &sp.otdr_mainframe_id);
        null_terminated_str(&mut bytes, &sp.otdr_mainframe_sn);
        null_terminated_str(&mut bytes, &sp.optical_module_id);
        null_terminated_str(&mut bytes, &sp.optical_module_sn);
        null_terminated_str(&mut bytes, &sp.software_revision);
        null_terminated_str(&mut bytes, &sp.other);
        Ok(bytes)
    }

    fn gen_fixed_parameters(&self) -> Result<Vec<u8>, WriteError> {
        let mut bytes: Vec<u8> = Vec::new();
        let fp = self.fixed_parameters.as_ref().ok_or_else(|| {
            WriteError::MissingMandatoryBlock(parser::BLOCK_ID_FXDPARAMS.to_string())
        })?;
        null_terminated_str(&mut bytes, parser::BLOCK_ID_FXDPARAMS);
        le_integer(&mut bytes, fp.date_time_stamp);
        fixed_length_str(&mut bytes, &fp.units_of_distance, 2)?;
        le_integer(&mut bytes, fp.actual_wavelength);
        le_integer(&mut bytes, fp.acquisition_offset);
        le_integer(&mut bytes, fp.acquisition_offset_distance);
        le_integer(&mut bytes, fp.total_n_pulse_widths_used);
        for pulse_width in &fp.pulse_widths_used {
            le_integer(&mut bytes, *pulse_width);
        }
        for data_spacing in &fp.data_spacing {
            le_integer(&mut bytes, *data_spacing);
        }
        for n_data_points_for_pulse_widths_used in &fp.n_data_points_for_pulse_widths_used {
            le_integer(&mut bytes, *n_data_points_for_pulse_widths_used);
        }
        le_integer(&mut bytes, fp.group_index);
        le_integer(&mut bytes, fp.backscatter_coefficient);
        le_integer(&mut bytes, fp.number_of_averages);
        le_integer(&mut bytes, fp.averaging_time);
        le_integer(&mut bytes, fp.acquisition_range);
        le_integer(&mut bytes, fp.acquisition_range_distance);
        le_integer(&mut bytes, fp.front_panel_offset);
        le_integer(&mut bytes, fp.noise_floor_level);
        le_integer(&mut bytes, fp.noise_floor_scale_factor);
        le_integer(&mut bytes, fp.power_offset_first_point);
        le_integer(&mut bytes, fp.loss_threshold);
        le_integer(&mut bytes, fp.reflectance_threshold);
        le_integer(&mut bytes, fp.end_of_fibre_threshold);
        fixed_length_str(&mut bytes, &fp.trace_type, 2)?;
        le_integer(&mut bytes, fp.window_coordinate_1);
        le_integer(&mut bytes, fp.window_coordinate_2);
        le_integer(&mut bytes, fp.window_coordinate_3);
        le_integer(&mut bytes, fp.window_coordinate_4);
        Ok(bytes)
    }

    fn gen_key_events(&self) -> Result<Vec<u8>, WriteError> {
        let mut bytes: Vec<u8> = Vec::new();
        let events = self.key_events.as_ref().ok_or_else(|| {
            WriteError::MissingMandatoryBlock(parser::BLOCK_ID_KEYEVENTS.to_string())
        })?;
        null_terminated_str(&mut bytes, parser::BLOCK_ID_KEYEVENTS);
        le_integer(&mut bytes, events.number_of_key_events);
        for ke in &events.key_events {
            le_integer(&mut bytes, ke.event_number);
            le_integer(&mut bytes, ke.event_propogation_time);
            le_integer(&mut bytes, ke.attenuation_coefficient_lead_in_fiber);
            le_integer(&mut bytes, ke.event_loss);
            le_integer(&mut bytes, ke.event_reflectance);
            fixed_length_str(&mut bytes, &ke.event_code, 6)?;
            fixed_length_str(&mut bytes, &ke.loss_measurement_technique, 2)?;
            le_integer(&mut bytes, ke.marker_location_1);
            le_integer(&mut bytes, ke.marker_location_2);
            le_integer(&mut bytes, ke.marker_location_3);
            le_integer(&mut bytes, ke.marker_location_4);
            le_integer(&mut bytes, ke.marker_location_5);
            null_terminated_str(&mut bytes, &ke.comment);
        }
        le_integer(&mut bytes, events.last_key_event.event_number);
        le_integer(&mut bytes, events.last_key_event.event_propogation_time);
        le_integer(&mut
            bytes,
            events.last_key_event.attenuation_coefficient_lead_in_fiber
        );
        le_integer(&mut bytes, events.last_key_event.event_loss);
        le_integer(&mut bytes, events.last_key_event.event_reflectance);
        fixed_length_str(&mut bytes, &events.last_key_event.event_code, 6)?;
        fixed_length_str(
            &mut bytes,
            &events.last_key_event.loss_measurement_technique,
            2,
        )?;
        le_integer(&mut bytes, events.last_key_event.marker_location_1);
        le_integer(&mut bytes, events.last_key_event.marker_location_2);
        le_integer(&mut bytes, events.last_key_event.marker_location_3);
        le_integer(&mut bytes, events.last_key_event.marker_location_4);
        le_integer(&mut bytes, events.last_key_event.marker_location_5);
        null_terminated_str(&mut bytes, &events.last_key_event.comment);
        le_integer(&mut bytes, events.last_key_event.end_to_end_loss);
        le_integer(&mut bytes, events.last_key_event.end_to_end_marker_position_1);
        le_integer(&mut bytes, events.last_key_event.end_to_end_marker_position_2);
        le_integer(&mut bytes, events.last_key_event.optical_return_loss);
        le_integer(&mut
            bytes,
            events.last_key_event.optical_return_loss_marker_position_1
        );
        le_integer(&mut
            bytes,
            events.last_key_event.optical_return_loss_marker_position_2
        );
        Ok(bytes)
    }

    fn gen_data_points(&self) -> Result<Vec<u8>, WriteError> {
        let mut bytes: Vec<u8> = Vec::new();
        let dp = self.data_points.as_ref().ok_or_else(|| {
            WriteError::MissingMandatoryBlock(parser::BLOCK_ID_DATAPTS.to_string())
        })?;
        null_terminated_str(&mut bytes, parser::BLOCK_ID_DATAPTS);
        le_integer(&mut bytes, dp.number_of_data_points);
        le_integer(&mut bytes, dp.total_number_scale_factors_used);
        for sf in &dp.scale_factors {
            le_integer(&mut bytes, sf.n_points);
            le_integer(&mut bytes, sf.scale_factor);
            for pt in &sf.data {
                le_integer(&mut bytes, *pt);
            }
        }
        Ok(bytes)
    }

    fn gen_proprietary_block(&self, pb: &ProprietaryBlock) -> Result<Vec<u8>, WriteError> {
        let mut bytes: Vec<u8> = Vec::new();
        null_terminated_str(&mut bytes, &pb.header);
        bytes.extend(pb.data.iter());
        Ok(bytes)
    }

    fn gen_checksum_block(&self, data: &Vec<u8>) -> Result<Vec<u8>, WriteError> {
        let mut bytes: Vec<u8> = Vec::new();
        null_terminated_str(&mut bytes, parser::BLOCK_ID_CHECKSUM);
        let crc: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_3740);
        le_integer(&mut bytes, crc.checksum(data.as_slice()));

        Ok(bytes)
    }
}

#[cfg(test)]
fn test_sor_load<'a>() -> SORFile {
    let data = include_bytes!("../data/example4-exfo-ftb4ftbx730c-mfdgainer-1310nm.sor");
    parser::parse_file(data).unwrap().1
}

#[test]
fn test_gen_general_parameters() {
    let in_sor = test_sor_load();
    let _bytes = in_sor.gen_general_parameters();
    // println!("{:#?}", bytes);
    // let mut file = std::fs::File::create("test_genparam.bin").unwrap();
    // file.write_all(bytes.as_slice()).unwrap();
    // dbg!(bytes);
}
#[test]
fn test_gen_supplier_parameters() {
    let in_sor = test_sor_load();
    let _bytes = in_sor.gen_supplier_parameters();
    // println!("{:#?}", bytes);
    // let mut file = std::fs::File::create("test_supparam.bin").unwrap();
    // file.write_all(bytes.as_slice()).unwrap();
    // dbg!(bytes);
}

#[test]
fn test_gen_fixed_parameters() {
    let in_sor = test_sor_load();
    let _bytes = in_sor.gen_fixed_parameters();
    // println!("{:#?}", bytes);
    // let mut file = std::fs::File::create("test_fixedparam.bin").unwrap();
    // file.write_all(bytes.as_slice()).unwrap();
    // dbg!(bytes);
}

#[test]
fn test_gen_key_events() {
    let in_sor = test_sor_load();
    let _bytes = in_sor.gen_key_events();
    // println!("{:#?}", bytes);
    // let mut file = std::fs::File::create("test_keyevents.bin").unwrap();
    // file.write_all(bytes.as_slice()).unwrap();
    // dbg!(bytes);
}
#[test]
fn test_roundtrip_sor() {
    let in_sor = test_sor_load();
    let bytes = in_sor.to_bytes().unwrap();
    let out_sor = parser::parse_file(&bytes).unwrap().1;
    // We don't check the map directly, as it's regenerated on write.
    // However, the data content of all other blocks should be identical.
    assert_eq!(in_sor.general_parameters, out_sor.general_parameters);
    assert_eq!(in_sor.supplier_parameters, out_sor.supplier_parameters);
    assert_eq!(in_sor.fixed_parameters, out_sor.fixed_parameters);
    assert_eq!(in_sor.key_events, out_sor.key_events);
    assert_eq!(in_sor.link_parameters, out_sor.link_parameters);
    assert_eq!(in_sor.data_points, out_sor.data_points);
    assert_eq!(in_sor.proprietary_blocks, out_sor.proprietary_blocks);
}

#[test]
fn test_roundtrip_sor_checksums() {
    let in_sor = test_sor_load();
    let bytes = in_sor.to_bytes().unwrap();
    let out_sor = parser::parse_file(&bytes).unwrap().1;
    assert_eq!(in_sor.general_parameters, out_sor.general_parameters);
    let checksum = parser::validate_checksum(&bytes, &out_sor);
    assert_eq!(checksum.status, types::ChecksumStatus::Valid);
    assert_eq!(
        checksum.matched_by.unwrap(),
        types::ChecksumStrategy::PrecedingBytes
    );
}

#[test]
fn test_roundtrip_sor_with_modification() {
    let mut in_sor = test_sor_load();
    let new_cable_id = "MODIFIED CABLE ID".to_string();

    // Modify a value
    in_sor.general_parameters.as_mut().unwrap().cable_id = new_cable_id.clone();

    let bytes = in_sor.to_bytes().unwrap();
    let out_sor = parser::parse_file(&bytes).unwrap().1;

    // Assert that the modified value is present in the re-parsed struct
    assert_eq!(out_sor.general_parameters.unwrap().cable_id, new_cable_id);
}

#[test]
fn test_write_file_with_missing_mandatory_block() {
    let mut sor = test_sor_load();
    sor.general_parameters = None;
    let result = sor.to_bytes();
    assert_eq!(
        result,
        Err(WriteError::MissingMandatoryBlock(
            parser::BLOCK_ID_GENPARAMS.to_string()
        ))
    );
}
