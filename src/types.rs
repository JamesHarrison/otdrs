/// Import pyo3 if required
#[cfg(feature = "python")]
use pyo3::prelude::*;
/// This module contains all of the struct definitions for the various types
/// we're pulling from OTDR files.
use serde::Serialize;
/// A BlockInfo struct contains information about a specific block later in the
/// file, and appears in the MapBlock
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Clone)]
#[cfg_attr(
    feature = "python",
    pyclass(frozen, eq, hash, module = "otdrs", get_all)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct BlockInfo {
    /// Name of the block
    pub identifier: String,
    /// Revision number - major (3 digits), minor, cosmetic
    pub revision_number: u16,
    /// Size in bytes of the block
    pub size: i32,
}

/// Every SOR file has a MapBlock which acts as a map to the file's contents
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Clone)]
#[cfg_attr(
    feature = "python",
    pyclass(frozen, eq, hash, module = "otdrs", get_all)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct MapBlock {
    /// Revision number - major (3 digits), minor, cosmetic - for the file as a
    /// whole
    pub revision_number: u16,
    /// Block size for the map block
    pub block_size: i32,
    /// Number of blocks in the file
    pub block_count: i16,
    /// Information on all the blocks in this file
    pub block_info: Vec<BlockInfo>,
}

/// The GeneralParametersBlock is mandatory for the format and contains
/// test-identifying information as well as generic information about the test
/// being run such as the nominal wavelength
#[derive(Debug, PartialEq, Eq, Hash, Serialize, Clone)]
#[cfg_attr(
    feature = "python",
    pyclass(frozen, eq, hash, module = "otdrs", get_all)
)]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct GeneralParametersBlock {
    /// Language code - EN, CN, JP, etc.
    pub language_code: String,
    /// Cable identifier
    pub cable_id: String,
    /// Fibre identifier
    pub fiber_id: String,
    /// Fibre type - this is generally coded as the ITU-T standard definition,
    /// sans letters, e.g. 657, 655.
    pub fiber_type: i16,
    /// Nominal test wavelength in nm
    pub nominal_wavelength: i16,
    /// Start location for the test
    pub originating_location: String,
    /// End location for the test
    pub terminating_location: String,
    /// Cable code - free field
    pub cable_code: String,
    ///  NC for new condition, RC for as-repaired, OT as something else
    pub current_data_flag: String,
    /// User offset - This is essentially the launch lead length from the front
    /// panel offset (provided in the fixed parameters block), in 100ps
    /// increments
    pub user_offset: i32,
    /// This is the same as user_offset, but measured in 10x the distance units
    /// in FixedParametersBlock.units_of_distance
    pub user_offset_distance: i32,
    /// Operator of the unit for the test
    pub operator: String,
    /// Free comment field
    pub comment: String,
}

/// Supplier parameters describe the OTDR unit itself, such as the optical
/// module ID/serial number. Often this block also contains information about
/// calibration dates in the "other" field.
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct SupplierParametersBlock {
    /// Manufacturer of the OTDR
    pub supplier_name: String,
    /// Mainframe model number
    pub otdr_mainframe_id: String,
    /// Mainframe serial number
    pub otdr_mainframe_sn: String,
    /// Optical module model number
    pub optical_module_id: String,
    /// Optical module serial number
    pub optical_module_sn: String,
    /// Software revision
    pub software_revision: String,
    /// Free text
    pub other: String,
}

