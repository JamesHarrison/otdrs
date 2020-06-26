# otdrs

`otdrs` is a Rust implementation of a SOR file (Bellcore OTDR interchange format, specified in Telcordia SR-4731) parser.

It is intended as an absolute minimal parser implementation to produce a translation from the binary format to open, self-documenting formats, to permit easy development of analysis tools without having to deal with the complexities of managing a binary format that is broadly undocumented and difficult to parse.

Wherever possible, translation preserves the input format; otdrs does not try to fix peculiarities of particular test equipment or post-processing software or work around quirks in their input, except where they would break parsing of the format. No normalisation is attempted.

Where the parser cannot reliably extract information it is omitted; in this sense, `otdrs` is a best-effort parser.

Rust was chosen for its robustness, type-safety, and the excellent `nom` parser library. `serde` is used for serialisation for output.

![Rust](https://github.com/JamesHarrison/otdrs/workflows/Rust/badge.svg)

## Usage

`otdrs` takes one argument, the path to a SOR file. Its output is a single JSON root which contains the information within the SOR file.

### Installing

If you have Rust/Cargo installed you can install otdrs with `cargo install otdrs`.

## Code Quality, Conformance/Compliance

This is the author's first major Rust project, so use with caution.

This is absolutely not guaranteed to work in all cases with all files or produce correct output in all cases (or indeed any). Check the output with the files you're using and a known-good viewer if you care about what you're doing.

## Known Issues

* The "link parameters" block is not currently decoded, as the author does not have files which contain it for testing. This is not used in common OTDR sets.
* Testing is not as comprehensive and extensive as it should be
* There is no application of fixed scaling factors described in SR-4731

## Proprietary Blocks

While SOR files are standardised, not all of the content is; there are a set of standard and required blocks for the basic information, and otdrs only attempts to parse the standard blocks in a SOR file.

The content of proprietary blocks is dumped for analysis by upstream tools that may either have knowledge of proprietary formats or wish to simply know of the existence of such blocks. The map block will in all cases list all blocks within the file.

## Testing

The parser has been tested on SOR files generated from:

* Noyes OFL280 OTDRs, including those re-exported from EXFO FastReporter3
* Anritsu Access Master OTDRs
* EXFO MaxTester and FTBx OTDRs
* EXFO iOLM files exported to SOR from EXFO FastReporter3

## Interpretation

To actually interpet any of this data correctly you are going to need to read SR-4731, which can be found [here](https://telecom-info.telcordia.com/site-cgi/ido/docs.cgi?ID=SEARCH&DOCUMENT=SR-4731&) for around $750.

This parser makes no attempt to correctly interpret the resulting data from the SOR file format, merely to make it accessible.

## License

otdrs - a SOR file parsing tool 
Copyright (C) 2020 James Harrison

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