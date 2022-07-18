/// Base library for otdrs
pub mod types;
pub mod parser;
use crc::{Crc, CRC_16_KERMIT};
use crate::types::{BlockInfo, MapBlock, ProprietaryBlock, SORFile};

// These macros are used to coherently and consistently produce all the binary encodings that we need
macro_rules! null_terminated_str {
    ( $b:expr, $s:expr ) => {
        $b.extend($s.as_bytes());
        $b.push(0x0);
    };
}
macro_rules! fixed_length_str {
    ( $b:expr, $s:expr, $len:expr ) => {
        let mut bytes: Vec<u8> = Vec::with_capacity($len);
        for c in $s.chars() {
            let mut byte = [0; 1];
            if c.len_utf8() > 1 {
                return Err("A character in a fixed-length string appears to be UTF-8 and require more than one byte to encode, which is not permitted in the standard.")
            }
            c.encode_utf8(&mut byte);
            bytes.push(byte[0]);
        }
        $b.extend(bytes);
    };
}
macro_rules! le_integer {
    ( $b:expr, $i:expr ) => {
        $b.extend(&$i.to_le_bytes());
    };
}

macro_rules! add_block {
    ($b:expr, $m:expr, $nm:expr, $block:expr, $gen_block:expr, $block_id:expr) => {
        if $block.is_some() {
            let block_bytes = match $gen_block {
                Ok(res) => res,
                Err(err) => { return Err(err); }
            };
            let block_info = $m.block_info.iter().find(|&x| x.identifier == $block_id);
            if block_info.is_none() {
                return Err("BlockInfo block is missing for one of your blocks in the Map!");
            }
            let new_block_info = BlockInfo {
                identifier: $block_id,
                revision_number: block_info.unwrap().revision_number,
                size: block_bytes.len() as i32
            };
            $nm.block_info.push(new_block_info);
            $nm.block_count += 1;
            // Per block: header string length + null terminating byte + 2-byte rev num + 4-byte size
            $nm.block_size += ($block_id.len() + 1 + 2 + 4) as i32; 
            $b.extend(block_bytes);
        }
    };
    // FIXME: This is just the above without the is_some check for the vector form for proprietary blocks, best not duplicated but it's 1am
    ($b:expr, $m:expr, $nm:expr, $gen_block:expr, $block_id:expr) => {
        let block_bytes = match $gen_block {
            Ok(res) => res,
            Err(err) => { return Err(err); }
        };
        let block_info = $m.block_info.iter().find(|&x| x.identifier == $block_id);
        if block_info.is_none() {
            return Err("BlockInfo block is missing for one of your blocks in the Map!");
        }
        let new_block_info = BlockInfo {
            identifier: $block_id,
            revision_number: block_info.unwrap().revision_number,
            size: block_bytes.len() as i32
        };
        $nm.block_info.push(new_block_info);
        $nm.block_count += 1;
        // Per block: header string length + null terminating byte + 2-byte rev num + 4-byte size
        $nm.block_size += ($block_id.len() + 1 + 2 + 4) as i32; 
        $b.extend(block_bytes);
    };
}

impl<'a> SORFile<'a> {
    #[allow(dead_code)]
    pub fn to_bytes(&self) -> Result<Vec<u8>, &str> {
        let mut bytes: Vec<u8> = Vec::new();
        // Basically, we're now going to generate everything from scratch from our internal state
        // We therefore need a new map block to describe the resulting blocks.
        let mut new_map = MapBlock{
            revision_number: self.map.revision_number,
            block_count: 0,
            block_size: 0,
            block_info: Vec::new()
        };
        // Then we add to this block for anything we have
        // FIXME: We should probably explode instead of producing non-compliant files, e.g. genparams is mandatory in spec
        // We are permissive in reading and parsing nonsense files but should be strict in production.
        add_block!(bytes, self.map, new_map, self.general_parameters, self.gen_general_parameters(), parser::BLOCK_ID_GENPARAMS);
        add_block!(bytes, self.map, new_map, self.supplier_parameters, self.gen_supplier_parameters(), parser::BLOCK_ID_SUPPARAMS);
        add_block!(bytes, self.map, new_map, self.fixed_parameters, self.gen_fixed_parameters(), parser::BLOCK_ID_FXDPARAMS);
        add_block!(bytes, self.map, new_map, self.key_events, self.gen_key_events(), parser::BLOCK_ID_KEYEVENTS);
        add_block!(bytes, self.map, new_map, self.data_points, self.gen_data_points(), parser::BLOCK_ID_DATAPTS);
        
        // For each proprietary block, just write it out
        for pb in &self.proprietary_blocks {
            add_block!(bytes, self.map, new_map, self.gen_proprietary_block(pb), pb.header);
        }
        
        // Now we want to generate our checksum block - first we have to add the block to the map, before we bake it in, so we do this manually here...
        let new_block_info = BlockInfo {
            identifier: parser::BLOCK_ID_CHECKSUM,
            revision_number: 200, // We're hardcoding this because we can
            size: (parser::BLOCK_ID_CHECKSUM.len() + 1 + 2) as i32
        };
        new_map.block_info.push(new_block_info);
        new_map.block_count += 1;
        new_map.block_size += (parser::BLOCK_ID_CHECKSUM.len() + 1 + 2 + 4) as i32; 

        // dbg!(&self.map);
        // dbg!(&new_map);
        
        let mut map_bytes = self.gen_map(new_map).unwrap();
        map_bytes.extend(bytes);

        // This is now the complete file - almost. We now gen the checksum block and tack it on the end.
        let cs_block = self.gen_checksum_block(&map_bytes).unwrap();
        map_bytes.extend(cs_block);
        
        Ok(map_bytes)
    }

