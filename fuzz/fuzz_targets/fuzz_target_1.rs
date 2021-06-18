#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let parser = otdrs::parser::parse_file(data);
    let _str = match parser {
        Ok(_res) => {
            // Dump to exercise the writer and then re-parse the output...
            let bytes = _res.1.to_bytes();
            match bytes {
                Ok(_sorbytes) => {
                    let parser2 = otdrs::parser::parse_file(&_sorbytes);
                    match parser2 {
                        Ok(_res) => "OK",
                        Err(_err) => "Failed to parse written file",
                    }
                },
                Err(_sorerr) => "Failed to write file"
            }
            
        },
        Err(_err) => "Parse error",
    };
    
    
});
