# otdrs Agent guide

This is a library for optical time domain reflectometry data processing. Refer to README.md for the full description of the project.

The library is primarily open source and intended for use in a range of projects, so code should be kept reasonably generic, flexible, and well-documented. Avoid single-purpose or tightly-coupled code.

Testing is important to ensure that no faults are introduced.

While the project is written in Rust, Python bindings are provided by the crate, along with type hinting.

Because the primary function of the project is to read and write a proprietary binary format, it is important to ensure that the format is preserved as accurately as possible. The serialisation and deserialisation of the format implemented as-is is correct and should never be altered. Likewise, the core functionality and structures of the `otdrs` crate should never be altered, as this could impact the accuracy of the library.

Use Rust best practices and idioms, and follow the Rust book for guidance. Avoid unsafe code, and ensure that Rust safety best practices are used throughout.

Where raw binary data is ingested, ensure that fuzzer configurations (under `fuzz` and `fuzz-afl`) are updated with entry points to check.

Where possible, use existing crates and libraries to avoid reinventing the wheel.
