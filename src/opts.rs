use clap::Parser;
use libbladerf_sys::{Hertz, Sps};

/// TODO cli docs
#[derive(Parser, Debug, Clone)]
#[clap(version)]
#[command(name = "bladerf-power")]
pub struct Opts {
    /// BladeRF device ID.
    ///
    /// Format: <backend>:[device=<bus>:<addr>] [instance=<n>] [serial=<serial>]
    ///
    /// Example: "*:serial=f12ce1037830a1b27f3ceeba1f521413"
    #[clap(short = 'd', long, env = "BLADERF_DEVICE_ID")]
    pub device_id: String,

    /// Frequency (Hertz)
    ///
    /// Accepts:
    ///
    /// * Hertz: <num>H | <num>h
    ///
    /// * KiloHertz: <num>K | <num>k
    ///
    /// * MegaHertz: <num>M | <num>m
    #[clap(short = 'f', long, env = "BLADERF_FREQUENCY")]
    pub frequency: Hertz,

    /// Sample rate (samples per second)
    #[clap(short = 's', long, env = "BLADERF_SAMPLE_RATE")]
    pub sample_rate: Sps,

    /// Bandwidth (Hertz)
    ///
    /// Accepts:
    ///
    /// * Hertz: <num>H | <num>h
    ///
    /// * KiloHertz: <num>K | <num>k
    ///
    /// * MegaHertz: <num>M | <num>m
    #[clap(short = 'b', long, env = "BLADERF_BANDWIDTH")]
    pub bandwidth: Hertz,

    /// Print info and exit
    #[clap(long)]
    pub dry_run: bool,

    /// Number of bins in the FFT
    #[clap(long, default_value = "8192")]
    pub fft_bins: usize,
}
