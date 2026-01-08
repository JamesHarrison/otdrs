"""Random OTDR configuration generator for training data creation.

Generates valid randomized OTDR simulation configurations that can be
passed to the Rust simulator to create synthetic training data.
"""

import random
import uuid
from dataclasses import dataclass, field
from enum import Enum
from typing import Any


class FiberType(str, Enum):
    """Standard fiber types."""

    G652D = "G652D"
    G657A1 = "G657A1"
    G657A2 = "G657A2"
    G657B3 = "G657B3"


class ConnectorType(str, Enum):
    """Connector types."""

    UPC = "UPC"
    APC = "APC"


class SpliceType(str, Enum):
    """Splice types."""

    FUSION = "fusion"
    MECHANICAL = "mechanical"


class EndType(str, Enum):
    """Fiber end types (for non-connector terminations)."""

    CLEAVED = "cleaved"  # Clean cleaved end - high Fresnel reflection (~-14dB)
    BROKEN = "broken"  # Broken/shattered end - lower, broader reflection
    TERMINATED = "terminated"  # Terminated with index-matching - low/no reflection


@dataclass
class ConfigBounds:
    """Bounds for random configuration generation."""

    # Fiber length bounds (meters)
    min_total_length_m: float = 100.0
    max_total_length_m: float = 100_000.0
    
    # Length distribution weighting
    short_run_prob: float = 0.8  # Probability of length < 10km
    short_run_max_m: float = 10_000.0

    # Segment counts
    min_segments: int = 1
    max_segments: int = 10

    # Event counts
    min_splices: int = 0
    max_splices: int = 6  # Reduced from 20 to crowd less
    min_connectors: int = 1  # At least start connector
    max_connectors: int = 5  # Reduced from 10
    min_macrobends: int = 0
    max_macrobends: int = 2  # Reduced from 4

    # Splice characteristics
    splice_loss_min_db: float = 0.02
    splice_loss_max_db: float = 0.5

    # Connector characteristics
    connector_loss_min_db: float = 0.1
    connector_loss_max_db: float = 0.5
    connector_return_loss_min_db: float = 40.0
    connector_return_loss_max_db: float = 60.0

    # Macrobend characteristics
    macrobend_radius_min_mm: float = 10.0
    macrobend_radius_max_mm: float = 30.0
    macrobend_turns_min: float = 0.5
    macrobend_turns_max: float = 5.0
    macrobend_span_min_m: float = 0.1
    macrobend_span_max_m: float = 1.0

    # Acquisition settings
    wavelengths: list[int] = field(default_factory=lambda: [1310, 1550, 1625])
    
    # Discrete averaging times (seconds)
    averaging_times: list[float] = field(default_factory=lambda: [5.0, 10.0, 15.0, 30.0, 60.0, 120.0, 180.0])
    
    # Standard OTDR ranges (meters)
    ranges: list[float] = field(default_factory=lambda: [
        500.0, 1_000.0, 2_500.0, 5_000.0, 10_000.0, 20_000.0, 40_000.0, 80_000.0, 160_000.0, 260_000.0
    ])

    # Hardware settings
    noise_floor_min_dbm: float = -70.0
    noise_floor_max_dbm: float = -55.0
    dynamic_range_min_db: float = 25.0
    dynamic_range_max_db: float = 45.0
    
    # Discrete front panel offsets (meters)
    front_panel_offsets: list[float] = field(default_factory=lambda: [50.0, 100.0, 150.0])
    
    # Discrete launch cable lengths (meters)
    launch_cable_lengths: list[float] = field(default_factory=lambda: [150.0, 250.0, 500.0, 2200.0])


def _get_pulse_width_options(range_m: float) -> list[float]:
    """Get valid pulse width options (ns) for a given range."""
    # Standard pulse widths: 3, 5, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000, 20000
    if range_m <= 500:
        return [3.0, 5.0, 10.0]
    elif range_m <= 1000:
        return [5.0, 10.0, 20.0]
    elif range_m <= 2500:
        return [10.0, 20.0, 50.0]
    elif range_m <= 5000:
        return [20.0, 50.0, 100.0]
    elif range_m <= 10000:
        return [50.0, 100.0, 200.0]
    elif range_m <= 20000:
        return [100.0, 200.0, 500.0]
    elif range_m <= 40000:
        return [200.0, 500.0, 1000.0]
    elif range_m <= 80000:
        return [500.0, 1000.0, 2000.0]
    elif range_m <= 160000:
        return [1000.0, 2000.0, 5000.0, 10000.0]
    else:
        return [5000.0, 10000.0, 20000.0]


