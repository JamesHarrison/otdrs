use crate::parser;
use crate::types::SORFile;
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::fs::File;
use std::io::Read;
/// Loads an OTDR file and returns the result
#[pyfunction]
fn parse_file(path: String) -> PyResult<SORFile> {
    let mut file = File::open(path)?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    let parse_result = parser::parse_file(buffer.as_slice());
    let result = match parse_result {
        Ok(sor) => Ok(sor.1),
        Err(_) => Err(PyRuntimeError::new_err("Error parsing SOR file")),
    };
    return result;
}

/// Parses provided bytestring as an OTDR file
#[pyfunction]
fn parse_bytes(bytes: &Bound<'_, PyBytes>) -> PyResult<SORFile> {
    let parse_result = parser::parse_file(bytes.as_bytes());
    let result = match parse_result {
        Ok(sor) => Ok(sor.1),
        Err(_) => Err(PyRuntimeError::new_err("Error parsing SOR file")),
    };
    return result;
}

/// This module is implemented in Rust.
#[pymodule]
fn otdrs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_file, m)?)?;
    m.add_function(wrap_pyfunction!(parse_bytes, m)?)?;
    return Ok(());
}
