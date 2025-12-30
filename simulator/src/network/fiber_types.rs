//! Fiber type library with compiled-in ITU-T specifications
//!
//! This module provides standard fiber types with their wavelength-dependent
//! characteristics based on ITU-T recommendations.

use serde::{Deserialize, Serialize};

/// Standard fiber types with ITU-T specifications
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    Custom(Box<CustomFiberParams>),
}

/// Parameters for custom fiber types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Backscatter coefficient in dB (1ns reference)
    pub backscatter_coeff_db: f64,
}

/// Wavelength-dependent fiber characteristics
#[derive(Debug, Clone, Copy)]
pub struct FiberCharacteristics {
    /// Mode field diameter in micrometers
    pub mfd_um: f64,
    /// Attenuation coefficient in dB/km
    pub attenuation_db_km: f64,
    /// Group index of refraction
    pub group_index: f64,
    /// Backscatter coefficient in dB (1ns reference)
    pub backscatter_coeff_db: f64,
}

impl FiberType {
    /// Get characteristics for this fiber type at a specific wavelength
    pub fn characteristics(&self, wavelength_nm: u16) -> FiberCharacteristics {
        match self {
            FiberType::G652D => Self::g652d_characteristics(wavelength_nm),
            FiberType::G657A1 => Self::g657a1_characteristics(wavelength_nm),
            FiberType::G657A2 => Self::g657a2_characteristics(wavelength_nm),
            FiberType::G657B3 => Self::g657b3_characteristics(wavelength_nm),
            FiberType::Custom(params) => Self::custom_characteristics(params, wavelength_nm),
        }
    }

    /// G.652.D characteristics
    /// Attenuation values based on typical modern cable specifications
    fn g652d_characteristics(wavelength_nm: u16) -> FiberCharacteristics {
        // Modern G.652.D cable typically achieves:
        // - 1310nm: 0.32-0.35 dB/km (ITU max: 0.35)
        // - 1550nm: 0.18-0.22 dB/km (ITU max: 0.22)
        // - 1625nm: 0.20-0.24 dB/km (ITU max: 0.24)
        let (mfd_um, attenuation_db_km) = match wavelength_nm {
            1260..=1360 => (9.2, 0.33),  // 1310nm window - typical modern
            1500..=1600 => (10.4, 0.19), // 1550nm window - typical modern
            1601..=1650 => (10.5, 0.21), // 1625nm window - typical modern
            _ => {
                // Linear interpolation for other wavelengths
                Self::interpolate_characteristics(wavelength_nm, 9.2, 10.4, 0.33, 0.19)
            }
        };

        FiberCharacteristics {
            mfd_um,
            attenuation_db_km,
            group_index: 1.4682,
            backscatter_coeff_db: Self::backscatter_coeff_for_wavelength(wavelength_nm),
        }
    }

    /// G.657.A1 characteristics
    /// Slightly lower attenuation than G.652.D due to improved manufacturing
    fn g657a1_characteristics(wavelength_nm: u16) -> FiberCharacteristics {
        let (mfd_um, attenuation_db_km) = match wavelength_nm {
            1260..=1360 => (8.6, 0.32),
            1500..=1600 => (9.8, 0.18),
            1601..=1650 => (9.9, 0.20),
            _ => Self::interpolate_characteristics(wavelength_nm, 8.6, 9.8, 0.32, 0.18),
        };

        FiberCharacteristics {
            mfd_um,
            attenuation_db_km,
            group_index: 1.4680,
            backscatter_coeff_db: Self::backscatter_coeff_for_wavelength(wavelength_nm),
        }
    }

    /// G.657.A2 characteristics (same attenuation as A1, different bend performance)
    fn g657a2_characteristics(wavelength_nm: u16) -> FiberCharacteristics {
        Self::g657a1_characteristics(wavelength_nm)
    }

    /// G.657.B3 characteristics
    fn g657b3_characteristics(wavelength_nm: u16) -> FiberCharacteristics {
        let (mfd_um, attenuation_db_km) = match wavelength_nm {
            1260..=1360 => (8.0, 0.32),
            1500..=1600 => (9.0, 0.18),
            1601..=1650 => (9.1, 0.20),
            _ => Self::interpolate_characteristics(wavelength_nm, 8.0, 9.0, 0.32, 0.18),
        };

        FiberCharacteristics {
            mfd_um,
            attenuation_db_km,
            group_index: 1.4678,
            backscatter_coeff_db: Self::backscatter_coeff_for_wavelength(wavelength_nm),
        }
    }

    /// Custom fiber characteristics with interpolation
    fn custom_characteristics(params: &CustomFiberParams, wavelength_nm: u16) -> FiberCharacteristics {
        let (mfd_um, attenuation_db_km) = match wavelength_nm {
            1260..=1360 => (params.mfd_1310_um, params.attenuation_1310_db_km),
            1500..=1600 => (params.mfd_1550_um, params.attenuation_1550_db_km),
            1601..=1650 => (
                params.mfd_1550_um + (params.mfd_1550_um - params.mfd_1310_um) * 0.3,
                params.attenuation_1625_db_km,
            ),
            _ => Self::interpolate_characteristics(
                wavelength_nm,
                params.mfd_1310_um,
                params.mfd_1550_um,
                params.attenuation_1310_db_km,
                params.attenuation_1550_db_km,
            ),
        };

        FiberCharacteristics {
            mfd_um,
            attenuation_db_km,
            group_index: params.group_index,
            backscatter_coeff_db: params.backscatter_coeff_db,
        }
    }

