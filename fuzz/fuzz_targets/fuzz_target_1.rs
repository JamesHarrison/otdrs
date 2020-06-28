#![no_main]
use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let parser = otdrs::parser::parse_file(data);
    let _str = match parser {
        Ok(_res) => "OK",
        Err(_err) => "Parse error",
    };
});
