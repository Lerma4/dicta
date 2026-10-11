use std::{
    sync::{
        mpsc::{self, Receiver, SyncSender},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    Device, SampleFormat, Stream, StreamConfig,
};
use rubato::{
    Resampler, SincFixedIn, SincInterpolationParameters, SincInterpolationType, WindowFunction,
};
use serde::Serialize;

const OUTPUT_RATE: usize = 16_000;

#[derive(Clone, Debug, Serialize)]
pub struct InputDevice {
    pub name: String,
    pub is_default: bool,
}

#[derive(Clone, Debug)]
pub enum AudioEvent {
    Level {
        level: f32,
        speech: bool,
        elapsed_ms: u64,
    },
    Error(String),
}

pub struct CapturedAudio {
    pub samples_16khz: Vec<f32>,
    pub recorded_ms: u64,
    pub speech_detected: bool,
}

struct CaptureState {
    samples: Vec<f32>,
    sample_rate: usize,
    channels: usize,
    max_samples: usize,
    threshold: f32,
    speech_detected: bool,
    last_event: Instant,
    events: SyncSender<AudioEvent>,
}

pub struct CaptureHandle {
    stream: Option<Stream>,
    state: Arc<Mutex<CaptureState>>,
    events: Receiver<AudioEvent>,
}

impl CaptureHandle {
    pub fn try_event(&self) -> Option<AudioEvent> {
        self.events.try_recv().ok()
    }

    pub fn stop(mut self) -> Result<CapturedAudio, String> {
        let stream = self.stream.take().ok_or("capture already stopped")?;
        let pause_result = stream.pause();
        drop(stream);
        let mut state = self.state.lock().map_err(|_| "audio buffer poisoned")?;
        let source = std::mem::take(&mut state.samples);
        let rate = state.sample_rate;
        let recorded_ms = (source.len() as u64 * 1000) / rate as u64;
        let speech_detected = state.speech_detected;
        drop(state);
        if source.is_empty() {
            pause_result.map_err(|e| e.to_string())?;
        }
        Ok(CapturedAudio {
            samples_16khz: resample_mono(&source, rate)?,
            recorded_ms,
            speech_detected,
        })
    }

    pub fn cancel(mut self) {
        self.stream.take();
    }
}

pub fn list_input_devices() -> Result<Vec<InputDevice>, String> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|d| d.description().ok())
        .map(|d| d.name().to_owned());
    host.input_devices()
        .map_err(|e| e.to_string())?
        .map(|d| {
            let name = d
                .description()
                .map_err(|e| e.to_string())?
                .name()
                .to_owned();
            Ok(InputDevice {
                is_default: default_name.as_deref() == Some(&name),
                name,
            })
        })
        .collect()
}

pub fn start_capture(
    device_name: Option<&str>,
    max_duration: Duration,
    vad_threshold: f32,
) -> Result<CaptureHandle, String> {
    if max_duration.is_zero() {
        return Err("maximum recording duration must be positive".into());
    }
    if max_duration > Duration::from_secs(300) {
        return Err("maximum recording duration cannot exceed five minutes".into());
    }
    if !vad_threshold.is_finite() || !(0.0..=1.0).contains(&vad_threshold) {
        return Err("VAD threshold must be between 0 and 1".into());
    }
    let host = cpal::default_host();
    let default_device = host.default_input_device();
    let device = match device_name {
        Some(name) => host
            .input_devices()
            .map_err(|e| e.to_string())?
            .find(|d| d.description().ok().is_some_and(|v| v.name() == name))
            .ok_or("input device not found")?,
        None => default_device.ok_or("no default input device")?,
    };
    let supported = device.default_input_config().map_err(|e| e.to_string())?;
    let config: StreamConfig = supported.into();
    let rate = config.sample_rate as usize;
    let channels = config.channels as usize;
    let max_samples = (max_duration.as_secs_f64() * rate as f64).ceil() as usize;
    let (tx, rx) = mpsc::sync_channel(32);
    let state = Arc::new(Mutex::new(CaptureState {
        samples: Vec::with_capacity(max_samples),
        sample_rate: rate,
        channels,
        max_samples,
        threshold: vad_threshold,
        speech_detected: false,
        last_event: Instant::now() - Duration::from_secs(1),
        events: tx,
    }));
    let error_state = Arc::clone(&state);
    let error_callback = move |err: cpal::Error| {
        if let Ok(s) = error_state.lock() {
            let _ = s.events.try_send(AudioEvent::Error(err.to_string()));
        }
    };
    let stream = match supported.sample_format() {
        SampleFormat::F32 => {
            build_stream::<f32>(&device, &config, Arc::clone(&state), error_callback)?
        }
        SampleFormat::I16 => {
            build_stream::<i16>(&device, &config, Arc::clone(&state), error_callback)?
        }
        SampleFormat::U16 => {
            build_stream::<u16>(&device, &config, Arc::clone(&state), error_callback)?
        }
        format => return Err(format!("unsupported microphone sample format: {format:?}")),
    };
    stream.play().map_err(|e| e.to_string())?;
    Ok(CaptureHandle {
        stream: Some(stream),
        state,
        events: rx,
    })
}

trait ToMono {
    fn into_f32(self) -> f32;
}
impl ToMono for f32 {
    fn into_f32(self) -> f32 {
        self.clamp(-1.0, 1.0)
    }
}
impl ToMono for i16 {
    fn into_f32(self) -> f32 {
        self as f32 / i16::MAX as f32
    }
}
impl ToMono for u16 {
    fn into_f32(self) -> f32 {
        (self as f32 - 32768.0) / 32768.0
    }
}

