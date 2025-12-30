# OTDR Simulator Implementation Plan

This document describes the design and implementation plan for the OTDR simulator crate, which will generate realistic simulated OTDR trace data for testing, validation, and machine learning training purposes.

## Overview

The simulator models the physical processes of optical time-domain reflectometry testing using a **Linear Time-Invariant (LTI) system approach** based on the mathematical formalism described by Feigel et al. (2014). This approach treats the fiber network as a system with an impulse response, allowing arbitrary pulse shapes and accurate modeling of overlapping events.

### Core Simulation Equation

The OTDR trace is computed as:

```
y(z) = h_f(z) * x(z)
```

Where:
- `y(z)` is the detected power at distance z
- `h_f(z) = f_l(z) × [h_b(z) + h_r(z)]` is the fiber system impulse response
- `x(z)` is the OTDR pulse shape
- `h_b(z)` is the backscattering impulse response (Rayleigh scattering)
- `h_r(z)` is the reflection impulse response (Fresnel reflections)
- `f_l(z)` is the cumulative loss function

---

## Architecture

### Crate Structure

```
simulator/
├── Cargo.toml
├── src/
│   ├── lib.rs                  # Library entry point
│   ├── main.rs                 # CLI binary entry point
│   │
│   ├── network/                # Fiber network description
│   │   ├── mod.rs
│   │   ├── segment.rs          # Fiber segment definition
│   │   ├── event.rs            # Event types (splice, connector, etc.)
│   │   ├── fiber_types.rs      # Compiled-in fiber type library
│   │   └── network.rs          # Full network description
│   │
│   ├── acquisition/            # OTDR acquisition parameters
│   │   ├── mod.rs
│   │   ├── params.rs           # Acquisition parameter structs
│   │   ├── pulse.rs            # Pulse shape generation
│   │   └── hardware.rs         # Hardware characteristics
│   │
│   ├── physics/                # Physical simulation engine
│   │   ├── mod.rs
│   │   ├── backscatter.rs      # Rayleigh backscattering
│   │   ├── reflection.rs       # Fresnel reflection
│   │   ├── loss.rs             # Cumulative loss calculation
│   │   ├── convolution.rs      # FFT-based convolution
│   │   └── wavelength.rs       # Wavelength-dependent calculations
│   │
│   ├── postprocess/            # Post-processing chain
│   │   ├── mod.rs
│   │   ├── saturation.rs       # Detector saturation model
│   │   ├── noise.rs            # Gaussian noise generation
│   │   ├── filter.rs           # Detector bandwidth filter
│   │   └── averaging.rs        # Multi-trace averaging
│   │
│   ├── output/                 # Output generation
│   │   ├── mod.rs
│   │   ├── sor.rs              # SOR file generation (via otdrs crate)
│   │   ├── sidecar.rs          # JSON ground truth sidecar
│   │   └── trace.rs            # Internal trace representation
│   │
│   └── config/                 # Configuration handling
│       ├── mod.rs
│       └── yaml.rs             # YAML configuration parsing
│
└── tests/
    ├── integration_tests.rs
    └── fixtures/               # Test YAML configurations
```

---

## Data Structures

### Network Description

