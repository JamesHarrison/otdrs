#![no_main]
#[macro_use] use libfuzzer_sys::fuzz_target;
fuzz_target!(|data: &[u8]| {
    let parser = otdrs::parser::parse_file(data);
    let str = match parser {
        Ok(res) => "OK",
        Err(err) => "Parse error",
    };
});
