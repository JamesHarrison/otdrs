class BlockInfo:
    """Details about a specific SOR block"""

    identifier: str
    revision_number: int
    size: int

class MapBlock:
    """File layout information.
    Every SOR file has a MapBlock which acts as a map to the file's contents"""

    revision_number: int
    block_size: int
    block_count: int
    block_info: list[BlockInfo]

class GeneralParametersBlock:
    """General details about this trace"""

    language_code: str
    """Language code - EN, CN, JP, etc."""
    cable_id: str
    """Cable identifier"""
    fiber_id: str
    """Fibre identifier"""
    fiber_type: int
    """Fibre type - this is generally coded as the ITU-T standard definition"""
    nominal_wavelength: int
    """Nominal test wavelength in nm"""
    originating_location: str
    """Start location for the test"""
    terminating_location: str
    """End location for the test"""
    cable_code: str
    """Cable code - free field"""
    current_data_flag: str
    """ NC for new condition, RC for as-repaired, OT as something else"""
    user_offset: int
    """User offset - This is essentially the launch lead length from the front panel offset (provided in the fixed parameters block), in 100ps increments"""
    user_offset_distance: int
    """This is the same as user_offset, but measured in 10x the distance units of FixedParametersBlock.units_of_distance"""
    operator: str
    """Operator of the unit for the test"""
    comment: str
    """Free comment field"""

class SupplierParametersBlock:
    """Details about the OTDR used in the test"""

    supplier_name: str
    """Manufacturer of the OTDR"""
    otdr_mainframe_id: str
    """Mainframe model number"""
    otdr_mainframe_sn: str
    """Mainframe serial number"""
    optical_module_id: str
    """Optical module model number"""
    optical_module_sn: str
    """Optical module serial number"""
    software_revision: str
    """Software revision"""
    other: str
    """Free text"""

class FixedParametersBlock:
    """Details about the trace that don't vary between datasets"""

    date_time_stamp: int
    """Datestamp - unix epoch seconds, 32-bit. Remember not to do any OTDR tests after 2038."""
    units_of_distance: str
    """Units of distance - km, mt, ft, kf, mi, etc. Typically mt (in civilised nations)"""
    actual_wavelength: int
    """Actual wavelength used - normally the factory-calibrated wavelength in nm, or nominal wavelength"""
    acquisition_offset: int
    """Acquisition offset - this is the length of fibre from the OTDR port to the first data point in the DataPoints, in 100ps increments"""
    acquisition_offset_distance: int
    """As acquisition_offset, but as 10x units_of_distance"""
    total_n_pulse_widths_used: int
    """The total number of pulse widths used, if more than one pulse width's results are stored in the file"""
    pulse_widths_used: list[int]
    """The list of pulse widths used, in nanoseconds"""
    data_spacing: list[int]
    """Data spacing, time taken to acquire 10,000 points in 100ps increments"""
    n_data_points_for_pulse_widths_used: list[int]
    """Number of points stored for each pulse width"""
    group_index: int
    """Group index - the refractive index of the fibre, default to 146800 if nothing supplied"""
    backscatter_coefficient: int
    """Backscatter coefficient -"""
    number_of_averages: int
    """Number of averages - the number of samples that were averaged to generate the result - may be used instead of averaging time"""
    averaging_time: int
    """Averaging time - may be supplied instead of number of averages - in seconds x 10"""
    acquisition_range: int
    """Acquisition range set by the tester to reach the end of the fibre - as with other distance measurements, 100ps increments"""
    acquisition_range_distance: int
    """Acquisition range in 10x distance units, as an alternate to acquisition_range"""
    front_panel_offset: int
    """Front panel offset is the time taken, in 100ps increments, between the front-end of the optical TRX and the front panel connector"""
    noise_floor_level: int
    """Noise floor level - the lowest power level for which 98% of the noise data lies below; 5-digit -dB value (e.g. 10200 = -10.2dB)"""
    noise_floor_scale_factor: int
    """Scale factor for the noise floor level - defaults to 1"""
    power_offset_first_point: int
    """Attenuation in dB*1000 applied by the instrument if done by the instrument"""
    loss_threshold: int
    """The threshold in dB*1000 for a loss-type event; default 00200"""
    reflectance_threshold: int
    """The threshold in -dB*1000 for reflectance events; default -55000"""
    end_of_fibre_threshold: int
    """The threshold in dB*1000 for the loss taken to detect the end of the fibre; default 03000"""
    trace_type: str
    """Trace type - identifies if this is a standard one-way trace, a bidirectional trace, reference trace, difference trace, or reversed trace"""
    window_coordinate_1: int
    """Window coordinate for the upper right window corner"""
    window_coordinate_2: int
    """Power coordinate for the upper right window corner"""
    window_coordinate_3: int
    """Window coordinate for the lower left window corner"""
    window_coordinate_4: int
    """Power coordinate for the lower left window corner"""

