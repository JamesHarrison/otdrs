//!
//! # otdrs
//! 
//! otdrs is a tool for parsing Telcordia SOR files into a neutral, open format
//! for further processing.
//! 
//! The serde library is used for serialisation, and currently only JSON output 
//! is supported.
//! 
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
/// By default we simply read the file provided as the first argument, and 
/// print the parsed file as JSON to stdout
fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();
    let filename = &args[1];
    let mut file = File::open(filename)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let parser = otdrs::parser::parse_file(buffer.as_slice());
    match parser {
        Ok(res) => print!("{}", serde_json::to_string(&res.1)?),
        Err(err) => print!("Parse error {}", err),
    }
    Ok(())
}