```rust
/// Complete description of a fiber network to simulate
pub struct NetworkDescription {
    /// Ordered list of fiber segments making up the network
    pub segments: Vec<FiberSegment>,
    /// Events along the network (splices, connectors, etc.)
    pub events: Vec<NetworkEvent>,
    /// Optional description/name for the network
    pub name: Option<String>,
}

/// A single fiber segment with uniform properties
pub struct FiberSegment {
    /// Length of this segment in meters
    pub length_m: f64,
    /// Fiber type (determines wavelength-dependent parameters)
    pub fiber_type: FiberType,
    /// Optional override for attenuation coefficient (dB/km)
    /// If None, uses fiber_type default for each wavelength
    pub attenuation_override: Option<f64>,
}

/// Compiled-in fiber type definitions
#[derive(Clone, Copy)]
pub enum FiberType {
    /// ITU-T G.652.D - Standard single-mode fiber
    G652D,
    /// ITU-T G.657.A1 - Bend-insensitive fiber (low bend loss)
    G657A1,
    /// ITU-T G.657.A2 - Bend-insensitive fiber (very low bend loss)
    G657A2,
    /// ITU-T G.657.B3 - Bend-insensitive fiber (ultra-low bend loss)
    G657B3,
    /// Custom fiber with explicit parameters
    Custom(CustomFiberParams),
}

/// Parameters for custom fiber types
pub struct CustomFiberParams {
    /// Mode field diameter at 1310nm in micrometers
    pub mfd_1310_um: f64,
    /// Mode field diameter at 1550nm in micrometers  
    pub mfd_1550_um: f64,
    /// Group index of refraction
    pub group_index: f64,
    /// Attenuation at 1310nm in dB/km
    pub attenuation_1310_db_km: f64,
    /// Attenuation at 1550nm in dB/km
    pub attenuation_1550_db_km: f64,
    /// Attenuation at 1625nm in dB/km
    pub attenuation_1625_db_km: f64,
    /// Backscatter coefficient at 1550nm in dB (1ns reference)
    pub backscatter_coeff_db: f64,
}

/// Events that can occur in the fiber network
pub enum NetworkEvent {
    Splice(SpliceEvent),
    Connector(ConnectorEvent),
    Macrobend(MacrobendEvent),
    Splitter(SplitterEvent),
    End(EndEvent),
}

/// Position marker for events
pub struct EventPosition {
    /// Distance from network start in meters
    pub distance_m: f64,
}

/// Fusion or mechanical splice
pub struct SpliceEvent {
    pub position: EventPosition,
    /// Insertion loss in dB (positive value)
    pub insertion_loss_db: f64,
    /// Type of splice affects modeling
    pub splice_type: SpliceType,
}

pub enum SpliceType {
    /// Fusion splice - very low loss, no reflection
    Fusion,
    /// Mechanical splice - slightly higher loss, may have small reflection
    Mechanical { reflectance_db: Option<f64> },
}

/// Optical connector
pub struct ConnectorEvent {
    pub position: EventPosition,
    /// Insertion loss in dB
    pub insertion_loss_db: f64,
    /// Whether this connector is mated (connected) or open
    pub state: ConnectorState,
    /// Connector type affects reflectance characteristics
    pub connector_type: ConnectorType,
}

pub enum ConnectorState {
    /// Mated connector pair - specified return loss
    Mated { return_loss_db: f64 },
    /// Unmated/open connector - high Fresnel reflection
    Open,
}

pub enum ConnectorType {
    /// Ultra Physical Contact - ~4% Fresnel reflection when open
    UPC,
    /// Angled Physical Contact - ~-65dB reflection when open
    APC,
}

/// Macrobend in the fiber
pub struct MacrobendEvent {
    pub position: EventPosition,
    /// Bend radius in millimeters
    pub bend_radius_mm: f64,
    /// Number of turns/wraps
    pub turns: f64,
    /// Span of the macrobend along the fiber in meters
    /// (affects how it appears on trace - distributed vs point loss)
    pub span_m: f64,
}

/// Optical splitter/coupler
pub struct SplitterEvent {
    pub position: EventPosition,
    /// Splitting ratio (e.g., 32 for 1:32 splitter)
    pub split_ratio: u32,
    /// Additional insertion loss beyond ideal splitting loss in dB
    pub excess_loss_db: f64,
}

/// End of fiber
pub struct EndEvent {
    pub position: EventPosition,
    /// End condition
    pub end_type: EndType,
}

pub enum EndType {
    /// Clean cleaved end - high Fresnel reflection (~-14dB)
    Cleaved,
    /// Broken/shattered end - lower, broader reflection
    Broken,
    /// Terminated with index-matching - low/no reflection
    Terminated,
    /// Connected to device with specified return loss
    Connected { return_loss_db: f64 },
}
```

### Acquisition Parameters