class KeyEvent:
    """Event detected by the OTDR"""

    event_number: int
    """Event number - this is from 0 to n"""

    event_propogation_time: int
    """Event propogation time is the time in 100ps units from the front panel to the event"""

    attenuation_coefficient_lead_in_fiber: int
    """The span loss in db/km (as a 5-digit value, i.e. dB*1000) for the fibre entering the event"""

    event_loss: int
    """Loss in dB*1000 for the event"""

    event_reflectance: int
    """Reflectance in -dB*1000 for the event"""

    event_code: str
    """Code for the event is a 6-byte string:
    Byte 1:
        0 = nonreflective, 1 = reflective, 2 = saturated reflective
    Byte 2:
        A = added by user, M = moved by user, E = end of fibre, F = found
        by software, O = out of range, D = modified end of fibre
    Remaining bytes are the Landmark number if used - 9s otherwise
    """

    loss_measurement_technique: str
    """Loss measurement technique - 2P for two point, LS for least squares, OT for other"""

    marker_location_1: int
    """Marker location - ML1 is the OTDR side for 2P/LS/OT measurements"""

    marker_location_2: int
    """Marker location - ML2 is the OTDR side for LS measurements, and bounds the event for 2P/OT"""

    marker_location_3: int
    """Marker location - ML3 is on the far side for LS measurements, and empty for 2P/OT"""

    marker_location_4: int
    """Marker location - ML4 is on the far side for LS measurements, and empty for 2P/OT"""

    marker_location_5: int
    """Marker location - ML5 is the reflectance calculation position"""

    comment: str
    """Free comment on the event"""

class LastKeyEvent:
    """Like a KeyEvent, but specific to the last event detected"""

    event_number: int
    event_propogation_time: int
    attenuation_coefficient_lead_in_fiber: int
    event_loss: int
    event_reflectance: int
    event_code: str
    loss_measurement_technique: str
    marker_location_1: int
    marker_location_2: int
    marker_location_3: int
    marker_location_4: int
    marker_location_5: int
    comment: str
    # End to end loss is in dB*1000 and measures the loss between the two
    # markers defined below
    end_to_end_loss: int
    # Start of the measurement span - typically user offset
    end_to_end_marker_position_1: int
    # End of the measurement span - typically end of fibre event position
    end_to_end_marker_position_2: int
    # Return loss in dB*1000 for the markers defined below
    optical_return_loss: int
    # Start of the measurement span - typically user offset
    optical_return_loss_marker_position_1: int
    # End of the measurement span - typically end of fibre event position
    optical_return_loss_marker_position_2: int

class KeyEvents:
    """The key events the OTDR detected."""

    number_of_key_events: int
    key_events: list[KeyEvent]
    last_key_event: LastKeyEvent

class DataPointsAtScaleFactor:
    """The data points for a specific scale factor"""

    n_points: int
    """Number of points in this block"""
    scale_factor: int
    """Scale factor for the data, as 1000*SF"""
    data: list[int]
    """Data points as dB*1000"""

class DataPoints:
    """The data points for this trace"""

    number_of_data_points: int
    total_number_scale_factors_used: int
    scale_factors: list[DataPointsAtScaleFactor]

class Landmark:
    """Not widely used; a landmark recorded in the trace by the system."""

    landmark_number: int
    # Landmark code identifies the landmark - see page 27 of the standard for
    # the list
    landmark_code: str
    # Location in 100ps from user offset to the landmark
    landmark_location: int
    related_event_number: int
    gps_longitude: int
    gps_latitude: int
    # Fibre correction factor is the difference in 100*% between the optical
    # path and the cable length; otherwise known as heliax correction
    fiber_correction_factor_lead_in_fiber: int
    sheath_marker_entering_landmark: int
    sheath_marker_leaving_landmark: int
    units_of_sheath_marks_leaving_landmark: str
    mode_field_diameter_leaving_landmark: int
    comment: str

class LinkParameters:
    """Landmark information for the trace"""

    number_of_landmarks: int
    landmarks: list[Landmark]

class ProprietaryBlock:
    """Binary proprietary block data."""

    header: str
    data: list[int]

class ChecksumBlock:
    """File checksum block."""
    checksum: int


class SORFile:
    """A SOR file.
    SORFile describes a full SOR file. All blocks except MapBlock are optional types as we cannot
    guarantee the parser will find them, but many blocks are in fact mandatory in the specification,
    so compliant instruments will provide them.
    """

    map: MapBlock
    general_parameters: GeneralParametersBlock | None
    supplier_parameters: SupplierParametersBlock | None
    fixed_parameters: FixedParametersBlock | None
    key_events: KeyEvents | None
    link_parameters: LinkParameters | None
    data_points: DataPoints | None
    proprietary_blocks: list[ProprietaryBlock]
    checksum: ChecksumBlock | None

    def to_bytes(self) -> bytes:
        """Returns the SOR file as a byte string."""
        ...

    def write_file(self, path: str) -> None:
        """Writes the SOR file to the given path."""
        ...

    def validate_checksum(self, data: bytes) -> enum.Enum:
        """Validates the specified raw bytes using the parsed file data.

        Checksums for OTDR data are optional and poorly-defined,
        so parsing the file before performing the checksum allows us to be smart about
        trying different strategies for how the checksum might have been computed.

        Returns an enum. You should generally treat a missing checksum as non-fatal."""
        ...

def parse_file(path: str) -> SORFile:
    """Load a SOR from the given path and parse it"""

def parse_bytes(bytes: bytes) -> SORFile:
    """Parse a SOR file from the given bytes"""
