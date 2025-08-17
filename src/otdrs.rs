//!
//! # otdrs
//!
//! otdrs is a tool for parsing Telcordia SOR files into a neutral, open format
//! for further processing.
//!
//! The serde library is used for serialisation, and currently only JSON output
//! is supported.
//!
use std::fs::File;
use std::io::prelude::*;
// use anyhow::Error;
// use thiserror::Error;
use clap::Parser;
/// This doc string acts as a help message when the user runs '--help'
/// as do all doc strings on fields
#[derive(Parser)]
#[clap(
    version = "1.1.0",
    author = "James Harrison <james@talkunafraid.co.uk>",
    about = "otdrs is a conversion utility to convert Telcordia SOR files, used by optical time-domain reflectometry testers, into open formats such as JSON"
)]
struct Opts {
    #[clap(index = 1, required = true)]
    input_filename: String,
    #[clap(short, long, default_value = "json")]
    format: String,
    #[clap(short, long, default_value = "stdout")]
    output_filename: String,
}

/// By default we simply read the file provided as the first argument, and
/// print the parsed file as JSON to stdout
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    let mut file = File::open(opts.input_filename)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let parser = otdrs::parser::parse_file(buffer.as_slice());
    let res = parser.unwrap().1;
    let out;
    // let output_file;
    //
    // let mut output_file = File::open(opts.output_filename)?;
    if opts.format == "json" {
        out = (&serde_json::to_vec(&res).unwrap()).to_owned();
    } else if opts.format == "cbor" {
        out = (&serde_cbor::to_vec(&res).unwrap()).to_owned();
    } else {
        panic!("Unimplemented output format");
    }
    if opts.output_filename == "stdout" {
        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(&out)?;
    } else {
        let mut output_file = File::create(opts.output_filename).unwrap();
        output_file.write_all(&out)?;
    }

    Ok(())
}
