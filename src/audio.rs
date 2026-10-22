//! Audio engine: one endless rodio source that mixes every synth.

use std::num::NonZero;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::{ChannelCount, MixerDeviceSink, Sample, SampleRate, Source};

use crate::synth::{self, Kind, Synth};

/// Clamp to [0, 1]; NaN (e.g. from a hand-edited levels file) becomes 0 so it
/// can never reach the audio path.
fn sanitize(v: f32) -> f32 {
    if v.is_nan() { 0.0 } else { v.clamp(0.0, 1.0) }
}

/// Levels shared between the UI thread and the audio callback.
pub struct Shared {
    levels: [AtomicU32; Kind::ALL.len()],
    master: AtomicU32,
    paused: AtomicBool,
}

impl Shared {
    pub fn new() -> Self {
        Self {
            levels: std::array::from_fn(|_| AtomicU32::new(0)),
            master: AtomicU32::new(0.8f32.to_bits()),
            paused: AtomicBool::new(false),
        }
    }

    pub fn level(&self, i: usize) -> f32 {
        f32::from_bits(self.levels[i].load(Ordering::Relaxed))
    }

    pub fn set_level(&self, i: usize, v: f32) {
        self.levels[i].store(sanitize(v).to_bits(), Ordering::Relaxed);
    }

    pub fn master(&self) -> f32 {
        f32::from_bits(self.master.load(Ordering::Relaxed))
    }

    pub fn set_master(&self, v: f32) {
        self.master.store(sanitize(v).to_bits(), Ordering::Relaxed);
    }

    pub fn paused(&self) -> bool {
        self.paused.load(Ordering::Relaxed)
    }

    pub fn set_paused(&self, v: bool) {
        self.paused.store(v, Ordering::Relaxed);
    }
}

pub struct Engine {
    pub shared: Arc<Shared>,
    // Dropping the sink stops playback, so it lives as long as the engine.
    _sink: MixerDeviceSink,
}

impl Engine {
    pub fn start() -> Result<Self> {
        let mut sink = rodio::DeviceSinkBuilder::open_default_sink()
            .context("could not open the default audio output")?;
        // The TUI owns the terminal; rodio's drop message would corrupt it.
        sink.log_on_drop(false);
        let sr = sink.config().sample_rate();
        let shared = Arc::new(Shared::new());
        sink.mixer().add(Ambience::new(sr, shared.clone()));
        Ok(Self {
            shared,
            _sink: sink,
        })
    }
}

struct Ambience {
    sr: SampleRate,
    shared: Arc<Shared>,
    synths: Vec<Box<dyn Synth>>,
    /// Smoothed gains, so level changes never click.
    gains: [f32; Kind::ALL.len()],
    master: f32,
    pause_gain: f32,
    pending_right: Option<f32>,
    smooth: f32,
}

impl Ambience {
    fn new(sr: SampleRate, shared: Arc<Shared>) -> Self {
        let srf = sr.get() as f32;
        let synths = Kind::ALL
            .iter()
            .enumerate()
            .map(|(i, &k)| synth::make(k, srf, 0x9E37_79B9u32.wrapping_mul(i as u32 + 1)))
            .collect();
        Self {
            sr,
            shared,
            synths,
            gains: [0.0; Kind::ALL.len()],
            master: 0.0,
            pause_gain: 1.0,
            pending_right: None,
            smooth: 1.0 - (-1.0 / (0.03 * srf)).exp(),
        }
    }

    fn frame(&mut self) -> (f32, f32) {
        let k = self.smooth;
        let target_master = self.shared.master();
        let target_pause = if self.shared.paused() { 0.0 } else { 1.0 };
        self.master += k * (target_master * target_master - self.master);
        self.pause_gain += k * (target_pause - self.pause_gain);

        // Fully faded out: skip synthesis so a paused session costs ~nothing.
        if target_pause == 0.0 && self.pause_gain < 1e-4 {
            return (0.0, 0.0);
        }

        let (mut l, mut r) = (0.0, 0.0);
        for (i, synth) in self.synths.iter_mut().enumerate() {
            let t = self.shared.level(i);
            self.gains[i] += k * (t * t - self.gains[i]);
            // Silent channels skip synthesis entirely.
            if self.gains[i] < 1e-5 && t == 0.0 {
                continue;
            }
            let (sl, sr) = synth.next();
            l += sl * self.gains[i];
            r += sr * self.gains[i];
        }
        let g = self.master * self.pause_gain;
        // Soft clip so stacking many sounds never hard-clips.
        ((l * g).tanh(), (r * g).tanh())
    }
}

impl Iterator for Ambience {
    type Item = Sample;

    fn next(&mut self) -> Option<Sample> {
        if let Some(r) = self.pending_right.take() {
            return Some(r as Sample);
        }
        let (l, r) = self.frame();
        self.pending_right = Some(r);
        Some(l as Sample)
    }
}

impl Source for Ambience {
    fn current_span_len(&self) -> Option<usize> {
        None
    }

    fn channels(&self) -> ChannelCount {
        NonZero::new(2).unwrap()
    }

    fn sample_rate(&self) -> SampleRate {
        self.sr
    }

    fn total_duration(&self) -> Option<Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_are_clamped_and_nan_safe() {
        let s = Shared::new();
        s.set_level(0, f32::NAN);
        assert_eq!(s.level(0), 0.0);
        s.set_level(0, 7.0);
        assert_eq!(s.level(0), 1.0);
        s.set_master(-1.0);
        assert_eq!(s.master(), 0.0);
    }
}