def generate_random_config(
    bounds: ConfigBounds | None = None,
    seed: int | None = None,
) -> dict[str, Any]:
    """Generate a random valid OTDR simulation configuration.

    Args:
        bounds: Configuration bounds. Uses defaults if None.
        seed: Random seed for reproducibility. Uses random if None.

    Returns:
        Dictionary configuration in the format expected by the simulator.
    """
    if bounds is None:
        bounds = ConfigBounds()

    if seed is not None:
        random.seed(seed)
    else:
        seed = random.randint(0, 2**32 - 1)

    # Generate total fiber length with bias towards shorter runs
    if random.random() < bounds.short_run_prob:
        # Short run (<10km)
        total_length = random.uniform(bounds.min_total_length_m, bounds.short_run_max_m)
    else:
        # Long run (>10km)
        total_length = random.uniform(bounds.short_run_max_m, bounds.max_total_length_m)
    
    # Round to avoid floating point issues with events at the end
    total_length = round(total_length, 2)

    # Generate segments
    num_segments = random.randint(bounds.min_segments, bounds.max_segments)
    segments = _generate_segments(total_length, num_segments)

    # Recompute total_length from actual segment lengths to avoid floating-point mismatch
    actual_total_length = sum(s["length_m"] for s in segments)
    
    # Generate events using the actual segment total
    events = _generate_events(actual_total_length, bounds)

    # Determine measurement range (smallest standard range that fits)
    # Add 10-20% margin to fiber length to map to range
    required_range = total_length * 1.1
    valid_ranges = [r for r in bounds.ranges if r >= required_range]
    
    if not valid_ranges:
        # Fallback if fiber is longer than max standard range
        range_m = bounds.ranges[-1]
    else:
        # Usually pick the tightest fit, but occasionally pick one size up (10% chance)
        best_fit_idx = 0
        if len(valid_ranges) > 1 and random.random() < 0.1:
            best_fit_idx = 1
        range_m = valid_ranges[best_fit_idx]

    # Select wavelengths (1-3 wavelengths)
    num_wavelengths = random.randint(1, len(bounds.wavelengths))
    wavelengths = random.sample(bounds.wavelengths, num_wavelengths)

    # Build acquisition config
    wavelength_configs = []
    pulse_options = _get_pulse_width_options(range_m)
    
    for wl in sorted(wavelengths):
        wavelength_configs.append({
            "wavelength_nm": wl,
            "pulse_width_ns": random.choice(pulse_options),
        })

    # Determine if launch cable is used
    launch_cable = None
    # 30% chance of launch cable
    if random.random() < 0.3:
        launch_cable = {
            "length_m": random.choice(bounds.launch_cable_lengths),
            "connector_type": random.choice(list(ConnectorType)).value,
            "connector_loss_db": random.uniform(0.1, 0.3),
        }

    # Generate a UUID for the simulation ID
    sim_id = str(uuid.uuid4())

    config = {
        "simulation": {
            "id": sim_id,
            "seed": seed,
            "output_dir": "./output",  # Will be overridden
            "output_basename": sim_id,  # Use UUID as basename
        },
        "network": {
            "name": f"Network {sim_id[:8]}",
            "segments": segments,
            "events": events,
        },
        "acquisition": {
            "wavelengths": wavelength_configs,
            "range_m": range_m,
            "averaging_time_s": random.choice(bounds.averaging_times),
            # pulse_shape will be set when writing JSON with proper object format
        },
        "hardware": {
            "noise_floor_dbm": random.uniform(
                bounds.noise_floor_min_dbm, bounds.noise_floor_max_dbm
            ),
            "dynamic_range_db": random.uniform(
                bounds.dynamic_range_min_db, bounds.dynamic_range_max_db
            ),
            "detector_bandwidth_mhz": random.uniform(30, 100),
            "saturation_power_mw": random.uniform(0.05, 0.2),
            "event_dead_zone_m": random.uniform(0.5, 1.5),
            "attenuation_dead_zone_m": random.uniform(2.0, 5.0),
            "group_index": 1.4682,  # Standard for SMF
            "injection_power_mw": 10 ** (random.uniform(3.0, 13.0) / 10.0), # 3dBm to 13dBm
            "front_panel_offset_m": random.choice(bounds.front_panel_offsets),
        },
    }

    if launch_cable:
        config["acquisition"]["launch_cable"] = launch_cable

    return config


