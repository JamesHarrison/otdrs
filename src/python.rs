use crate::parser;
use crate::types::{ChecksumValidationResult, SORFile};
use pyo3::exceptions::PyRuntimeError;
use pyo3::prelude::*;
use pyo3::types::PyBytes;
use std::fs::File;
use std::io::{Read, Write};

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

#[pymethods]
impl SORFile {
    /// Returns the SOR file as a byte string.
    #[pyo3(name = "to_bytes")]
    fn to_bytes_py<'py>(&self, py: Python<'py>) -> PyResult<Bound<'py, PyBytes>> {
        match self.to_bytes() {
            Ok(bytes) => Ok(PyBytes::new(py, &bytes)),
            Err(err) => Err(PyRuntimeError::new_err(err.to_string())),
        }
    }

    /// Writes the SOR file to the given path.
    #[pyo3(name = "write_file")]
    fn write_file_py(&self, path: String) -> PyResult<()> {
        match self.to_bytes() {
            Ok(bytes) => {
                let mut file = std::fs::File::create(path)?;
                file.write_all(&bytes)?;
                Ok(())
            }
            Err(err) => Err(PyRuntimeError::new_err(err.to_string())),
        }
    }

    /// Validates checksum given the original SOR bytes. Returns a Python-friendly result.
    #[pyo3(name = "validate_checksum")]
    fn validate_checksum_py(&self, bytes: &Bound<'_, PyBytes>) -> PyResult<ChecksumValidationResult> {
        let result = parser::validate_checksum(bytes.as_bytes(), self);
        Ok(result.into())
    }

}

/// This module is implemented in Rust.
#[pymodule]
fn otdrs(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_function(wrap_pyfunction!(parse_file, m)?)?;
    m.add_function(wrap_pyfunction!(parse_bytes, m)?)?;
    m.add_class::<SORFile>()?;
    return Ok(());
}
