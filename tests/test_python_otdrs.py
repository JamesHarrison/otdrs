import otdrs
import os
import tempfile

def test_roundtrip():
    """
    Tests that a SOR file can be read, written to bytes, and read back again
    without changing the content.
    """
    original_sor = otdrs.parse_file("data/example1-noyes-ofl280.sor")

    sor_bytes = original_sor.to_bytes()
    roundtrip_sor = otdrs.parse_bytes(sor_bytes)

    # The map block is re-calculated on write, so it will be different.
    # We can't compare it directly.
    # Let's compare the other blocks.
    assert original_sor.general_parameters == roundtrip_sor.general_parameters
    assert original_sor.supplier_parameters == roundtrip_sor.supplier_parameters
    assert original_sor.fixed_parameters == roundtrip_sor.fixed_parameters
    assert original_sor.key_events == roundtrip_sor.key_events
    assert original_sor.link_parameters == roundtrip_sor.link_parameters
    assert original_sor.data_points == roundtrip_sor.data_points
    assert original_sor.proprietary_blocks == roundtrip_sor.proprietary_blocks

    # Test write_file()
    fd, temp_filename = tempfile.mkstemp(suffix=".sor")
    os.close(fd)

    try:
        original_sor.write_file(temp_filename)
        roundtrip_sor_from_file = otdrs.parse_file(temp_filename)

        assert original_sor.general_parameters == roundtrip_sor_from_file.general_parameters
        assert original_sor.supplier_parameters == roundtrip_sor_from_file.supplier_parameters
        assert original_sor.fixed_parameters == roundtrip_sor_from_file.fixed_parameters
        assert original_sor.key_events == roundtrip_sor_from_file.key_events
        assert original_sor.link_parameters == roundtrip_sor_from_file.link_parameters
        assert original_sor.data_points == roundtrip_sor_from_file.data_points
        assert original_sor.proprietary_blocks == roundtrip_sor_from_file.proprietary_blocks

    finally:
        os.remove(temp_filename)