def _generate_segments(total_length: float, num_segments: int) -> list[dict[str, Any]]:
    """Generate random fiber segments that sum to total_length."""
    
    # We want longer segments in the middle to avoid too many mode field shifts there.
    # We can achieve this by biasing the proportions.
    # 30% chance of "uniform" segments, 70% chance of "middle-heavy" segments
    if num_segments > 2 and random.random() < 0.7:
        # Generate proportions that favor the middle
        # Using a bell curve shape for segment *length* means bigger in middle
        # We can just pick random weights and scale them?
        # A simple way simulates "few in intermediate":
        # Just use random dirichlet?
        # Let's try: random weights, but boost the middle ones
        weights = []
        for i in range(num_segments):
            base_w = random.random()
            # If it's a middle segment (not first or last), boost it
            if 0 < i < num_segments - 1:
                base_w += 2.0  # Boost middle segments
            weights.append(base_w)
        
        total_w = sum(weights)
        proportions = [w / total_w for w in weights]
    else:
        # Standard random cut points (effectively Dirichlet(1,1,...))
        cut_points = sorted([random.random() for _ in range(num_segments - 1)])
        proportions = []
        last = 0.0
        for cp in cut_points:
            proportions.append(cp - last)
            last = cp
        proportions.append(1.0 - last)

    # Generate segments
    segments = []
    current_length = 0.0
    
    for i, prop in enumerate(proportions):
        if i == num_segments - 1:
            # Last segment takes the remaining length to match total exactly
            # Use round() to avoid floating point accumulation errors
            length = round(total_length - current_length, 2)
        else:
            length = round(prop * total_length, 2)
            
        current_length += length
        
        fiber_type = random.choice(list(FiberType))
        segment: dict[str, Any] = {
            "length_m": length,
            "fiber_type": fiber_type.value,
        }
        
        # Occasionally add MFD overrides (10% chance)
        if random.random() < 0.1:
            segment["mfd_1310_override"] = random.uniform(8.0, 10.5)
            segment["mfd_1550_override"] = random.uniform(9.0, 11.5)

        segments.append(segment)

    return segments


def _generate_events(total_length: float, bounds: ConfigBounds) -> list[dict[str, Any]]:
    """Generate random events along the fiber."""
    events = []

    # Always have a start connector at position 0
    events.append(_generate_connector(0.0, bounds, is_start=True))

    # To simulate crowded ends and sparse middle, we use a Beta(0.5, 0.5) distribution
    # which is U-shaped (higher probability near 0 and 1).
    def random_pos(min_m: float, max_m: float) -> float:
        # Beta variate gives [0, 1]. We map it to [min_m, max_m]
        # alpha=0.4, beta=0.4 gives a slightly stronger U-shape than 0.5
        u = random.betavariate(0.4, 0.4)
        return min_m + u * (max_m - min_m)

    # Generate splices
    num_splices = random.randint(bounds.min_splices, bounds.max_splices)
    splice_positions = sorted([random_pos(100, total_length - 100) for _ in range(num_splices)])
    for pos in splice_positions:
        events.append(_generate_splice(pos, bounds))

    # Generate additional connectors (not at start)
    num_connectors = random.randint(0, bounds.max_connectors - 1)
    connector_positions = sorted(
        [random_pos(100, total_length - 100) for _ in range(num_connectors)]
    )
    for pos in connector_positions:
        events.append(_generate_connector(pos, bounds))

    # Generate macrobends (randomly placed, maybe faults can happen anywhere?)
    # Let's bias them too, assuming faults happen at stress points (often ends)
    num_macrobends = random.randint(bounds.min_macrobends, bounds.max_macrobends)
    macrobend_positions = sorted(
        [random_pos(100, total_length - 100) for _ in range(num_macrobends)]
    )
    for pos in macrobend_positions:
        events.append(_generate_macrobend(pos, bounds))

    # End connector/termination
    # Distribution: 50% connector, 50% non-connector end (cleaved, broken, terminated)
    end_termination = random.choice(["connector", "end"])
    if end_termination == "connector":
        events.append(_generate_connector(total_length, bounds, is_end=True))
    else:
        # Generate explicit end event for non-connector terminations
        events.append(_generate_end_event(total_length))

    # Sort all events by position
    events.sort(key=lambda e: e["position_m"])

    return events