```rust
/// Parameters for a single wavelength acquisition
pub struct WavelengthAcquisition {
    /// Wavelength in nanometers
    pub wavelength_nm: u16,
    /// Pulse width in nanoseconds
    pub pulse_width_ns: f64,
}

/// Complete acquisition configuration
pub struct AcquisitionConfig {
    /// List of wavelengths to simulate (all on same fiber)
    pub wavelengths: Vec<WavelengthAcquisition>,
    /// Acquisition range in meters (determines sampling interval)
    pub range_m: f64,
    /// Averaging time in seconds (affects noise level)
    pub averaging_time_s: f64,
    /// Number of averages (alternative to averaging_time)
    pub num_averages: Option<u32>,
    /// Pulse shape to use
    pub pulse_shape: PulseShape,
}

pub enum PulseShape {
    /// Ideal rectangular pulse
    Rectangular,
    /// Hyperbolic secant (mode-locked laser approximation)
    HyperbolicSecant,
    /// Rectangular with finite rise/fall times
    RealRectangular {
        rise_time_ns: f64,
        fall_time_ns: f64,
    },
}
```

### Hardware Characteristics

```rust
/// Models the characteristics of the simulated OTDR hardware
pub struct HardwareConfig {
    /// Noise floor in dBm (RMS)
    pub noise_floor_dbm: f64,
    /// Dynamic range in dB
    pub dynamic_range_db: f64,
    /// Detector bandwidth in MHz
    pub detector_bandwidth_mhz: f64,
    /// Saturation power in mW
    pub saturation_power_mw: f64,
    /// Event dead zone in meters (at shortest pulse)
    pub event_dead_zone_m: f64,
    /// Attenuation dead zone in meters (at shortest pulse)
    pub attenuation_dead_zone_m: f64,
    /// Group index of refraction (default ~1.4682 for SMF)
    pub group_index: f64,
    /// Peak injection power in mW
    pub injection_power_mw: f64,
}

impl Default for HardwareConfig {
    fn default() -> Self {
        // Reasonable defaults for a mid-range OTDR
        Self {
            noise_floor_dbm: -65.0,
            dynamic_range_db: 40.0,
            detector_bandwidth_mhz: 50.0,
            saturation_power_mw: 0.1,
            event_dead_zone_m: 0.8,
            attenuation_dead_zone_m: 3.0,
            group_index: 1.4682,
            injection_power_mw: 1.0,
        }
    }
}
```

### Simulation Output

```rust
/// Complete simulation result for a multi-wavelength run
pub struct SimulationResult {
    /// Unique identifier for this simulation run
    pub simulation_id: Uuid,
    /// Simulated traces, one per wavelength
    pub traces: Vec<SimulatedTrace>,
    /// Ground truth information
    pub ground_truth: GroundTruth,
    /// Configuration used for this simulation
    pub config: SimulationConfig,
}

/// A single simulated OTDR trace
pub struct SimulatedTrace {
    /// Wavelength this trace was acquired at
    pub wavelength_nm: u16,
    /// Pulse width used
    pub pulse_width_ns: f64,
    /// Raw data points (power in internal units)
    pub data_points: Vec<f64>,
    /// Spatial sampling interval in meters
    pub delta_z_m: f64,
    /// Number of data points
    pub num_points: usize,
}

/// Ground truth for ML training and validation
pub struct GroundTruth {
    /// All events with their true parameters
    pub events: Vec<GroundTruthEvent>,
    /// Fiber segments
    pub segments: Vec<GroundTruthSegment>,
    /// Total fiber length
    pub total_length_m: f64,
}

/// Ground truth for a single event
pub struct GroundTruthEvent {
    /// Position along fiber in meters
    pub position_m: f64,
    /// Event type and parameters
    pub event_type: GroundTruthEventType,
}

pub enum GroundTruthEventType {
    Splice {
        insertion_loss_db: f64,
        splice_type: String,
    },
    Connector {
        insertion_loss_db: f64,
        return_loss_db: Option<f64>,
        connector_type: String,
        state: String,
    },
    Macrobend {
        /// Loss at each simulated wavelength
        loss_by_wavelength: HashMap<u16, f64>,
        bend_radius_mm: f64,
        turns: f64,
        span_m: f64,
    },
    Splitter {
        split_ratio: u32,
        total_loss_db: f64,
    },
    End {
        end_type: String,
        return_loss_db: Option<f64>,
    },
}

pub struct GroundTruthSegment {
    /// Start position in meters
    pub start_m: f64,
    /// End position in meters  
    pub end_m: f64,
    /// Fiber type name
    pub fiber_type: String,
    /// Attenuation at each wavelength in dB/km
    pub attenuation_by_wavelength: HashMap<u16, f64>,
}
```

