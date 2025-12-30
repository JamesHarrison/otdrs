//! Common unit definitions using uom
//!
//! This module defines the SI units used throughout the simulator to ensure
//! dimensional correctness.

use uom::si::{Quantity, ISQ, SI};
use uom::typenum::{N1, Z0};

pub use uom::si::f64::*;
pub use uom::si::length::meter;
pub use uom::si::length::kilometer;
pub use uom::si::time::second;
pub use uom::si::time::nanosecond;
pub use uom::si::frequency::hertz;
pub use uom::si::frequency::megahertz;
pub use uom::si::power::watt;
pub use uom::si::power::milliwatt;
pub use uom::si::ratio::ratio;
pub use uom::si::velocity::meter_per_second;
pub use uom::si::area::square_micrometer;
// Use reciprocal_meter from reciprocal_length module as our per_meter
pub use uom::si::reciprocal_length::reciprocal_meter as per_meter;

// Attenuation is often dB/km, which is technically dimensionless per length (Neper/m)
// but uom handles this via Quantity generic.
// We'll define a type alias for linear attenuation (1/length)
pub type AttenuationCoef = Quantity<
    ISQ<N1, Z0, Z0, Z0, Z0, Z0, Z0>,
    SI<f64>,
    f64
>;

/// Speed of light in vacuum
pub const SPEED_OF_LIGHT: Velocity = Velocity {
    dimension: uom::lib::marker::PhantomData,
    units: uom::lib::marker::PhantomData,
    value: 299_792_458.0,
};

/// Type wrapper for Optical Reflectance (ratio of reflected to incident power)
/// Convention: Usually expressed in negative dB (e.g. -50 dB).
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct Reflectance(Ratio);

impl Reflectance {
    /// Create from a dB value.
    /// Handles sign convention: if positive, it is negated (assumed to be intended as negative).
    pub fn from_db(db: f64) -> Self {
        let reflectance_db = if db > 0.0 { -db } else { db };
        let ratio_val = 10.0_f64.powf(reflectance_db / 10.0);
        Reflectance(Ratio::new::<ratio>(ratio_val))
    }

    /// Get valid Ratio (0.0 to 1.0)
    pub fn as_ratio(&self) -> Ratio {
        self.0
    }
}

/// Type wrapper for Return Loss (ratio of incident to reflected power, in dB)
/// Convention: Always positive dB (e.g. 50 dB).
/// A Return Loss of 50 dB means Reflected is 50 dB below Incident.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct ReturnLoss(Ratio);

impl ReturnLoss {
    /// Create from a dB value.
    /// Handles sign convention: if negative, it is negated (assumed to be intended as positive).
    pub fn from_db(db: f64) -> Self {
        let loss_db = if db < 0.0 { -db } else { db };
        // RL = 10 log(Pi/Pr) => Pr/Pi = 10^(-RL/10)
        let ratio_val = 10.0_f64.powf(-loss_db / 10.0);
        ReturnLoss(Ratio::new::<ratio>(ratio_val))
    }

    /// Get valid Ratio (0.0 to 1.0) representing the Reflection Fraction
    pub fn as_ratio(&self) -> Ratio {
        self.0
    }
}
