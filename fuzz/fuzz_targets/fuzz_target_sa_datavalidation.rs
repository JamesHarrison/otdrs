#![no_main]
use libfuzzer_sys::fuzz_target;
use otdrs::types::SORFile;
fuzz_target!(|sor: SORFile| {
    // Dump to exercise the writer and then re-parse the output...
    let bytes = sor.to_bytes();
    match bytes {
        Ok(_sorbytes) => {
            let parser2 = otdrs::parser::parse_file(&_sorbytes);
            match parser2 {
                Ok(_res) => {
                    match sor.general_parameters {
                        Some(block) => {
                            let reparsed_block = _res.1.general_parameters.unwrap();
                            assert_eq!(block.cable_id, reparsed_block.cable_id);
                            // null term
                            assert_eq!(block.language_code, reparsed_block.language_code);
                            // fixed length
                        }
                        None => {}
                    }
                    match sor.fixed_parameters {
                        Some(block) => {
                            let reparsed_block = _res.1.fixed_parameters.unwrap();
                            assert_eq!(block.units_of_distance, reparsed_block.units_of_distance);
                            assert_eq!(block.actual_wavelength, reparsed_block.actual_wavelength);
                            assert_eq!(block.acquisition_offset, reparsed_block.acquisition_offset);
                            assert_eq!(block.pulse_widths_used, reparsed_block.pulse_widths_used);
                        }
                        None => {}
                    }

                    "OK"
                }
                Err(_err) => "Failed to parse written file",
            }
        }
        Err(_sorerr) => "Failed to write file",
    };
});
