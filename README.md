# otdrs

`otdrs` is a Rust implementation of a SOR file (Bellcore OTDR interchange format, specified in Telcordia SR-4731) parser and generator.

It is intended as an absolute minimal parser implementation to produce a translation from the binary format to open, self-documenting formats, to permit easy development of analysis tools without having to deal with the complexities of managing a binary format that is broadly undocumented and difficult to parse. It also provides Rust primitives for OTDR files and a writer to allow files to be loaded, modified, and written back.

Wherever possible, translation preserves the input format; otdrs does not try to fix peculiarities of particular test equipment or post-processing software or work around quirks in their input, except where they would break parsing of the format. No normalisation is attempted.

Where the parser cannot reliably extract information it is omitted; in this sense, `otdrs` is a best-effort parser.

Writing of files is performed with as much care as possible and the optional checksum is computed. Generated files open without issue in several professional applications, and proprietary data is preserved losslessly.

Rust was chosen for its robustness, type-safety, and the excellent `nom` parser library. `serde` is used for serialisation for output.

![Rust](https://github.com/JamesHarrison/otdrs/workflows/Rust/badge.svg) [![Cargo](https://img.shields.io/crates/v/otdrs)](https://crates.io/crates/otdrs) [![Downloads](https://img.shields.io/crates/d/otdrs)](https://crates.io/crates/otdrs)

## Usage

`otdrs` takes one positional argument, the path to a SOR file. Its output is a single JSON or CBOR blob which contains the information within the SOR file; flags are used to set the output path (default is stdout) or the format to output. `otdrs --help` shows the available options.

### Installing

If you have Rust/Cargo installed you can install otdrs with `cargo install otdrs`. `otdrs` is not otherwise packaged currently.

## Code Quality, Conformance/Compliance

This is the author's first major Rust project, so use with caution.

This is absolutely not guaranteed to work in all cases with all files or produce correct output in all cases (or indeed any). Check the output with the files you're using and a known-good viewer if you care about what you're doing.

### Security Considerations

As `otdrs` handles arbitrary binary input and performs some arithmetic on it which can potentially lead to underruns and overruns as well as exciting undefined behaviour.

While Rust is a very good language to write such tools in, since runtime errors such as this are handled, `otdrs` makes an effort to avoid obvious situations where slice pointers violate bounds or where arithmetic on SOR contents may lead to unexpected situations. [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) is used to fuzz with libFuzzer (on Linux only at present) to discover scenarios in which this can occur.

Checking of result validity is not performed on all fields, and users of the tool should take care to avoid trusting input parsed from SOR files. Sanitise your inputs.

## Known Issues

* The "link parameters" block is not currently decoded, as the author does not have files which contain it for testing. This is not used in common OTDR sets.
* Testing is not as comprehensive and extensive as it should be, particularly for writing files.

There is no application of fixed scaling factors described in SR-4731. This is generally intentional, to permit correct post-processing as required in other applications.

## Proprietary Blocks

While SOR files are standardised, not all of the content is; there are a set of standard and required blocks for the basic information, and otdrs only attempts to parse the standard blocks in a SOR file.

The content of proprietary blocks is dumped for analysis by upstream tools that may either have knowledge of proprietary formats or wish to simply know of the existence of such blocks. The map block will in all cases list all blocks within the file.

## Writing SORs

`otdrs` has experimental support for generating SORs from Rust data structures. Strictly, the map block is heavily recomputed when writing; a BlockInfo block with a revision number and header will be expected for all blocks, but sizes and counters are dynamically generated.

Editors are responsible for ensuring that any modification of data elsewhere in the file makes sense, e.g. if the number of points within a `DataPointsAtScaleFactor` struct is changed, then the `n_points` field must be amended by the editor; `otdrs` will not do this for you.

Currently, landmarks and `LinkParameters` are not written out.

## Testing

The parser has been tested on SOR files generated from:

* Noyes OFL280 OTDRs, including those re-exported from EXFO FastReporter3
* Anritsu Access Master OTDRs
* EXFO MaxTester 730C and FTB-4/FTBx730 OTDRs
* EXFO iOLM files exported to SOR from EXFO FastReporter3

Round-trip testing has also confirmed that all of the above files can be read and written out by `otdrs` with no loss or alteration of data.

Further test files are desired and should be submitted to the author or as a pull request with tests against known values.

## Interpretation

To actually interpet any of this data correctly you are probably going to need to read SR-4731, which can be found [here](https://telecom-info.telcordia.com/site-cgi/ido/docs.cgi?ID=SEARCH&DOCUMENT=SR-4731&) for around $750.

This parser makes no attempt to correctly interpret the resulting data from the SOR file format, merely to make it accessible for applications to perform correct interpretation. Actually locating events and measuring cable data based on OTDR data requires careful consideration of data offsets (e.g. front panel to user offset, scaling factors, etc).

### Vendor Quirks

As with pretty much every standard out there, every vendor has interpreted it differently. For one example, some Noyes OTDRs store a 30 second averaging period as `3000` whereas EXFO and Anritsu record it as `30`. The specification, of course, says this should be stored as `300`. Professional software struggles with this - for instance, EXFO's FastReporter3 software misinterprets the Noyes result.

Documentation of storage quirks compared to "standard" behaviour such as the above, against *known* behaviour would be helpful for those developing post-processing software; if you have access to test equipment, you can helpfully run a controlled test, write down the *actual* values displayed by the tester etc and store the SOR file directly.

While "fixing" vendor quirks to generate a *standards-compliant* output is not currently in the scope of `otdrs`, this is something that could be added as an optional post-processing step.

## Versions

* 0.4.0 - added SORFile#to_bytes and a whole bunch of related functions and macros which altogether mean that `otdrs` can now write OTDR files
* 0.3.0 - switched to using [clap](https://github.com/clap-rs/clap) for command-line argument handling for better error handling, added option to write to file instead of stdout and added [CBOR](https://github.com/pyfisch/cbor) export support
* 0.2.0 - restructured to allow use as a library, added fuzzing and fixed a number of bounds-check/error propogation problems
* 0.1.0 - initial release

## License

GPLv3 has been selected specifically to drive improved open source engagement with equipment manufacturers and developers of OTDR processing software in an industry that has struggled with open data exchange, proprietary (and vendor-locked) software, and poor maintenance of existing software.

otdrs - a SOR file parsing tool 
Copyright (C) 2021 James Harrison

This program is free software: you can redistribute it and/or modify
it under the terms of the GNU General Public License as published by
the Free Software Foundation, either version 3 of the License, or
(at your option) any later version.

This program is distributed in the hope that it will be useful,
but WITHOUT ANY WARRANTY; without even the implied warranty of
MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
GNU General Public License for more details.

You should have received a copy of the GNU General Public License
along with this program.  If not, see <http://www.gnu.org/licenses/>.