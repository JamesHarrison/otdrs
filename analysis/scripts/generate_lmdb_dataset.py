#!/usr/bin/env python3
"""Generate LMDB dataset directly from simulator outputs.

Processes SOR files and ground truth JSON to create LMDB dataset,
bypassing the intermediate HDF5 step for better performance.
"""

import argparse
import json
import os
import pickle
import sys
from concurrent.futures import ProcessPoolExecutor, as_completed
from pathlib import Path

import lmdb
import numpy as np
from tqdm import tqdm

# Add src to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent / "src"))

from otdr_ml.data import (
    WindowConfig,
    extract_windows,
    parse_ground_truth,
    parse_sor_file,
)
from otdr_ml.data.dataset import EVENT_TYPES, NUM_EVENT_TYPES


def find_simulation_pairs(input_dir: Path) -> list[tuple[Path, Path]]:
    """Find matching SOR files and ground truth JSONs.

    Args:
        input_dir: Directory containing simulation outputs.

    Returns:
        List of (sor_path, ground_truth_path) tuples.
    """
    pairs = []

    # Find all ground truth files
    for gt_path in input_dir.glob("*_ground_truth.json"):
        basename = gt_path.stem.replace("_ground_truth", "")

        # Find corresponding SOR files (any wavelength)
        sor_files = list(input_dir.glob(f"{basename}_*nm.sor"))
        for sor_path in sor_files:
            pairs.append((sor_path, gt_path))

    return pairs


def _serialize_window(window, max_events: int) -> dict:
    """Convert TraceWindow to LMDB-compatible sample dict.

    Args:
        window: TraceWindow object with features and labels.
        max_events: Maximum events per window.

    Returns:
        Dictionary with numpy arrays ready for pickle serialization.
    """
    # Ensure power is 2D (seq_len, n_features)
    if window.power.ndim == 1:
        features = window.power.reshape(-1, 1).astype(np.float32)
    else:
        features = window.power.astype(np.float32)

    sample = {
        "features": features,
    }

    # Add labels if present
    if window.labels is not None:
        labels = window.labels

        # Event present (multi-hot encoding)
        sample["event_present"] = np.array(labels["event_present"], dtype=np.float32)

        # Initialize label arrays
        num_events = min(len(labels["events"]), max_events)
        event_positions = np.zeros((max_events, 3), dtype=np.float32)
        event_types = np.zeros(max_events, dtype=np.int64)
        loss_db = np.zeros(max_events, dtype=np.float32)
        reflectance_db = np.zeros(max_events, dtype=np.float32)

        # Fill in event data
        for j, event in enumerate(labels["events"][:max_events]):
            event_positions[j] = [
                event["start_pos"],
                event["center_pos"],
                event["end_pos"],
            ]
            event_types[j] = event["event_type"]

        # Fill in measurements
        for j, loss in enumerate(labels["loss_db"][:max_events]):
            loss_db[j] = loss
        for j, refl in enumerate(labels["reflectance_db"][:max_events]):
            reflectance_db[j] = refl

        sample["event_positions"] = event_positions
        sample["event_types"] = event_types
        sample["loss_db"] = loss_db
        sample["reflectance_db"] = reflectance_db
        sample["num_events"] = num_events

    return sample


def process_simulation(
    sor_path: Path,
    gt_path: Path,
    config: WindowConfig,
    max_events: int,
) -> tuple[list[dict], str | None]:
    """Process a single simulation to extract serialized samples.

    Args:
        sor_path: Path to SOR file.
        gt_path: Path to ground truth JSON.
        config: Window extraction configuration.
        max_events: Maximum events per window.

    Returns:
        Tuple of (list of sample dicts, error message if any).
    """
    try:
        # Parse files
        trace = parse_sor_file(sor_path)
        ground_truth = parse_ground_truth(gt_path)

        # Get pulse width from trace
        pulse_width_ns = (
            trace.metadata.pulse_widths_ns[0] if trace.metadata.pulse_widths_ns else 100.0
        )

        # Extract windows
        windows = extract_windows(
            trace,
            config,
            ground_truth=ground_truth,
            pulse_width_ns=pulse_width_ns,
        )

        # Serialize windows to sample dicts
        samples = [_serialize_window(window, max_events) for window in windows]

        return samples, None

    except Exception as e:
        return [], f"{sor_path.name}: {str(e)}"


def _process_wrapper(args: tuple) -> tuple[list[dict], str | None]:
    """Wrapper for parallel execution."""
    return process_simulation(*args)


