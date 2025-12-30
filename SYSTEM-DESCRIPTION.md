# OTDR Simulator: Trace Construction & Noise Implementation

This document describes the current implementation of trace generation and noise modeling in the OTDR simulator.

## 1. Trace Construction Overview

The simulator uses a Linear Time-Invariant (LTI) system approach. The fiber network is modeled as a system with an impulse response $h_f(z)$, and the output trace $y(z)$ is the convolution of this impulse response with the source pulse $x(z)$.

$$y(z) = h_f(z) * x(z)$$

The construction process follows these steps:

### A. Effective Network Building
Before simulation, the network is augmented to include internal OTDR components:
1.  **Front Panel Fiber**: A short internal section (typically 20-100m).
2.  **Front Panel Connector**: The interface between the internal fiber and external network.
3.  **Launch Cable**: If configured, prepended to the user's fiber.
4.  **Network Offset**: All user-defined events are shifted by the total length of the internal fiber and launch cable.

### B. Impulse Response Calculation ($h_f(z)$)
The total impulse response is composed of three components:
1.  **Backscatter Impulse ($h_b(z)$)**:
    - Calculates Rayleigh backscatter based on the fiber's backscatter coefficient (e.g., -79 dB for G.652).
    - Includes the round-trip power factor: $10^{-2\alpha z / 10}$.
    - Formula: $h_b(z) = \Delta z \cdot K' \cdot 10^{-2\alpha z / 10}$
2.  **Reflection Impulse ($h_r(z)$)**:
    - Models discrete reflections from connectors and fiber ends as Dirac-like spikes.
    - Scaled by the round-trip loss to the event position.
3.  **Loss Function ($f_l(z)$)**:
    - Accounts for one-way (and effectively round-trip via the exponent) losses from splices, splitters, and fiber attenuation.

**Total Impulse Response**: $h_f(z) = f_l(z) \cdot [h_b(z) + h_r(z)]$

### C. Pulse Generation and Convolution
1.  A source pulse $x(z)$ is generated (default: Hyperbolic Secant shape).
2.  The pulse width is converted to spatial length: $W = (v_g \cdot \tau) / 2$.
3.  The trace is computed via convolution: `convolve_same_power(&h_f, &pulse)`.

---

## 2. Post-Processing Chain

Once the raw optical power trace is generated, it passes through a realistic detector chain:

1.  **Detector Saturation**: Applies a non-linear compression to high power levels (e.g., near-end reflections).
2.  **Receiver Bandwidth**: Applies a low-pass filter (TIA tail simulation) to the trace, limiting spatial resolution.
3.  **Noise Addition**: Adds random Gaussian fluctuations.
4.  **Clipping**: Removes negative optical power values that may result from noise addition.

---

## 3. Noise Implementation

The noise model simulates electronic/thermal noise in the OTDR receiver.

### Parameters
-   **Noise Floor ($P_{floor}$)**: The base noise level defined in dBm.
-   **Averaging Time ($T$)**: The duration of acquisition.
-   **Number of Averages ($N$)**: Calculated from time using a standard rate (e.g., 10,000 Hz).

### Calculation
1.  **Standard Deviation**: The noise standard deviation ($\sigma$) is derived from the noise floor power and the number of averages.
    
    $$\sigma = \frac{P_{floor}}{\sqrt{N}}$$
    
    The noise standard deviation scales naturally with $\sqrt{N}$ as expected for Gaussian noise averaging, allowing long acquisition times (e.g., 30s or 180s) to effectively reduce the noise fluctuations.

2.  **Random Generation**: Gaussian noise with zero mean and standard deviation $\sigma$ is added to each point in the trace.
    
    $$P_{noisy}[z] = P_{signal}[z] + \text{Gaussian}(0, \sigma)$$

3.  **Baseline Shift**: Because the trace is clipped at zero *after* adding zero-mean noise, the average noise floor level appears slightly elevated (biased) when the signal is absent.

## 5. Backscatter Power & Scaling

The initial power level of the backscatter trace is determined by the "Backscatter Level" ($L_{bs}$), which represents the ratio of backscattered power to incident pulse power.

### A. Physical Factors
The backscattered power $P_{bs}$ at the beginning of the trace depends on:
1.  **Injection Power**: $P_{peak} \propto P_{bs}$.
2.  **Pulse Width ($\tau$ )**: More energy in the fiber yields more scattering. $P_{bs} \propto \tau$.
3.  **Backscatter Coefficient ($B$)**: An intrinsic fiber property (Rayleigh scattering coefficient $\alpha_s$ and capture fraction $S$). Typical values for SMF are **-79 to -82 dB** (referenced to a 1ns pulse).

**Formula (10 log10 Power):**
$$P_{bs}(\text{dBm}) = P_{peak}(\text{dBm}) + 10 \log_{10}(\tau / 1\text{ns}) + B$$

> [!NOTE]
> For a **10ns** pulse and **-79dB** coefficient, the backscatter starts at **-69dB** relative to the peak.
> For a **1µs** pulse, the level would be **-49dB** relative to the peak.

### B. One-Way vs. Two-Way Scaling (The -40dB vs -70dB Issue)
A common point of confusion in OTDR systems is the choice between 10 log10 and 5 log10 scaling:
-   **10 log10 (Two-Way Power)**: Used for absolute power measurements (dBm). A 10ns trace starts at **-70dB**.
-   **5 log10 (One-Way Loss)**: Used for trace visualization to make the slope match the fiber's one-way attenuation (e.g., 0.2 dB/km). In this scale, -70dB (power) is displayed as **-35dB** (loss).

Most OTDR viewers expect the data points in the SOR file to be stored using the **5 log10** convention to fit the full dynamic range into the 16-bit data format and to display one-way fiber losses directly.

> [!WARNING]
> The current implementation incorrectly uses 10 log10 for SOR data points, causing values to exceed the 65.5 dB limit of the SOR format for short pulses, leading to "clamping" at the bottom of the trace.