/// Fixed parameters block contains key information for interpreting the test
/// data
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct FixedParametersBlock {
    /// Datestamp - unix epoch seconds, 32-bit. Remember not to do any OTDR
    /// tests after 2038.
    pub date_time_stamp: u32,
    /// Units of distance - km, mt, ft, kf, mi, etc. Typically mt (in civilised
    /// nations)
    pub units_of_distance: String,
    /// Actual wavelength used - normally the factory-calibrated wavelength in
    /// nm, or nominal wavelength
    pub actual_wavelength: i16,
    /// Acquisition offset - this is the length of fibre from the OTDR port to
    /// the first data point in the DataPoints, in 100ps increments
    pub acquisition_offset: i32,
    /// As acquisition_offset, but as 10x units_of_distance
    pub acquisition_offset_distance: i32,
    /// The total number of pulse widths used, if more than one pulse width's
    /// results are stored in the file
    pub total_n_pulse_widths_used: i16,
    /// The list of pulse widths used, in nanoseconds
    pub pulse_widths_used: Vec<i16>,
    /// Data spacing, time taken to acquire 10,000 points in 100ps increments
    pub data_spacing: Vec<i32>,
    /// Number of points stored for each pulse width
    pub n_data_points_for_pulse_widths_used: Vec<i32>,
    /// Group index - the refractive index of the fibre, default to 146800 if
    /// nothing supplied
    pub group_index: i32,
    /// Backscatter coefficient -
    pub backscatter_coefficient: i16,
    /// Number of averages - the number of samples that were averaged to
    /// generate the result - may be used instead of averaging time
    pub number_of_averages: i32,
    /// Averaging time - may be supplied instead of number of averages - in
    /// seconds x 10
    pub averaging_time: u16,
    /// Acquisition range set by the tester to reach the end of the fibre - as
    /// with other distance measurements, 100ps increments
    pub acquisition_range: i32,
    /// Acquisition range in 10x distance units, as an alternate to
    /// acquisition_range
    pub acquisition_range_distance: i32,
    /// Front panel offset is the time taken, in 100ps increments, between the
    /// front-end of the optical TRX and the front panel connector
    pub front_panel_offset: i32,
    /// Noise floor level - the lowest power level for which 98% of the noise
    /// data lies below; 5-digit -dB value (e.g. 10200 = -10.2dB)
    pub noise_floor_level: u16,
    /// Scale factor for the noise floor level - defaults to 1
    pub noise_floor_scale_factor: i16,
    /// Attenuation in dB*1000 applied by the instrument if done by the
    /// instrument
    pub power_offset_first_point: u16,
    /// The threshold in dB*1000 for a loss-type event; default 00200
    pub loss_threshold: u16,
    /// The threshold in -dB*1000 for reflectance events; default -55000
    pub reflectance_threshold: u16,
    /// The threshold in dB*1000 for the loss taken to detect the end of the
    /// fibre; default 03000
    pub end_of_fibre_threshold: u16,
    /// Trace type - identifies if this is a standard one-way trace, a
    /// bidirectional trace, reference trace, difference trace, or reversed
    /// trace
    pub trace_type: String,
    /// Window coordinate for the upper right window corner
    pub window_coordinate_1: i32,
    /// Power coordinate for the upper right window corner
    pub window_coordinate_2: i32,
    /// Window coordinate for the lower left window corner
    pub window_coordinate_3: i32,
    /// Power coordinate for the lower left window corner
    pub window_coordinate_4: i32,
}

/// KeyEvents describe a single event along the fibre path detected by the OTDR
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct KeyEvent {
    /// Event number - this is from 0 to n
    pub event_number: i16,
    /// Event propogation time is the time in 100ps units from the front panel
    /// to the event
    pub event_propogation_time: i32,
    /// The span loss in db/km (as a 5-digit value, i.e. dB*1000) for the fibre
    /// entering the event
    pub attenuation_coefficient_lead_in_fiber: i16,
    /// Loss in dB*1000 for the event
    pub event_loss: i16,
    /// Reflectance in -dB*1000 for the event
    pub event_reflectance: i32,
    /// Code for the event is a 6-byte string:
    /// Byte 1:
    ///     0 = nonreflective, 1 = reflective, 2 = saturated reflective
    /// Byte 2:
    ///     A = added by user, M = moved by user, E = end of fibre, F = found
    ///     by software, O = out of range, D = modified end of fibre
    /// Remaining bytes are the Landmark number if used - 9s otherwise
    pub event_code: String,
    /// Loss measurement technique - 2P for two point, LS for least squares, OT
    /// for other
    pub loss_measurement_technique: String,
    /// Marker location - ML1 is the OTDR side for 2P/LS/OT measurements
    pub marker_location_1: i32,
    /// Marker location - ML2 is the OTDR side for LS measurements, and bounds
    /// the event for 2P/OT
    pub marker_location_2: i32,
    /// Marker location - ML3 is on the far side for LS measurements, and empty
    /// for 2P/OT
    pub marker_location_3: i32,
    /// Marker location - ML4 is on the far side for LS measurements, and empty
    /// for 2P/OT
    pub marker_location_4: i32,
    /// Marker location - ML5 is the reflectance calculation position
    pub marker_location_5: i32,
    /// Free comment on the event
    pub comment: String,
}

/// The last key event is as the KeyEvent, with some additional fields; see
/// KeyEvent for the documentation of other fields
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct LastKeyEvent {
    pub event_number: i16,
    pub event_propogation_time: i32,
    pub attenuation_coefficient_lead_in_fiber: i16,
    pub event_loss: i16,
    pub event_reflectance: i32,
    pub event_code: String,
    pub loss_measurement_technique: String,
    pub marker_location_1: i32,
    pub marker_location_2: i32,
    pub marker_location_3: i32,
    pub marker_location_4: i32,
    pub marker_location_5: i32,
    pub comment: String,
    /// End to end loss is in dB*1000 and measures the loss between the two
    /// markers defined below
    pub end_to_end_loss: i32,
    /// Start of the measurement span - typically user offset
    pub end_to_end_marker_position_1: i32,
    /// End of the measurement span - typically end of fibre event position
    pub end_to_end_marker_position_2: i32,
    /// Return loss in dB*1000 for the markers defined below
    pub optical_return_loss: u16,
    /// Start of the measurement span - typically user offset
    pub optical_return_loss_marker_position_1: i32,
    /// End of the measurement span - typically end of fibre event position
    pub optical_return_loss_marker_position_2: i32,
}