    /// Linear interpolation between 1310nm and 1550nm characteristics
    fn interpolate_characteristics(
        wavelength_nm: u16,
        mfd_1310: f64,
        mfd_1550: f64,
        atten_1310: f64,
        atten_1550: f64,
    ) -> (f64, f64) {
        // Clamp wavelength to valid range
        let wl = wavelength_nm.clamp(1260, 1650) as f64;
        let t = (wl - 1310.0) / (1550.0 - 1310.0);

        let mfd = mfd_1310 + t * (mfd_1550 - mfd_1310);
        let attenuation = atten_1310 + t * (atten_1550 - atten_1310);

        (mfd, attenuation)
    }

    /// Get wavelength-dependent backscatter coefficient
    ///
    /// The backscatter coefficient varies with wavelength due to the λ⁻⁴
    /// dependence of Rayleigh scattering. Typical values (referenced to 1ns pulse):
    /// - 1310nm: -79 dB
    /// - 1550nm: -82 dB
    /// - 1650nm: -83 dB
    fn backscatter_coeff_for_wavelength(wavelength_nm: u16) -> f64 {
        match wavelength_nm {
            1260..=1360 => -79.0, // 1310nm window
            1500..=1600 => -82.0, // 1550nm window
            1601..=1700 => -83.0, // 1650nm window
            _ => {
                // Linear interpolation for other wavelengths
                let wl = wavelength_nm.clamp(1260, 1700) as f64;
                if wl < 1430.0 {
                    // Between 1310 and 1550 windows
                    let t = (wl - 1310.0) / (1550.0 - 1310.0);
                    -79.0 + t * (-82.0 - (-79.0))
                } else {
                    // Between 1550 and 1650 windows
                    let t = (wl - 1550.0) / (1650.0 - 1550.0);
                    -82.0 + t * (-83.0 - (-82.0))
                }
            }
        }
    }

    /// Get macrobend loss coefficient for this fiber type
    /// Returns loss in dB per turn at specified wavelength and bend radius
    pub fn macrobend_loss_per_turn(&self, wavelength_nm: u16, bend_radius_mm: f64) -> f64 {
        // Simplified exponential model: Loss ∝ exp(-B × R / λ)
        // Where R is bend radius and λ is wavelength
        // Coefficients are fiber-type dependent

        let (a_coeff, b_coeff) = match self {
            FiberType::G652D => (50.0, 20.0),    // Standard fiber, high bend loss
            FiberType::G657A1 => (10.0, 15.0),   // Bend-insensitive
            FiberType::G657A2 => (2.0, 12.0),    // Very bend-insensitive
            FiberType::G657B3 => (0.5, 10.0),    // Ultra bend-insensitive
            FiberType::Custom(_) => (10.0, 15.0), // Default to A1 characteristics
        };

        // Wavelength in micrometers for calculation
        let lambda_um = wavelength_nm as f64 / 1000.0;

        // Loss increases exponentially as bend radius decreases
        // and increases with wavelength
        a_coeff * (lambda_um / 1.55).powi(3) * (-b_coeff * bend_radius_mm / lambda_um / 1000.0).exp()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_g652d_characteristics_1310() {
        let chars = FiberType::G652D.characteristics(1310);
        assert!((chars.mfd_um - 9.2).abs() < 0.1);
        assert!((chars.attenuation_db_km - 0.33).abs() < 0.01);
        assert!((chars.group_index - 1.4682).abs() < 0.001);
    }

    #[test]
    fn test_g652d_characteristics_1550() {
        let chars = FiberType::G652D.characteristics(1550);
        assert!((chars.mfd_um - 10.4).abs() < 0.1);
        assert!((chars.attenuation_db_km - 0.19).abs() < 0.01);
    }

    #[test]
    fn test_macrobend_loss_increases_with_wavelength() {
        let loss_1310 = FiberType::G652D.macrobend_loss_per_turn(1310, 15.0);
        let loss_1550 = FiberType::G652D.macrobend_loss_per_turn(1550, 15.0);
        let loss_1625 = FiberType::G652D.macrobend_loss_per_turn(1625, 15.0);

        // Loss should increase with wavelength
        assert!(loss_1550 > loss_1310);
        assert!(loss_1625 > loss_1550);
    }

    #[test]
    fn test_macrobend_loss_decreases_with_radius() {
        let loss_5mm = FiberType::G652D.macrobend_loss_per_turn(1550, 5.0);
        let loss_15mm = FiberType::G652D.macrobend_loss_per_turn(1550, 15.0);
        let loss_30mm = FiberType::G652D.macrobend_loss_per_turn(1550, 30.0);

        // Loss should decrease as bend radius increases
        assert!(loss_5mm > loss_15mm);
        assert!(loss_15mm > loss_30mm);
    }

    #[test]
    fn test_g657_lower_bend_loss_than_g652() {
        let g652_loss = FiberType::G652D.macrobend_loss_per_turn(1550, 10.0);
        let g657a1_loss = FiberType::G657A1.macrobend_loss_per_turn(1550, 10.0);
        let g657a2_loss = FiberType::G657A2.macrobend_loss_per_turn(1550, 10.0);
        let g657b3_loss = FiberType::G657B3.macrobend_loss_per_turn(1550, 10.0);

        // G.657 fibers should have lower bend loss than G.652
        assert!(g657a1_loss < g652_loss);
        assert!(g657a2_loss < g657a1_loss);
        assert!(g657b3_loss < g657a2_loss);
    }
}