def _generate_splice(position: float, bounds: ConfigBounds) -> dict[str, Any]:
    """Generate a random splice event."""
    return {
        "type": "splice",
        "position_m": round(position, 2),
        "insertion_loss_db": random.uniform(bounds.splice_loss_min_db, bounds.splice_loss_max_db),
        "splice_type": random.choice(list(SpliceType)).value,
    }


def _generate_connector(
    position: float,
    bounds: ConfigBounds,
    is_start: bool = False,
    is_end: bool = False,
) -> dict[str, Any]:
    """Generate a random connector event."""
    connector_type = random.choice(list(ConnectorType))
    insertion_loss = random.uniform(bounds.connector_loss_min_db, bounds.connector_loss_max_db)

    # Determine state
    return_loss_db = None
    if is_end and random.random() < 0.3:
        # 30% chance end connector is open
        state = "open"
    else:
        state = "mated"
        return_loss_db = random.uniform(
            bounds.connector_return_loss_min_db, bounds.connector_return_loss_max_db
        )

    # For end connectors, subtract epsilon to avoid floating-point issues
    # 0.01m = 1cm tolerance is insignificant for OTDR measurements
    if is_end:
        position_rounded = round(position - 0.01, 2)
    else:
        position_rounded = round(position, 2)

    event = {
        "type": "connector",
        "position_m": position_rounded,
        "insertion_loss_db": insertion_loss,
        "connector_type": connector_type.value,
        "state": state,
    }
    
    if return_loss_db is not None:
        event["return_loss_db"] = return_loss_db
        
    return event


def _generate_macrobend(position: float, bounds: ConfigBounds) -> dict[str, Any]:
    """Generate a random macrobend event."""
    return {
        "type": "macrobend",
        "position_m": round(position, 2),
        "bend_radius_mm": random.uniform(bounds.macrobend_radius_min_mm, bounds.macrobend_radius_max_mm),
        "turns": random.uniform(bounds.macrobend_turns_min, bounds.macrobend_turns_max),
        "span_m": random.uniform(bounds.macrobend_span_min_m, bounds.macrobend_span_max_m),
    }


def _generate_end_event(position: float) -> dict[str, Any]:
    """Generate an end-of-fiber event.

    End events represent fiber terminations that are NOT connectors.
    These include cleaved ends, broken fibers, and terminated (index-matched) ends.

    Args:
        position: Position in meters.

    Returns:
        End event configuration.
    """
    # Distribution weighted towards cleaved (most common in practice)
    # 60% cleaved, 25% terminated, 15% broken
    weights = [0.60, 0.25, 0.15]
    end_types = [EndType.CLEAVED, EndType.TERMINATED, EndType.BROKEN]
    end_type = random.choices(end_types, weights=weights, k=1)[0]

    # Subtract epsilon before rounding to ensure the position is strictly
    # less than network length after JSON serialization. This avoids floating-point
    # precision issues where the rounded value might exceed the network length.
    # 0.01m = 1cm tolerance is insignificant for OTDR measurements (resolution ~1m).
    position_safe = round(position - 0.01, 2)

    return {
        "type": "end",
        "position_m": position_safe,
        "end_type": end_type.value,
    }