---

## YAML Configuration Format

The CLI tool accepts a YAML configuration file describing the simulation to run:

```yaml
# Simulation configuration
simulation:
  # Optional: seed for reproducibility (random if not specified)
  seed: 12345
  # Output directory for generated files
  output_dir: "./output"
  # Base filename (will append wavelength and .sor/.json)
  output_basename: "simulated_trace"

# Fiber network description
network:
  name: "Example FTTH Drop"
  
  # Define fiber segments in order
  segments:
    - length_m: 1500
      fiber_type: G652D
    - length_m: 500
      fiber_type: G657A1
    - length_m: 200
      fiber_type: G657A2

  # Define events at positions along the network
  events:
    # Mated connector at central office
    - type: connector
      position_m: 0.0
      insertion_loss_db: 0.3
      connector_type: UPC
      state:
        mated:
          return_loss_db: 45.0
    
    # Fusion splice joining segments
    - type: splice
      position_m: 1500.0
      insertion_loss_db: 0.05
      splice_type: fusion
    
    # Macrobend where cable is bent around corner
    - type: macrobend
      position_m: 1800.0
      bend_radius_mm: 15.0
      turns: 2.0
      span_m: 0.5
    
    # Splice to drop cable
    - type: splice
      position_m: 2000.0
      insertion_loss_db: 0.08
      splice_type: fusion
    
    # Customer premises connector
    - type: connector
      position_m: 2200.0
      insertion_loss_db: 0.35
      connector_type: APC
      state: open
    
    # End of fiber (implied at total length, but can be explicit)
    - type: end
      position_m: 2200.0
      end_type: cleaved

# Acquisition settings
acquisition:
  # Multiple wavelengths for the same fiber
  wavelengths:
    - wavelength_nm: 1310
      pulse_width_ns: 10
    - wavelength_nm: 1550
      pulse_width_ns: 10
    - wavelength_nm: 1625
      pulse_width_ns: 10
  
  range_m: 5000
  averaging_time_s: 30
  pulse_shape: hyperbolic_secant

# Hardware characteristics (optional - defaults used if not specified)
hardware:
  noise_floor_dbm: -65
  dynamic_range_db: 40
  detector_bandwidth_mhz: 50
  saturation_power_mw: 0.1
  group_index: 1.4682
  injection_power_mw: 1.0
```

---

## Implementation Phases

### Phase 1: Core Data Structures and Configuration
**Estimated effort: 2-3 days** | **Status: ✅ COMPLETE**

- [x] Set up crate structure and `Cargo.toml` with dependencies
- [x] Implement network description structures (`network/` module)
- [x] Implement compiled-in fiber type library with wavelength-dependent parameters
- [x] Implement acquisition and hardware configuration structures
- [x] Implement YAML configuration parsing
- [x] Add comprehensive validation for configuration

**Completed:** All core data structures implemented with 31 passing tests. Includes:
- Fiber type library with ITU-T standards (G.652.D, G.657.A1/A2/B3)
- Wavelength-dependent characteristics and macrobend loss modeling
- Network events (splices, connectors, macrobends, splitters, ends)
- Acquisition parameters with multi-wavelength support
- YAML configuration parsing and validation

**Key dependencies:**
- `serde`, `serde_yaml` for YAML parsing
- `uuid` for simulation identifiers
- `thiserror` for error handling

---

### Phase 2: Physics Engine - Impulse Response
**Estimated effort: 3-4 days** | **Status: ✅ COMPLETE**

- [x] Implement backscattering impulse response `h_b(z)`
  - Rayleigh backscattering coefficient calculation
  - Wavelength-dependent attenuation
  - Group velocity and index handling
  
- [x] Implement reflection impulse response `h_r(z)`
  - Fresnel reflection from connectors (mated/open)
  - UPC vs APC reflection characteristics
  - End reflection modeling
  
- [x] Implement cumulative loss function `f_l(z)`
  - Track insertion losses through network
  - Handle wavelength-dependent macrobend losses
  - Splitter loss calculation (1/N² model)

- [x] Implement macrobend loss modeling
  - Wavelength-dependent loss curves (higher loss at longer wavelengths)
  - Bend radius and turn count effects
  - Use established bend loss formulas (e.g., from ITU-T G.657)

