#[macro_use]
extern crate afl;
fn main() {
    fuzz!(|data: &[u8]| {
        let parser = otdrs::parser::parse_file(data);
        let _str = match parser {
            Ok(_res) => {
                // Dump to exercise the writer and then re-parse the output...
                let bytes = _res.1.to_bytes();
                match bytes {
                    Ok(_sorbytes) => {
                        let parser2 = otdrs::parser::parse_file(&_sorbytes);
                        match parser2 {
                            Ok(_res) => {
                                otdrs::parser::validate_checksum(&_sorbytes, &_res.1);
                                "OK"
                            },
                            Err(_err) => "Failed to parse written file",
                        }
                    }
                    Err(_sorerr) => "Failed to write file",
                }
            }
            Err(_err) => "Parse error",
        };
    });
}
