use crate::{interruptor::Interruptor, opts::Opts};
use clap::Parser;
use libbladerf_sys::*;
use rustfft::{num_complex::Complex, FftPlanner};
use tracing::{debug, error, info, trace, warn};

mod dsp;
mod interruptor;
mod opts;

fn main() {
    match do_main() {
        Ok(()) => (),
        Err(e) => {
            error!("{e}");
            let mut cause = e.source();
            while let Some(err) = cause {
                error!("Caused by: {err}");
                cause = err.source();
            }
            std::process::exit(exitcode::SOFTWARE);
        }
    }
}

fn do_main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::builder()
                .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
                .from_env_lossy(),
        )
        .compact()
        .init();

    let intr = Interruptor::new();
    let intr_clone = intr.clone();
    ctrlc::set_handler(move || {
        if intr_clone.is_set() {
            let exit_code = if cfg!(target_family = "unix") {
                // 128 (fatal error signal "n") + 2 (control-c is fatal error signal 2)
                130
            } else {
                // Windows code 3221225786
                // -1073741510 == C000013A
                -1073741510
            };
            std::process::exit(exit_code);
        }

        info!("Requesting shutdown");
        intr_clone.set();
    })?;

    let opts = Opts::parse();

    let channel = Channel::Rx0;
    let channel_layout = ChannelLayout::RxX1;
    let format = Format::Sc16Q11Meta;
    let num_buffers = 32;
    let samples_per_buffer = 32 * 1024;
    let iq_pairs_per_buffer = samples_per_buffer / 2;
    let num_transfers = 16;
    let config_timeout_ms = 1000_u32.ms();
    let rx_timeout_ms = 50_u32.ms();

    info!(%channel);
    info!(frequency = %opts.frequency);
    info!(sample_rate = %opts.sample_rate);
    info!(bandwidth = %opts.bandwidth);
    info!(layout = %channel_layout);
    info!(%format);
    info!(fft_bins = opts.fft_bins);

    debug!(num_buffers);
    debug!(samples_per_buffer);
    debug!(iq_pairs_per_buffer);
    debug!(num_transfers);
    debug!(?config_timeout_ms);
    debug!(?rx_timeout_ms);

    check_limits(opts.frequency, opts.bandwidth, opts.sample_rate)?;

    if opts.dry_run {
        return Ok(());
    }

    let mut sample_buffer: Vec<i16> = vec![0_i16; samples_per_buffer];
    let mut complex_storage: Vec<Complex<f64>> = Vec::with_capacity(iq_pairs_per_buffer / 2);
    let mut fft_scratch = vec![Complex::new(0.0, 0.0); iq_pairs_per_buffer / 2];
    let mut fft_planner = FftPlanner::<f64>::new();

    let fft = fft_planner.plan_fft_forward(opts.fft_bins);

    info!(id = opts.device_id, "Opening device");
    let mut dev = Device::open(&opts.device_id)?;

    let info = dev.device_info()?;
    info!(device_info = %info);

    let speed = dev.device_speed()?;
    info!(%speed);

    dev.enable_module(channel, false)?;

    dev.set_frequency(channel, opts.frequency)?;

    let actual_sample_rate = dev.set_sample_rate(channel, opts.sample_rate)?;
    if opts.sample_rate != actual_sample_rate {
        warn!(%actual_sample_rate, "Actual sample rate differs from requested");
    }

    let actual_bandwidth = dev.set_bandwidth(channel, opts.bandwidth)?;
    if opts.bandwidth != actual_bandwidth {
        warn!(%actual_bandwidth, "Actual bandwidth differs from requested");
    }

    info!("Enabling sync rx config");
    dev.sync_config(
        channel_layout,
        format,
        num_buffers,
        samples_per_buffer,
        num_transfers,
        config_timeout_ms,
    )?;

    dev.enable_module(channel, true)?;

    info!(%channel, "Channel active");

    while !intr.is_set() {
        let mut metadata = Metadata::new_rx_now();
        match dev.sync_rx(&mut sample_buffer, Some(&mut metadata), rx_timeout_ms) {
            Ok(_) => {}
            Err(Error::Timeout) => continue,
            Err(e) => return Err(e.into()),
        }
        trace!(%metadata, "SyncRx");

        let num_samples = if metadata.actual_count() == 0 {
            warn!("SyncRx empty packet");
            continue;
        } else if metadata.status().underrun() {
            warn!("SyncRx underrun");
            continue;
        } else {
            metadata.actual_count() as usize
        };
        debug_assert_eq!(num_samples % 2, 0);

        complex_storage.clear();
        dsp::push_normalize_sc16_q11(&sample_buffer[..num_samples], &mut complex_storage);
        if complex_storage.len() < opts.fft_bins {
            warn!(
                storage_len = complex_storage.len(),
                fft_bins = opts.fft_bins,
                "Not enough samples to process"
            );
            continue;
        }
        // TODO need to configure bins and/or downsample and/or reduce samples_per_buffer to match
        // bins
        //trace!(complex_storage_len = complex_storage.len());
        debug_assert_eq!(complex_storage.len(), opts.fft_bins);

        fft.process_with_scratch(&mut complex_storage, &mut fft_scratch);
    }

    let _ignore = dev.enable_module(channel, false);
    dev.close();

    Ok(())
}

fn check_limits(
    frequency: Frequency,
    bandwidth: Bandwidth,
    sample_rate: SampleRate,
) -> Result<(), String> {
    if (frequency < device_limits::FREQUENCY_MIN) || (frequency > device_limits::FREQUENCY_MAX) {
        Err(format!("Frequency {frequency} invalid"))
    } else if (bandwidth < device_limits::BANDWIDTH_MIN)
        || (bandwidth > device_limits::BANDWIDTH_MAX)
    {
        Err(format!("Bandwidth {bandwidth} invalid"))
    } else if (sample_rate < device_limits::SAMPLE_RATE_MIN)
        || (sample_rate > device_limits::SAMPLE_RATE_MAX)
    {
        Err(format!("Sample rate {sample_rate} invalid"))
    } else {
        Ok(())
    }
}