    fn gen_map(&self, map: MapBlock) -> Result<Vec<u8>, &str> {
        let mut bytes: Vec<u8> = Vec::new();
        null_terminated_str!(bytes, parser::BLOCK_ID_MAP);
        le_integer!(bytes, map.revision_number);
        // length of header + null terminal + u16 + i32 + i16 for this, added to the blockinfo size
        // blockinfo size is already set by the add_block! macro
        le_integer!(bytes, map.block_size + (parser::BLOCK_ID_MAP.len() as i32) + 1 + 2 + 4 + 2);
        le_integer!(bytes, map.block_count + 1); // We add one to the 
        for bi in map.block_info {
            null_terminated_str!(bytes, bi.identifier);
            le_integer!(bytes, bi.revision_number);
            le_integer!(bytes, bi.size);
        }
        Ok(bytes)
    }

    fn gen_general_parameters(&self) -> Result<Vec<u8>, &str> {
        let mut bytes: Vec<u8> = Vec::new();
        let gp = self.general_parameters.as_ref().unwrap();
        null_terminated_str!(bytes, parser::BLOCK_ID_GENPARAMS);
        fixed_length_str!(bytes, gp.language_code, 2);
        null_terminated_str!(bytes, gp.cable_id);
        null_terminated_str!(bytes, gp.fiber_id); 
        le_integer!(bytes, gp.fiber_type);
        le_integer!(bytes, gp.nominal_wavelength);
        null_terminated_str!(bytes, gp.originating_location);
        null_terminated_str!(bytes, gp.terminating_location);
        null_terminated_str!(bytes, gp.cable_code);
        fixed_length_str!(bytes, gp.current_data_flag, 2);
        le_integer!(bytes, gp.user_offset);
        le_integer!(bytes, gp.user_offset_distance);
        null_terminated_str!(bytes, gp.operator); 
        null_terminated_str!(bytes, gp.comment); 
        Ok(bytes)
    }

    fn gen_supplier_parameters(&self) -> Result<Vec<u8>, &str> {
        let mut bytes: Vec<u8> = Vec::new();
        let sp = self.supplier_parameters.as_ref().unwrap();
        null_terminated_str!(bytes, parser::BLOCK_ID_SUPPARAMS);
        null_terminated_str!(bytes, sp.supplier_name);
        null_terminated_str!(bytes, sp.otdr_mainframe_id);
        null_terminated_str!(bytes, sp.otdr_mainframe_sn);
        null_terminated_str!(bytes, sp.optical_module_id);
        null_terminated_str!(bytes, sp.optical_module_sn);
        null_terminated_str!(bytes, sp.software_revision);
        null_terminated_str!(bytes, sp.other);
        Ok(bytes)
    }

    fn gen_fixed_parameters(&self) -> Result<Vec<u8>, &str> {
        let mut bytes: Vec<u8> = Vec::new();
        let fp = self.fixed_parameters.as_ref().unwrap();
        null_terminated_str!(bytes, parser::BLOCK_ID_FXDPARAMS);
        le_integer!(bytes, fp.date_time_stamp);
        fixed_length_str!(bytes, fp.units_of_distance, 2);
        le_integer!(bytes, fp.actual_wavelength);
        le_integer!(bytes, fp.acquisition_offset);
        le_integer!(bytes, fp.acquisition_offset_distance);
        le_integer!(bytes, fp.total_n_pulse_widths_used);
        for pulse_width in &fp.pulse_widths_used {
            le_integer!(bytes, pulse_width);
        }
        for data_spacing in &fp.data_spacing {
            le_integer!(bytes, data_spacing);
        }
        for n_data_points_for_pulse_widths_used in &fp.n_data_points_for_pulse_widths_used {
            le_integer!(bytes, n_data_points_for_pulse_widths_used);
        }
        le_integer!(bytes, fp.group_index);
        le_integer!(bytes, fp.backscatter_coefficient);
        le_integer!(bytes, fp.number_of_averages);
        le_integer!(bytes, fp.averaging_time);
        le_integer!(bytes, fp.acquisition_range);
        le_integer!(bytes, fp.acquisition_range_distance);
        le_integer!(bytes, fp.front_panel_offset);
        le_integer!(bytes, fp.noise_floor_level);
        le_integer!(bytes, fp.noise_floor_scale_factor);
        le_integer!(bytes, fp.power_offset_first_point);
        le_integer!(bytes, fp.loss_threshold);
        le_integer!(bytes, fp.reflectance_threshold);
        le_integer!(bytes, fp.end_of_fibre_threshold);
        fixed_length_str!(bytes, fp.trace_type, 2);
        le_integer!(bytes, fp.window_coordinate_1);
        le_integer!(bytes, fp.window_coordinate_2);
        le_integer!(bytes, fp.window_coordinate_3);
        le_integer!(bytes, fp.window_coordinate_4);
        Ok(bytes)
    }