def generate_lmdb_dataset(
    input_dir: Path,
    output_path: Path,
    window_size_m: float = 1000.0,
    target_points: int = 512,
    max_events_per_window: int = 5,
    max_workers: int | None = None,
    map_size: int = 10**12,
    commit_interval: int = 5000,
) -> dict:
    """Generate LMDB dataset directly from simulation outputs.

    Streams results to LMDB as workers complete processing,
    minimizing memory usage for large datasets.

    Args:
        input_dir: Directory containing simulation outputs.
        output_path: Output LMDB directory path.
        window_size_m: Window size in meters.
        target_points: Target number of points per window.
        max_events_per_window: Maximum events per window.
        max_workers: Number of worker processes (default: CPU count).
        map_size: Maximum LMDB size in bytes (default: 1TB).
        commit_interval: Commit transaction every N samples.

    Returns:
        Dictionary with generation statistics.
    """
    print(f"Input directory: {input_dir}")
    print(f"Output LMDB: {output_path}")

    # Find simulation pairs
    pairs = find_simulation_pairs(input_dir)
    print(f"Found {len(pairs)} SOR/ground-truth pairs")

    if not pairs:
        raise ValueError("No simulation pairs found in input directory")

    # Configure windowing
    config = WindowConfig(
        window_size_m=window_size_m,
        target_points=target_points,
    )

    # Determine workers
    if max_workers is None:
        max_workers = os.cpu_count() or 1

    print(f"Processing with {max_workers} workers...")

    # Ensure output directory exists
    output_path = Path(output_path)
    output_path.mkdir(parents=True, exist_ok=True)

    # Open LMDB environment
    env = lmdb.open(str(output_path), map_size=map_size)

    # Statistics
    total_samples = 0
    errors = []
    n_features = None
    has_labels = None

    # Error log file
    error_log_path = output_path / "errors.log"
    error_log = open(error_log_path, "w")

    try:
        # Start transaction
        txn = env.begin(write=True)

        # Process simulations with parallel workers
        map_args = [
            (sor_path, gt_path, config, max_events_per_window) for sor_path, gt_path in pairs
        ]

        with ProcessPoolExecutor(max_workers=max_workers) as executor:
            # Submit all tasks
            futures = {executor.submit(_process_wrapper, args): args for args in map_args}

            # Process results as they complete
            with tqdm(total=len(pairs), desc="Processing simulations") as pbar:
                for future in as_completed(futures):
                    samples, error = future.result()

                    # Handle errors
                    if error:
                        errors.append(error)
                        error_log.write(f"{error}\n")
                        error_log.flush()
                        pbar.update(1)
                        continue

                    # Write samples to LMDB
                    for sample in samples:
                        # Detect features on first sample
                        if n_features is None:
                            n_features = sample["features"].shape[-1]
                            has_labels = "event_present" in sample
                            print(
                                f"\nDetected {n_features} features per sample, labels: {has_labels}"
                            )

                        # Serialize and write
                        key = f"{total_samples:08d}".encode("ascii")
                        txn.put(key, pickle.dumps(sample))
                        total_samples += 1

                        # Commit periodically
                        if total_samples % commit_interval == 0:
                            txn.commit()
                            txn = env.begin(write=True)

                    pbar.update(1)

        # Final commit
        txn.commit()

        # Write metadata
        metadata = {
            "num_samples": total_samples,
            "n_features": n_features or 1,
            "has_labels": has_labels if has_labels is not None else False,
            "source": str(input_dir),
            "window_size_m": window_size_m,
            "target_points": target_points,
            "max_events_per_window": max_events_per_window,
        }

        with env.begin(write=True) as txn:
            txn.put(b"__metadata__", json.dumps(metadata).encode("utf-8"))

    finally:
        error_log.close()
        env.close()

    print(f"\nGenerated {total_samples} samples")
    print(f"Errors: {len(errors)} (see {error_log_path})")

    if total_samples == 0:
        raise ValueError("No samples generated - check error log")

    # Statistics
    stats = {
        "num_pairs": len(pairs),
        "num_samples": total_samples,
        "num_errors": len(errors),
        "window_size_m": window_size_m,
        "target_points": target_points,
        "n_features": n_features,
    }

    # Save stats
    stats_path = output_path / "stats.json"
    with open(stats_path, "w") as f:
        json.dump(stats, f, indent=2)

    print(f"Dataset saved to {output_path}")
    print(f"Statistics saved to {stats_path}")

    return stats


def main() -> None:
    """Main entry point."""
    parser = argparse.ArgumentParser(
        description="Generate LMDB dataset directly from OTDR simulations"
    )
    parser.add_argument(
        "--input-dir",
        type=Path,
        required=True,
        help="Directory containing simulation outputs (SOR + JSON files)",
    )
    parser.add_argument(
        "--output",
        type=Path,
        required=True,
        help="Output LMDB directory path",
    )
    parser.add_argument(
        "--window-meters",
        type=float,
        default=1000.0,
        help="Window size in meters (default: 1000)",
    )
    parser.add_argument(
        "--target-points",
        type=int,
        default=512,
        help="Target points per window (default: 512)",
    )
    parser.add_argument(
        "--max-events",
        type=int,
        default=5,
        help="Maximum events per window (default: 5)",
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=None,
        help="Number of worker processes (default: CPU count)",
    )
    parser.add_argument(
        "--map-size",
        type=int,
        default=10**12,
        help="Maximum LMDB size in bytes (default: 1TB)",
    )

    args = parser.parse_args()

    try:
        generate_lmdb_dataset(
            input_dir=args.input_dir,
            output_path=args.output,
            window_size_m=args.window_meters,
            target_points=args.target_points,
            max_events_per_window=args.max_events,
            max_workers=args.workers,
            map_size=args.map_size,
        )
    except Exception as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