/// List of key events and a pointer to the last key event
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct KeyEvents {
    pub number_of_key_events: i16,
    pub key_events: Vec<KeyEvent>,
    pub last_key_event: LastKeyEvent,
}

/// Landmarks are a slightly esoteric feature not often used in SOR files for
/// field test equipment. They act to relate OTDR events to real-world
/// information such as WGS84 GPS data, known fibre MFDs, metre markers, etc
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct Landmark {
    pub landmark_number: i16,
    /// Landmark code identifies the landmark - see page 27 of the standard for
    /// the list
    pub landmark_code: String,
    /// Location in 100ps from user offset to the landmark
    pub landmark_location: i32,
    pub related_event_number: i16,
    pub gps_longitude: i32,
    pub gps_latitude: i32,
    /// Fibre correction factor is the difference in 100*% between the optical
    /// path and the cable length; otherwise known as heliax correction
    pub fiber_correction_factor_lead_in_fiber: i16,
    pub sheath_marker_entering_landmark: i32,
    pub sheath_marker_leaving_landmark: i32,
    pub units_of_sheath_marks_leaving_landmark: String,
    pub mode_field_diameter_leaving_landmark: i16,
    pub comment: String,
}

/// DataPointsAtScaleFactor is the struct that actually contains the data
/// points of the measurements for a given scale factor
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct DataPointsAtScaleFactor {
    /// Number of points in this block
    pub n_points: i32,
    /// Scale factor for the data, as 1000*SF
    pub scale_factor: i16,
    /// Data points as dB*1000
    pub data: Vec<u16>,
}

/// DataPoints holds all the different datasets in this file - one per scale
/// factor
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct DataPoints {
    pub number_of_data_points: i32,
    pub total_number_scale_factors_used: i16,
    pub scale_factors: Vec<DataPointsAtScaleFactor>,
}

/// LinkParameters are a bit esoteric and not often found in test equipment,
/// more the likes of network management systems.
/// Contains a set of landmarks which describe the physical fibre path and may
/// relate this to described KeyEvents
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct LinkParameters {
    pub number_of_landmarks: i16,
    pub landmarks: Vec<Landmark>,
}

/// ProprietaryBlock is a struct to contain third-party proprietary information.
/// This is mostly used for vendor-specific special sauce, extra data, extra
/// analysis, etc.
/// otdrs extracts the header, and stores the data as an array of bytes.
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]

pub struct ProprietaryBlock {
    pub header: String,
    pub data: Vec<u8>,
}

// ChecksumBlock stores a checksum value, computed from 0xffff.
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]

pub struct ChecksumBlock {
    pub checksum: i16,
}


/// SORFile describes a full SOR file. All blocks except MapBlock are Option
/// types as we cannot guarantee the parser will find them, but many blocks are
/// in fact mandatory in the specification so compliant files will provide them.
#[derive(Debug, PartialEq, Serialize, Clone)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
#[cfg_attr(feature = "arbitrary", derive(arbitrary::Arbitrary))]
pub struct SORFile {
    pub map: MapBlock,
    pub general_parameters: Option<GeneralParametersBlock>,
    pub supplier_parameters: Option<SupplierParametersBlock>,
    pub fixed_parameters: Option<FixedParametersBlock>,
    pub key_events: Option<KeyEvents>,
    pub link_parameters: Option<LinkParameters>,
    pub data_points: Option<DataPoints>,
    pub proprietary_blocks: Vec<ProprietaryBlock>,
    pub checksum: Option<ChecksumBlock>,
}


/// Informational checksum validation status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
pub enum ChecksumStatus {
    /// No checksum block was present.
    Missing,
    /// A checksum block exists and at least one strategy matched the stored value.
    Valid,
    /// A checksum block exists but no strategy matched.
    Mismatch,
    /// The checksum block is present but appears truncated or offsets cannot be derived safely.
    Error,
}

/// Strategy that produced a match (if any).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
pub enum ChecksumStrategy {
    /// CRC over all bytes before the checksum block (i.e., Map + all prior blocks).
    PrecedingBytes,
    /// CRC over entire file with only the checksum field (2 bytes) zeroed.
    WholeFileChecksumZeroed,
    /// CRC over entire file excluding the entire checksum block ("Cksum\0" + 2 bytes).
    WholeFileExcludingBlock,
}

/// Result of checksum validation.
#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "python", pyclass(frozen, eq, module = "otdrs", get_all))]
pub struct ChecksumValidationResult {
    pub status: ChecksumStatus,
    pub stored: Option<u16>,
    pub matched: Option<u16>,
    pub matched_by: Option<ChecksumStrategy>,
}
