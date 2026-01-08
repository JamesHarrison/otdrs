"""Tests for random configuration generator."""

import pytest

from otdr_ml.data.config_generator import (
    ConfigBounds,
    ConnectorType,
    EndType,
    FiberType,
    SpliceType,
    generate_random_config,
)


class TestGenerateRandomConfig:
    """Tests for generate_random_config function."""

    def test_generates_valid_config(self) -> None:
        """Test that generated config has required structure."""
        config = generate_random_config(seed=42)

        assert "simulation" in config
        assert "network" in config
        assert "acquisition" in config
        assert "hardware" in config

    def test_reproducible_with_seed(self) -> None:
        """Test that configs are reproducible with same seed.

        Note: simulation.id and network.name are excluded as they contain UUIDs
        which are not seeded by the random seed.
        """
        config1 = generate_random_config(seed=123)
        config2 = generate_random_config(seed=123)

        # Exclude UUID-based fields from comparison
        def comparable_config(cfg: dict) -> dict:
            result = dict(cfg)
            result["simulation"] = {
                k: v for k, v in cfg["simulation"].items() if k not in ("id", "output_basename")
            }
            result["network"] = dict(cfg["network"])
            result["network"]["name"] = "test"  # Normalize name
            return result

        assert comparable_config(config1) == comparable_config(config2)

    def test_different_with_different_seeds(self) -> None:
        """Test that different seeds produce different configs."""
        config1 = generate_random_config(seed=1)
        config2 = generate_random_config(seed=2)

        # At least the seeds should be different
        assert config1["simulation"]["seed"] != config2["simulation"]["seed"]

    def test_network_has_segments(self) -> None:
        """Test that network contains segments."""
        config = generate_random_config(seed=42)

        segments = config["network"]["segments"]
        assert len(segments) >= 1

        for segment in segments:
            assert "length_m" in segment
            assert segment["length_m"] > 0
            assert "fiber_type" in segment
            assert segment["fiber_type"] in [ft.value for ft in FiberType]

    def test_network_has_events(self) -> None:
        """Test that network contains events."""
        config = generate_random_config(seed=42)

        events = config["network"]["events"]
        assert len(events) >= 1  # At least start connector

        # First event should be at position 0 (start connector)
        start_events = [e for e in events if e["position_m"] == 0.0]
        assert len(start_events) >= 1

    def test_event_types_valid(self) -> None:
        """Test that all event types are valid."""
        config = generate_random_config(seed=42)

        valid_types = {"splice", "connector", "macrobend", "end"}
        for event in config["network"]["events"]:
            assert event["type"] in valid_types, f"Invalid event type: {event['type']}"

    def test_splice_events_valid(self) -> None:
        """Test that splice events have required fields."""
        # Generate until we get some splices
        for seed in range(100):
            config = generate_random_config(seed=seed)
            splices = [e for e in config["network"]["events"] if e["type"] == "splice"]
            if splices:
                break

        for splice in splices:
            assert "insertion_loss_db" in splice
            assert splice["insertion_loss_db"] >= 0
            assert "splice_type" in splice
            assert splice["splice_type"] in [st.value for st in SpliceType]

    def test_connector_events_valid(self) -> None:
        """Test that connector events have required fields."""
        config = generate_random_config(seed=42)

        connectors = [e for e in config["network"]["events"] if e["type"] == "connector"]
        assert len(connectors) >= 1

        for connector in connectors:
            assert "insertion_loss_db" in connector
            assert connector["insertion_loss_db"] >= 0
            assert "connector_type" in connector
            assert connector["connector_type"] in [ct.value for ct in ConnectorType]
            assert "state" in connector

    def test_end_events_valid(self) -> None:
        """Test that end events have required fields."""
        # Generate until we get an end event
        for seed in range(100):
            config = generate_random_config(seed=seed)
            end_events = [e for e in config["network"]["events"] if e["type"] == "end"]
            if end_events:
                break

        assert len(end_events) >= 1, "Should have found at least one end event in 100 configs"

        for end_event in end_events:
            assert "position_m" in end_event
            assert end_event["position_m"] > 0
            assert "end_type" in end_event
            assert end_event["end_type"] in [et.value for et in EndType]

    def test_always_has_end_termination(self) -> None:
        """Test that every config has either a connector at end OR an end event."""
        for seed in range(20):
            config = generate_random_config(seed=seed)
            events = config["network"]["events"]
            total_length = sum(s["length_m"] for s in config["network"]["segments"])

            # Find event at fiber end (within tolerance)
            end_tolerance = 1.0  # 1 meter tolerance
            end_events = [
                e
                for e in events
                if abs(e["position_m"] - total_length) < end_tolerance
            ]

            assert (
                len(end_events) >= 1
            ), f"Seed {seed}: No termination event found at fiber end ({total_length}m)"

            # Verify it's either a connector or end event
            for e in end_events:
                assert e["type"] in (
                    "connector",
                    "end",
                ), f"End event should be connector or end, got {e['type']}"

    def test_acquisition_has_wavelengths(self) -> None:
        """Test that acquisition config has wavelengths."""
        config = generate_random_config(seed=42)

        wavelengths = config["acquisition"]["wavelengths"]
        assert len(wavelengths) >= 1

        for wl_config in wavelengths:
            assert "wavelength_nm" in wl_config
            assert wl_config["wavelength_nm"] in [1310, 1550, 1625]
            assert "pulse_width_ns" in wl_config
            assert wl_config["pulse_width_ns"] > 0

    def test_respects_bounds(self) -> None:
        """Test that config respects provided bounds."""
        bounds = ConfigBounds(
            min_total_length_m=1000.0,
            max_total_length_m=2000.0,
            min_segments=2,
            max_segments=3,
            min_splices=5,
            max_splices=5,
        )

        config = generate_random_config(bounds=bounds, seed=42)

        # Check segment count
        segments = config["network"]["segments"]
        assert 2 <= len(segments) <= 3

        # Check total length
        total_length = sum(s["length_m"] for s in segments)
        assert 1000 <= total_length <= 2000

        # Check splice count
        splices = [e for e in config["network"]["events"] if e["type"] == "splice"]
        assert len(splices) == 5

    def test_hardware_config_present(self) -> None:
        """Test that hardware configuration is present."""
        config = generate_random_config(seed=42)

        hw = config["hardware"]
        assert "noise_floor_dbm" in hw
        assert "dynamic_range_db" in hw
        assert "detector_bandwidth_mhz" in hw
        assert "front_panel_offset_m" in hw

    def test_range_appropriate_for_length(self) -> None:
        """Test that acquisition range exceeds fiber length."""
        for seed in range(10):
            config = generate_random_config(seed=seed)

            total_length = sum(s["length_m"] for s in config["network"]["segments"])
            range_m = config["acquisition"]["range_m"]

            assert range_m > total_length, f"Range {range_m} should exceed length {total_length}"
