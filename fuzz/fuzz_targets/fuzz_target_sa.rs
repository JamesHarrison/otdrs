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
                    // match sor.general_parameters {
                    //     Some(block) => {
                    //         assert_eq!(block.cable_id, _res.1.general_parameters.unwrap().cable_id);
                    //     }
                    //     None => {}
                    // }

                    "OK"
                }
                Err(_err) => "Failed to parse written file",
            }
        }
        Err(_sorerr) => "Failed to write file",
    };
});