fn build_stream<T: cpal::SizedSample + ToMono>(
    device: &Device,
    config: &StreamConfig,
    state: Arc<Mutex<CaptureState>>,
    error: impl FnMut(cpal::Error) + Send + 'static,
) -> Result<Stream, String> {
    device
        .build_input_stream(
            *config,
            move |input: &[T], _| {
                let Ok(mut s) = state.lock() else { return };
                let room = s.max_samples.saturating_sub(s.samples.len());
                if room == 0 {
                    return;
                }
                let channels = s.channels;
                let frames = input.chunks_exact(channels).take(room);
                let start = s.samples.len();
                for frame in frames {
                    s.samples
                        .push(frame.iter().map(|x| x.into_f32()).sum::<f32>() / channels as f32);
                }
                let added = &s.samples[start..];
                if added.is_empty() {
                    return;
                }
                let (rms, speech) = vad(added, s.threshold);
                s.speech_detected |= speech;
                if s.last_event.elapsed() >= Duration::from_millis(50) {
                    let elapsed_ms = (s.samples.len() as u64 * 1000) / s.sample_rate as u64;
                    let _ = s.events.try_send(AudioEvent::Level {
                        level: rms,
                        speech,
                        elapsed_ms,
                    });
                    s.last_event = Instant::now();
                }
            },
            error,
            None,
        )
        .map_err(|e| e.to_string())
}

fn vad(samples: &[f32], threshold: f32) -> (f32, bool) {
    let rms = if samples.is_empty() {
        0.0
    } else {
        (samples.iter().map(|x| x * x).sum::<f32>() / samples.len() as f32).sqrt()
    };
    (rms, rms >= threshold)
}

fn resample_mono(input: &[f32], input_rate: usize) -> Result<Vec<f32>, String> {
    if input.is_empty() {
        return Ok(Vec::new());
    }
    if input_rate == 0 {
        return Err("input sample rate must be positive".into());
    }
    if input_rate == OUTPUT_RATE {
        return Ok(input.to_vec());
    }
    let chunk = 1024;
    let mut resampler = SincFixedIn::<f32>::new(
        OUTPUT_RATE as f64 / input_rate as f64,
        2.0,
        SincInterpolationParameters {
            sinc_len: 256,
            f_cutoff: 0.95,
            interpolation: SincInterpolationType::Cubic,
            oversampling_factor: 256,
            window: WindowFunction::BlackmanHarris2,
        },
        chunk,
        1,
    )
    .map_err(|e| e.to_string())?;
    let expected = (input.len() * OUTPUT_RATE + input_rate / 2) / input_rate;
    let mut output = Vec::with_capacity(expected + chunk);
    let input_len = resampler.input_frames_next();
    let mut pos = 0;
    while pos < input.len() {
        let n = input_len.min(input.len() - pos);
        let mut block = vec![0.0; input_len];
        block[..n].copy_from_slice(&input[pos..pos + n]);
        let out = if n == input_len {
            resampler.process(&[&block], None)
        } else {
            resampler.process_partial(Some(&[&block]), None)
        }
        .map_err(|e| e.to_string())?;
        output.extend_from_slice(&out[0]);
        pos += n;
    }
    let delay = resampler.output_delay();
    let target = delay.saturating_add(expected);
    for _ in 0..4 {
        if output.len() >= target {
            break;
        }
        let silence = vec![0.0; input_len];
        let out = resampler
            .process(&[&silence], None)
            .map_err(|e| e.to_string())?;
        output.extend_from_slice(&out[0]);
    }
    if output.len() < target {
        return Err("resampler could not flush its filter delay".into());
    }
    let end = target;
    Ok(output.get(delay..end).unwrap_or_default().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resamples_to_exact_duration_and_keeps_tone() {
        let source: Vec<f32> = (0..48_000)
            .map(|i| (std::f32::consts::TAU * 440.0 * i as f32 / 48_000.0).sin())
            .collect();
        let output = resample_mono(&source, 48_000).unwrap();
        assert_eq!(output.len(), 16_000);
        assert!(output.iter().any(|v| v.abs() > 0.5));
        assert!(resample_mono(&source, 0).is_err());
        let short: Vec<f32> = (0..317)
            .map(|i| (std::f32::consts::TAU * 700.0 * i as f32 / 44_100.0).sin())
            .collect();
        assert_eq!(resample_mono(&short, 44_100).unwrap().len(), 115);
    }

    #[test]
    fn vad_threshold_tracks_rms_and_sinc_resampling_rejects_aliases() {
        assert_eq!(vad(&[0.0; 4], 0.01), (0.0, false));
        let (level, speech) = vad(&[0.02; 4], 0.01);
        assert!((level - 0.02).abs() < 1e-6 && speech);
        let high: Vec<f32> = (0..48_000)
            .map(|i| (std::f32::consts::TAU * 12_000.0 * i as f32 / 48_000.0).sin())
            .collect();
        let output = resample_mono(&high, 48_000).unwrap();
        let steady = &output[1000..];
        let rms = (steady.iter().map(|v| v * v).sum::<f32>() / steady.len() as f32).sqrt();
        assert!(rms < 0.05, "out-of-band tone aliased at RMS {rms}");
    }
}
