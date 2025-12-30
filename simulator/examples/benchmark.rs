use std::time::Instant;
use otdrs_simulator::postprocess::noise::add_noise_with_floor;
use otdrs_simulator::physics::convolution::convolve;
use otdrs_simulator::units::{Power, watt};

fn main() {
    println!("Running benchmarks...");

    benchmark_noise();
    benchmark_convolution();
}

fn benchmark_noise() {
    println!("\n--- Noise Generation Benchmark ---");
    let size = 1_000_000;
    // Trace level -60dBm = 1nW = 1e-9 W. Let's use 1uW (-30dBm) for signal.
    let signal_level = Power::new::<watt>(1e-6);
    let trace = vec![signal_level; size];
    
    // Noise floor -90dBm = 1pW = 1e-12 W.
    let noise_floor = Power::new::<watt>(1e-12);
    // Noise std deviation
    let noise_std = Power::new::<watt>(1e-13);
    
    let iterations = 100;

    let start = Instant::now();
    for _ in 0..iterations {
        // Benchmark now uses strongly typed Power values
        let _ = add_noise_with_floor(&trace, noise_floor, noise_std, None);
    }
    let duration = start.elapsed();

    println!("add_noise_with_floor for {} points, {} iterations: {:?}", size, iterations, duration);
    println!("Average per iteration: {:?}", duration / iterations as u32);
    println!("Points per second: {:.2e}", (size * iterations) as f64 / duration.as_secs_f64());
}

fn benchmark_convolution() {
    println!("\n--- Convolution Benchmark ---");
    let signal_size = 100_000;
    let kernel_size = 1_000;
    let signal = vec![1.0; signal_size];
    let kernel = vec![0.5; kernel_size];
    let iterations = 10;

    let start = Instant::now();
    for _ in 0..iterations {
        // convolve still accepts primitive f64 slices
        let _ = convolve(&signal, &kernel);
    }
    let duration = start.elapsed();

    println!("convolve for signal {}, kernel {}, {} iterations: {:?}", signal_size, kernel_size, iterations, duration);
    println!("Average per iteration: {:?}", duration / iterations as u32);
}