**Completed:** Full physics engine with 38 tests covering backscatter, reflection, loss, and wavelength dependencies.

**Key formulas:**

Backscattering impulse response:
```
h_b(z) = Δz × K' × 10^(-2αz/10)
```

Where `K'` is the spatial backscatter coefficient derived from the fiber's backscatter coefficient.

Macrobend loss approximation (simplified model):
```
Loss_dB ≈ A × exp(-B × R / λ) × N
```
Where R is bend radius, λ is wavelength, N is number of turns, and A/B are fiber-dependent constants.

---

### Phase 3: Physics Engine - Pulse and Convolution
**Estimated effort: 2-3 days** | **Status: ✅ COMPLETE**

- [x] Implement pulse shape generation
  - Rectangular pulse: `x(z) = P₀ × rect(z/W)`
  - Hyperbolic secant: `x(z) = P₀ × sech²(z/(ηW))`
  - Real rectangular with rise/fall times
  
- [x] Implement FFT-based convolution
  - Use `rustfft` crate for efficient FFT
  - Handle proper padding and normalization
  - Optimize for large traces (100k+ points)

- [x] Combine impulse response: `h_f(z) = f_l(z) × [h_b(z) + h_r(z)]`

**Completed:** Complete convolution engine with 14 tests including large-scale performance tests.

**Key dependencies:**
- `rustfft` for FFT operations

---

### Phase 4: Post-Processing Chain
**Estimated effort: 2-3 days** | **Status: ✅ COMPLETE**

- [x] Implement detector saturation model
  ```
  y_sat(z) = P_sat × tanh(y_LTI(z) / P_sat)
  ```

- [x] Implement Gaussian noise generation
  - Use seeded RNG for reproducibility (Box-Muller transform)
  - Model noise level based on averaging time/count
  - Noise scales as `σ / √N` with N averages
  
- [x] Implement detector bandwidth filter
  - Gaussian low-pass filter approximating detector response
  - Digital implementation using spatial convolution
  
- [x] Implement power clipping and dB conversion
  - Clip negative values before log
  - Convert to dB scale: `y_dB = 5 × log₁₀(y/P_ref)`

- [x] Implement trace averaging simulation
  - Generate multiple noisy traces
  - Average to reduce noise

**Completed:** Full post-processing chain with 25 tests covering saturation, noise, filtering, and averaging.

**Key dependencies:**
- `rand` and `rand_chacha` for reproducible random number generation

---

### Phase 5: Output Generation
**Estimated effort: 2-3 days** | **Status: ✅ COMPLETE**

- [x] Implement SOR file generation using `otdrs` crate
  - Populate `GeneralParametersBlock` with simulation metadata
  - Populate `FixedParametersBlock` with acquisition parameters
  - Generate `DataPointsAtScaleFactor` from simulated trace
  - Generate `KeyEvents` from network events
  
- [x] Implement JSON sidecar generation
  - Serialize `GroundTruth` structure
  - Include simulation configuration for reproducibility
  - Include per-wavelength event characteristics (macrobend loss)

- [x] Implement output file naming and organization
  - Group multi-wavelength outputs together
  - Consistent naming: `{base}_{wavelength}nm.sor`, `{base}_ground_truth.json`

**Completed:** Full output generation with SOR file creation and JSON sidecar. SOR files verified to round-trip through otdrs parser.

**Key dependencies:**
- `otdrs` (workspace dependency)
- `serde_json` for JSON output

---

### Phase 6: CLI Binary
**Estimated effort: 1-2 days** | **Status: ✅ COMPLETE**

- [x] Implement CLI argument parsing
  - Input YAML file path
  - Output directory override
  - Verbosity control
  - Dry-run mode (validate config only)
  
- [x] Implement progress reporting for simulation

- [x] Add validation and helpful error messages

**Completed:** Full CLI with clap-based argument parsing, verbose mode, dry-run validation, and output directory override.

**Key dependencies:**
- `clap` for CLI argument parsing

---

### Phase 7: Testing and Validation
**Estimated effort: 2-3 days** | **Status: ✅ COMPLETE**

