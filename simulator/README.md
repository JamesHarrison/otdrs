# OTDR Simulator

This is a new crate for generating simulated OTDR data. It is intended to produce large volumes of OTDR data to be used for testing and validation of OTDR processing software, as well as to generate training and validation datasets for machine learning models processing OTDR information.

## Goals of simulator output

We need to produce data which looks as accurate as possible compared to the "real thing". This means we should model the physical processes of OTDR testing as accurately as possible, and we should also model the noise and other imperfections of the OTDR as accurately as possible.

The output of the simulator should be a `SORFile`, provided by the `otdrs` crate, along with a JSON sidecar for the SOR file containing metadata about the simulation configuration and output usable as input for a system evaluating the performance of an analysis algorithm. 

The simulator should be able to generate a wide range of different OTDR files, including those with different acquisition parameters, different fiber parameters, and different fiber types and events.

The input of the simulator should allow a caller to specify simulated acquisition parameters that one would normally find on an OTDR - such as range, pulse width, etc - as well as other parameters such as thermal noise, shot noise, dynamic range, dead zone lengths, etc that can be used to simulate different quality OTDR acquisitions. The output SORFile should reflect the input acquisition parameters as far as possible, and should contain the simulation parameters as a CBOR serialised blob stored in a proprietary block in the SORFile.

## Physical realisation of optical time domain reflectometry testing

The simulator should be able to simulate the physical processes of optical time domain reflectometry testing as accurately as possible. This includes the following:

- Light passing through fibers, including backscattering (Rayleigh scattering, etc)
- Light loss at splices where mode field diameters match
- Light "gain" or loss at splices where mode field diameters do not match
- Fresnel/dirt reflectance of mated connectors and loss of light through a mated connector join
- Fresnel reflectance of unmated connectors (APC/UPC)
- Macrobending effects (different losses at different wavelengths)
- Light being split and combined at splitters/couplers
- Pulse width effects (e.g. shorter pulse widths leading to higher resolution and better discrimination of close events, but lower signal-to-noise ratio and lower effective range)
- Range setting effects (e.g. higher range setting leads to higher injection power, larger dead zones after reflective events)
- Detector saturation (e.g. high reflectance/higher injection power leading to detector saturation and clipping of the signal)
- Detector bandwidth limitations
- Repeated shots and averaging time/count variation
- Dynamic range effects (e.g. lower dynamic range leading to lower signal-to-noise ratio and lower effective range)
- Thermal noise effects (e.g. higher temperature leading to higher thermal noise)
- Shot noise effects (e.g. higher shot noise in the avalanche photodiode and receiver front-end, leading to lower signal-to-noise ratio)
- Pulse shaping effects
    - "better" hardware will have more precise on/off/rectangular shapes in the pulse, but as most OTDR units use a mode-locked pulsed laser, the pulse will in practice have on/off rise/fall periods on all units
    - the speed of rise/fall periods should be configurable

The simulator should also be able to simulate complex scenarios, such as:

- Long fibres with many splices of varying attenuation characteristics
- Varying fibre types along the fibre (with "visible" or "invisible" (e.g. very low loss) splices, e.g. joining G.657.A1 to G.652.D might have MFDs of 8.6um to 9.2um, with different indexes of refraction)
- Combinations of events very close to each other e.g. splices within 50cm of connectors
- Generation of traces at multiple wavelengths (e.g. the same simulated fibre measured at 1310nm, 1550nm, and 1625nm)
- Generation of traces at multiple acquisition ranges (e.g. the same simulated fibre measured at 10km, 20km, and 50km)


The simulator should permit configuration of all fibre and events, such as position, insertion loss, reflectance/return loss, distance, etc in easy-to-parse formats e.g. dB, metres, etc, and perform any necessary calculations to convert these to internal values suitable for storage in a SORFile and complying with the Bellcore/Telcordia specification.