    fn gen_key_events(&self) -> Result<Vec<u8>, &str> {
        let mut bytes: Vec<u8> = Vec::new();
        let events = self.key_events.as_ref().unwrap();
        null_terminated_str!(bytes, parser::BLOCK_ID_KEYEVENTS);
        le_integer!(bytes, events.number_of_key_events);
        for ke in &events.key_events {
            le_integer!(bytes, ke.event_number);
            le_integer!(bytes, ke.event_propogation_time);
            le_integer!(bytes, ke.attenuation_coefficient_lead_in_fiber);
            le_integer!(bytes, ke.event_loss);
            le_integer!(bytes, ke.event_reflectance);
            fixed_length_str!(bytes, ke.event_code, 6);
            fixed_length_str!(bytes, ke.loss_measurement_technique, 2);
            le_integer!(bytes, ke.marker_location_1);
            le_integer!(bytes, ke.marker_location_2);
            le_integer!(bytes, ke.marker_location_3);
            le_integer!(bytes, ke.marker_location_4);
            le_integer!(bytes, ke.marker_location_5);
            null_terminated_str!(bytes, ke.comment);
        }
        le_integer!(bytes, events.last_key_event.event_number);
        le_integer!(bytes, events.last_key_event.event_propogation_time);
        le_integer!(bytes, events.last_key_event.attenuation_coefficient_lead_in_fiber);
        le_integer!(bytes, events.last_key_event.event_loss);
        le_integer!(bytes, events.last_key_event.event_reflectance);
        fixed_length_str!(bytes, events.last_key_event.event_code, 6);
        fixed_length_str!(bytes, events.last_key_event.loss_measurement_technique, 2);
        le_integer!(bytes, events.last_key_event.marker_location_1);
        le_integer!(bytes, events.last_key_event.marker_location_2);
        le_integer!(bytes, events.last_key_event.marker_location_3);
        le_integer!(bytes, events.last_key_event.marker_location_4);
        le_integer!(bytes, events.last_key_event.marker_location_5);
        null_terminated_str!(bytes, events.last_key_event.comment);
        le_integer!(bytes, events.last_key_event.end_to_end_loss);
        le_integer!(bytes, events.last_key_event.end_to_end_marker_position_1);
        le_integer!(bytes, events.last_key_event.end_to_end_marker_position_2);
        le_integer!(bytes, events.last_key_event.optical_return_loss);
        le_integer!(bytes, events.last_key_event.optical_return_loss_marker_position_1);
        le_integer!(bytes, events.last_key_event.optical_return_loss_marker_position_2);
        Ok(bytes)
    }

    fn gen_data_points(&self) -> Result<Vec<u8>, &str> {
        let mut bytes: Vec<u8> = Vec::new();
        let dp = self.data_points.as_ref().unwrap();
        null_terminated_str!(bytes, parser::BLOCK_ID_DATAPTS);
        le_integer!(bytes, dp.number_of_data_points);
        le_integer!(bytes, dp.total_number_scale_factors_used);
        for sf in &dp.scale_factors {
            le_integer!(bytes, sf.n_points);
            le_integer!(bytes, sf.scale_factor);
            for pt in &sf.data {
                le_integer!(bytes, pt);
            }
        }
        Ok(bytes)
    }

    fn gen_proprietary_block(&self, pb: &ProprietaryBlock) -> Result<Vec<u8>, &str> {
        let mut bytes: Vec<u8> = Vec::new();
        null_terminated_str!(bytes, pb.header);
        bytes.extend(pb.data);
        Ok(bytes)
    }

    fn gen_checksum_block(&self, data: &Vec<u8>) -> Result<Vec<u8>, &str> {
        let mut bytes: Vec<u8> = Vec::new();
        null_terminated_str!(bytes, parser::BLOCK_ID_CHECKSUM);
        let crc: Crc<u16> = Crc::<u16>::new(&CRC_16_KERMIT);
        le_integer!(bytes, crc.checksum(data.as_slice()));

        Ok(bytes)
    }

}


#[cfg(test)]
fn test_sor_load<'a>() -> SORFile<'a> {
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
    let _out_sor = parser::parse_file(&bytes).unwrap().1;
    // assert_eq!(in_sor.map, out_sor.map);
    // dbg!(out_sor);
    // let mut file = std::fs::File::create("test_out.sor").unwrap();
    // file.write_all(bytes.as_slice()).unwrap();
    // FIXME: Actually assert some stuff in these!
    // FIXME: Test round-trip *with modification of the data* to make sure we're not copying stuff that should be modified
}