- [x] Unit tests for each physics component (77 tests)
- [x] Integration tests with known configurations (2 tests)
- [x] Validate SOR roundtrip parsing with otdrs crate
- [x] Test multi-wavelength consistency
- [x] Test macrobend wavelength dependence (1625nm >> 1550nm >> 1310nm loss)

---

## Fiber Type Library

Compiled-in fiber parameters based on ITU-T specifications:

| Fiber Type | MFD@1310 (μm) | MFD@1550 (μm) | α@1310 (dB/km) | α@1550 (dB/km) | α@1625 (dB/km) | n_g | Backscatter (dB) |
|------------|---------------|---------------|----------------|----------------|----------------|-----|------------------|
| G.652.D    | 9.2 ± 0.4     | 10.4 ± 0.5    | ≤0.35          | ≤0.22          | ≤0.25          | 1.4682 | -79 (1ns ref)    |
| G.657.A1   | 8.6 ± 0.4     | 9.8 ± 0.5     | ≤0.35          | ≤0.22          | ≤0.25          | 1.4680 | -79              |
| G.657.A2   | 8.6 ± 0.4     | 9.8 ± 0.5     | ≤0.35          | ≤0.22          | ≤0.25          | 1.4680 | -79              |
| G.657.B3   | 8.0 ± 0.4     | 9.0 ± 0.5     | ≤0.35          | ≤0.22          | ≤0.25          | 1.4678 | -79              |

Macrobend loss coefficients (approximate, for modeling):

| Fiber Type | Min Bend Radius (mm) | Loss@1550 at min radius (dB/turn) | Loss@1625 at min radius (dB/turn) |
|------------|---------------------|-----------------------------------|-----------------------------------|
| G.652.D    | 30                  | ~0.5                              | ~1.5                              |
| G.657.A1   | 10                  | ≤0.25                             | ≤0.75                             |
| G.657.A2   | 7.5                 | ≤0.03                             | ≤0.1                              |
| G.657.B3   | 5                   | ≤0.03                             | ≤0.08                             |

---

## Dependencies

```toml
[package]
name = "otdrs-simulator"
version = "0.1.0"
edition = "2024"

[dependencies]
# Workspace dependency for SOR file generation
otdrs = { path = ".." }

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
serde_yaml = "0.9"
serde_cbor = "0.11"

# CLI
clap = { version = "4.5", features = ["derive"] }

# Math and DSP
rustfft = "6.2"

# Random number generation (reproducible)
rand = "0.8"
rand_chacha = "0.3"

# Utilities
uuid = { version = "1.0", features = ["v4", "serde"] }
thiserror = "2"

[dev-dependencies]
tempfile = "3"
approx = "0.5"  # For floating-point comparisons in tests
```

---

## Success Criteria

1. **Physical Accuracy**: Simulated traces should qualitatively match real OTDR traces from the `data/` directory
2. **Multi-wavelength Consistency**: Same fiber should produce consistent results across wavelengths with expected wavelength-dependent variations
3. **Macrobend Detection**: Macrobends should show characteristic wavelength-dependent loss (higher at 1625nm than 1310nm)
4. **Ground Truth Accuracy**: Ground truth positions and values should exactly match input configuration
5. **SOR Compatibility**: Generated SOR files should open correctly in professional OTDR analysis software
6. **Reproducibility**: Same seed should produce identical outputs

---

## Future Enhancements (Out of Scope for Initial Implementation)

- PON splitter simulation with branch contributions
- Time-varying effects (connector degradation)
- Environmental effects (temperature variation)
- Batch generation with parameter sweeps
- Python bindings for ML pipeline integration
- GPU-accelerated convolution for very large batches
- Real pulse shape capture and replay

---

## References

1. Feigel, B., et al. "OTDR Simulations of PONs: An LTI System Approach for Arbitrary Pulses." Journal of Lightwave Technology, vol. 32, no. 17, September 2014.
2. ITU-T G.652: Characteristics of a single-mode optical fibre and cable
3. ITU-T G.657: Characteristics of a bending-loss insensitive single-mode optical fibre and cable
4. Telcordia SR-4731: OTDR Measurement of Optical Fiber Parameters (for SOR file format)
5. Anderson, D.R., Johnson, L., & Bell, F.G. "Troubleshooting Optical-Fiber Networks." Academic Press, 2004.
