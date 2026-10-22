//! Procedural ambient sound generators. Every sound is synthesised from noise
//! and oscillators, so amb ships without any audio assets.

// Per-channel state lives in parallel `[T; 2]` arrays, so indexing by channel is clearer.
#![allow(clippy::needless_range_loop)]

use std::f32::consts::TAU;

/// Tiny xorshift PRNG; plenty for audio noise and cheap enough for the callback.
pub struct Rng(u32);

impl Rng {
    pub fn new(seed: u32) -> Self {
        Self(seed.max(1))
    }

    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    /// Uniform in [-1, 1).
    pub fn bipolar(&mut self) -> f32 {
        self.next_u32() as f32 / (u32::MAX as f32 / 2.0) - 1.0
    }

    /// Uniform in [0, 1).
    pub fn unit(&mut self) -> f32 {
        self.next_u32() as f32 / (u32::MAX as f32 + 1.0)
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + (hi - lo) * self.unit()
    }
}

#[derive(Default, Clone, Copy)]
struct LowPass {
    y: f32,
}

impl LowPass {
    fn process(&mut self, x: f32, cutoff: f32, sr: f32) -> f32 {
        let a = 1.0 - (-TAU * cutoff / sr).exp();
        self.y += a * (x - self.y);
        self.y
    }
}

/// Chamberlin state-variable filter, bandpass output.
#[derive(Default, Clone, Copy)]
struct Bandpass {
    low: f32,
    band: f32,
}

impl Bandpass {
    fn process(&mut self, x: f32, freq: f32, q: f32, sr: f32) -> f32 {
        let f = 2.0 * (std::f32::consts::PI * freq / sr).sin();
        self.low += f * self.band;
        let high = x - self.low - self.band / q;
        self.band += f * high;
        self.band
    }
}

/// Slowly wandering control signal in [0, 1]: glides towards random targets.
struct Wander {
    value: f32,
    target: f32,
    countdown: u32,
}

impl Wander {
    fn new() -> Self {
        Self {
            value: 0.5,
            target: 0.5,
            countdown: 0,
        }
    }

    fn next(&mut self, rng: &mut Rng, sr: f32, min_s: f32, max_s: f32, glide: f32) -> f32 {
        if self.countdown == 0 {
            self.target = rng.unit();
            self.countdown = (rng.range(min_s, max_s) * sr) as u32;
        }
        self.countdown -= 1;
        self.value += glide * (self.target - self.value);
        self.value
    }
}

/// Sample rate the per-sample constants below were tuned at.
const REF_SR: f32 = 44_100.0;

/// Re-express a per-sample decay/leak factor tuned at `REF_SR` for `sr`.
fn decay_at(factor: f32, sr: f32) -> f32 {
    factor.powf(REF_SR / sr)
}

/// Re-express a one-pole glide coefficient tuned at `REF_SR` for `sr`.
fn glide_at(glide: f32, sr: f32) -> f32 {
    1.0 - decay_at(1.0 - glide, sr)
}

/// Per-sample step of a random walk, scaled so its loudness is rate-independent.
fn walk_step_at(step: f32, sr: f32) -> f32 {
    step * (REF_SR / sr).sqrt()
}

pub trait Synth: Send {
    /// Next stereo frame at roughly unit amplitude.
    fn next(&mut self) -> (f32, f32);
}

pub fn make(kind: Kind, sr: f32, seed: u32) -> Box<dyn Synth> {
    let rng = Rng::new(seed);
    match kind {
        Kind::Rain => Box::new(Rain::new(sr, rng)),
        Kind::Thunder => Box::new(Thunder::new(sr, rng)),
        Kind::Wind => Box::new(Wind::new(sr, rng)),
        Kind::Waves => Box::new(Waves::new(sr, rng)),
        Kind::Fire => Box::new(Fire::new(sr, rng)),
        Kind::Stream => Box::new(Stream::new(sr, rng)),
        Kind::Birds => Box::new(Birds::new(sr, rng)),
        Kind::Crickets => Box::new(Crickets::new(sr, rng)),
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Rain,
    Thunder,
    Wind,
    Waves,
    Fire,
    Stream,
    Birds,
    Crickets,
}

impl Kind {
    pub const ALL: [Kind; 8] = [
        Kind::Rain,
        Kind::Thunder,
        Kind::Wind,
        Kind::Waves,
        Kind::Fire,
        Kind::Stream,
        Kind::Birds,
        Kind::Crickets,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Kind::Rain => "Rain",
            Kind::Thunder => "Thunder",
            Kind::Wind => "Wind",
            Kind::Waves => "Waves",
            Kind::Fire => "Fire",
            Kind::Stream => "Stream",
            Kind::Birds => "Birds",
            Kind::Crickets => "Crickets",
        }
    }
}

// ---------------------------------------------------------------- rain

struct Rain {
    sr: f32,
    rng: Rng,
    lp: [LowPass; 2],
    hp: [LowPass; 2],
    drops: [Raindrop; 2],
    drop_decay: f32,
}

#[derive(Default)]
struct Raindrop {
    env: f32,
    phase: f32,
    freq: f32,
}

impl Rain {
    fn new(sr: f32, rng: Rng) -> Self {
        Self {
            sr,
            rng,
            lp: Default::default(),
            hp: Default::default(),
            drops: Default::default(),
            drop_decay: decay_at(0.9985, sr),
        }
    }
}

impl Synth for Rain {
    fn next(&mut self) -> (f32, f32) {
        let mut out = [0.0; 2];
        for c in 0..2 {
            let white = self.rng.bipolar();
            let band = self.lp[c].process(white, 7000.0, self.sr);
            let hissy = band - self.hp[c].process(band, 600.0, self.sr);
            let d = &mut self.drops[c];
            if self.rng.unit() < 40.0 / self.sr {
                d.env = self.rng.range(0.2, 0.8);
                d.freq = self.rng.range(1800.0, 5500.0);
            }
            d.phase = (d.phase + d.freq / self.sr).fract();
            let drop = (d.phase * TAU).sin() * d.env;
            d.env *= self.drop_decay;
            out[c] = hissy * 0.55 + drop * 0.25;
        }
        (out[0], out[1])
    }
}

// ------------------------------------------------------------- thunder

struct Thunder {
    sr: f32,
    rng: Rng,
    brown: [f32; 2],
    lp: [LowPass; 2],
    env: f32,
    target: f32,
    decay: f32,
    countdown: u32,
    leak: f32,
    step: f32,
}

impl Thunder {
    fn new(sr: f32, mut rng: Rng) -> Self {
        let countdown = (rng.range(2.0, 8.0) * sr) as u32;
        Self {
            sr,
            rng,
            brown: [0.0; 2],
            lp: Default::default(),
            env: 0.0,
            target: 0.0,
            decay: 0.0,
            countdown,
            leak: decay_at(0.998, sr),
            step: walk_step_at(0.02, sr),
        }
    }
}

impl Synth for Thunder {
    fn next(&mut self) -> (f32, f32) {
        if self.countdown == 0 {
            self.target = self.rng.range(0.5, 1.0);
            self.decay = self.rng.range(2.5, 6.0);
            self.countdown = (self.rng.range(8.0, 30.0) * self.sr) as u32;
        }
        self.countdown -= 1;
        // Fast-ish attack towards the peak, then a long exponential tail.
        if self.target > 0.01 {
            self.env += (self.target - self.env) * (1.0 / (0.4 * self.sr));
            self.target *= (-1.0 / (self.decay * self.sr)).exp();
        } else {
            self.env *= (-1.0 / (self.decay * self.sr)).exp();
        }
        let mut out = [0.0; 2];
        for c in 0..2 {
            self.brown[c] = (self.brown[c] + self.rng.bipolar() * self.step) * self.leak;
            let rumble = self.lp[c].process(self.brown[c], 180.0, self.sr);
            out[c] = rumble * 5.0 * self.env;
        }
        (out[0], out[1])
    }
}

// ---------------------------------------------------------------- wind

struct Wind {
    sr: f32,
    rng: Rng,
    bp: [Bandpass; 2],
    sweep: [Wander; 2],
    gust: Wander,
    gust_glide: f32,
    sweep_glide: f32,
}

impl Wind {
    fn new(sr: f32, rng: Rng) -> Self {
        Self {
            sr,
            rng,
            bp: Default::default(),
            sweep: [Wander::new(), Wander::new()],
            gust: Wander::new(),
            gust_glide: glide_at(0.00004, sr),
            sweep_glide: glide_at(0.00006, sr),
        }
    }
}

impl Synth for Wind {
    fn next(&mut self) -> (f32, f32) {
        let gust = self
            .gust
            .next(&mut self.rng, self.sr, 3.0, 8.0, self.gust_glide);
        let mut out = [0.0; 2];
        for c in 0..2 {
            let s = self.sweep[c].next(&mut self.rng, self.sr, 2.0, 6.0, self.sweep_glide);
            let freq = 250.0 + 700.0 * s * (0.5 + gust);
            let white = self.rng.bipolar();
            out[c] = self.bp[c].process(white, freq, 1.4, self.sr) * (0.5 + 1.1 * gust);
        }
        (out[0], out[1])
    }
}

// --------------------------------------------------------------- waves

struct Waves {
    sr: f32,
    rng: Rng,
    lp: [LowPass; 2],
    lp2: [LowPass; 2],
    phase: [f32; 2],
    period: Wander,
    period_glide: f32,
}

impl Waves {
    fn new(sr: f32, rng: Rng) -> Self {
        Self {
            sr,
            rng,
            lp: Default::default(),
            lp2: Default::default(),
            phase: [0.0, 0.31],
            period: Wander::new(),
            period_glide: glide_at(0.00002, sr),
        }
    }
}

impl Synth for Waves {
    fn next(&mut self) -> (f32, f32) {
        let rate = 0.09
            + 0.06
                * self
                    .period
                    .next(&mut self.rng, self.sr, 4.0, 10.0, self.period_glide);
        let mut out = [0.0; 2];
        for c in 0..2 {
            self.phase[c] = (self.phase[c] + rate / self.sr).fract();
            let swell = 0.5 - 0.5 * (self.phase[c] * TAU).cos();
            let swell = swell * swell;
            let white = self.rng.bipolar();
            // The wash gets brighter as the wave breaks.
            let cutoff = 400.0 + 2600.0 * swell;
            let wash = self.lp[c].process(white, cutoff, self.sr);
            let wash = self.lp2[c].process(wash, cutoff, self.sr);
            out[c] = wash * (0.15 + swell) * 2.2;
        }
        (out[0], out[1])
    }
}

// ---------------------------------------------------------------- fire

struct Fire {
    sr: f32,
    rng: Rng,
    brown: f32,
    lp: LowPass,
    hp: [LowPass; 2],
    pops: [f32; 2],
    leak: f32,
    step: f32,
    pop_decay: f32,
}

impl Fire {
    fn new(sr: f32, rng: Rng) -> Self {
        Self {
            sr,
            rng,
            brown: 0.0,
            lp: Default::default(),
            hp: Default::default(),
            pops: [0.0; 2],
            leak: decay_at(0.998, sr),
            step: walk_step_at(0.03, sr),
            pop_decay: decay_at(0.9965, sr),
        }
    }
}

impl Synth for Fire {
    fn next(&mut self) -> (f32, f32) {
        self.brown = (self.brown + self.rng.bipolar() * self.step) * self.leak;
        let body = self.lp.process(self.brown, 500.0, self.sr) * 1.0;
        let mut out = [body * 0.8; 2];
        for c in 0..2 {
            if self.rng.unit() < 14.0 / self.sr {
                self.pops[c] = self.rng.range(0.3, 1.0);
            }
            let n = self.rng.bipolar();
            // Low-pass state subtracted from the input leaves the high-passed snap.
            let crack = n - self.hp[c].process(n, 2500.0, self.sr);
            out[c] += crack * self.pops[c] * 0.9;
            self.pops[c] *= self.pop_decay;
        }
        (out[0], out[1])
    }
}

// -------------------------------------------------------------- stream

struct Stream {
    sr: f32,
    rng: Rng,
    bp: [Bandpass; 2],
    lp: [LowPass; 2],
    babble: [LowPass; 2],
    lp_mod: [LowPass; 2],
}

impl Stream {
    fn new(sr: f32, rng: Rng) -> Self {
        Self {
            sr,
            rng,
            bp: Default::default(),
            lp: Default::default(),
            babble: Default::default(),
            lp_mod: Default::default(),
        }
    }
}

impl Synth for Stream {
    fn next(&mut self) -> (f32, f32) {
        let mut out = [0.0; 2];
        for c in 0..2 {
            // Band-limited noise as a ~12 Hz random control signal -> babbling.
            let m = self.babble[c].process(self.rng.bipolar(), 14.0, self.sr) * 45.0;
            let m = self.lp_mod[c].process(m, 30.0, self.sr).clamp(-1.0, 1.0);
            let freq = 1100.0 + 900.0 * m;
            let white = self.rng.bipolar();
            let burble = self.bp[c].process(white, freq, 2.5, self.sr);
            let base = self.lp[c].process(white, 2500.0, self.sr);
            out[c] = burble * 0.9 + base * 0.25;
        }
        (out[0], out[1])
    }
}

// --------------------------------------------------------------- birds

struct Birds {
    sr: f32,
    rng: Rng,
    countdown: u32,
    notes_left: u32,
    note_samples: u32,
    gap_samples: u32,
    pos: u32,
    f0: f32,
    slope: f32,
    phase: f32,
    pan: f32,
    amp: f32,
}

impl Birds {
    fn new(sr: f32, mut rng: Rng) -> Self {
        let countdown = (rng.range(0.5, 2.0) * sr) as u32;
        Self {
            sr,
            rng,
            countdown,
            notes_left: 0,
            note_samples: 1,
            gap_samples: 0,
            pos: 0,
            f0: 3000.0,
            slope: 0.0,
            phase: 0.0,
            pan: 0.5,
            amp: 0.0,
        }
    }

    fn start_phrase(&mut self) {
        self.notes_left = (self.rng.range(1.0, 6.0)) as u32;
        self.note_samples = (self.rng.range(0.04, 0.16) * self.sr) as u32;
        self.gap_samples = (self.rng.range(0.03, 0.09) * self.sr) as u32;
        self.f0 = self.rng.range(2200.0, 5200.0);
        self.slope = self.rng.range(-1500.0, 2500.0);
        self.pan = self.rng.unit();
        self.amp = self.rng.range(0.25, 0.6);
        self.pos = 0;
    }
}

impl Synth for Birds {
    fn next(&mut self) -> (f32, f32) {
        if self.notes_left == 0 {
            if self.countdown == 0 {
                self.start_phrase();
                self.countdown = (self.rng.range(1.0, 5.0) * self.sr) as u32;
            } else {
                self.countdown -= 1;
            }
            return (0.0, 0.0);
        }
        let total = self.note_samples + self.gap_samples;
        let s = if self.pos < self.note_samples {
            let t = self.pos as f32 / self.note_samples as f32;
            let env = (t * std::f32::consts::PI).sin();
            let freq = self.f0 + self.slope * t;
            self.phase = (self.phase + freq / self.sr).fract();
            // A touch of 2nd harmonic keeps it from sounding like a test tone.
            ((self.phase * TAU).sin() + 0.25 * (self.phase * 2.0 * TAU).sin()) * env * self.amp
        } else {
            0.0
        };
        self.pos += 1;
        if self.pos >= total {
            self.pos = 0;
            self.notes_left -= 1;
            // Each note in a phrase drifts a little in pitch.
            self.f0 *= self.rng.range(0.92, 1.1);
        }
        (s * (1.0 - self.pan) * 2.0, s * self.pan * 2.0)
    }
}

// ------------------------------------------------------------ crickets

struct Cricket {
    freq: f32,
    phase: f32,
    pulse_period: f32,
    pulses: u32,
    cycle: f32,
    t: f32,
}

struct Crickets {
    sr: f32,
    crickets: [Cricket; 3],
}

impl Crickets {
    fn new(sr: f32, mut rng: Rng) -> Self {
        let mut mk = |freq: f32, cycle: f32| Cricket {
            freq,
            phase: 0.0,
            pulse_period: rng.range(0.032, 0.042),
            pulses: 3,
            cycle,
            t: rng.range(0.0, cycle),
        };
        Self {
            sr,
            crickets: [mk(4300.0, 0.62), mk(4650.0, 0.71), mk(4050.0, 0.83)],
        }
    }
}

impl Synth for Crickets {
    fn next(&mut self) -> (f32, f32) {
        let mut out = [0.0f32; 2];
        for (i, c) in self.crickets.iter_mut().enumerate() {
            c.t += 1.0 / self.sr;
            if c.t >= c.cycle {
                c.t -= c.cycle;
            }
            let burst = c.pulses as f32 * c.pulse_period;
            let amp = if c.t < burst {
                let p = (c.t % c.pulse_period) / c.pulse_period;
                // Raised-cosine pulse, ~60% duty.
                if p < 0.6 {
                    (p / 0.6 * std::f32::consts::PI).sin()
                } else {
                    0.0
                }
            } else {
                0.0
            };
            c.phase = (c.phase + c.freq / self.sr).fract();
            let s = (c.phase * TAU).sin() * amp * 0.25;
            let pan = [0.25, 0.5, 0.8][i];
            out[0] += s * (1.0 - pan) * 2.4;
            out[1] += s * pan * 2.4;
        }
        (out[0], out[1])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_synth_is_finite_and_audible() {
        for sr in [44_100.0, 48_000.0] {
            for (i, kind) in Kind::ALL.into_iter().enumerate() {
                let mut s = make(kind, sr, i as u32 + 1);
                let (mut peak, mut energy) = (0.0f32, 0.0f32);
                // 60s covers thunder/bird/cricket scheduling.
                for _ in 0..(sr as usize * 60) {
                    let (l, r) = s.next();
                    assert!(l.is_finite() && r.is_finite(), "{kind:?} produced NaN/inf");
                    peak = peak.max(l.abs()).max(r.abs());
                    energy += l * l + r * r;
                }
                println!(
                    "{kind:?}: peak {peak:.2} rms {:.3}",
                    (energy / (sr * 120.0)).sqrt()
                );
                assert!(peak < 4.0, "{kind:?} peak {peak} is far too hot");
                assert!(energy > 1.0, "{kind:?} is silent");
            }
        }
    }
}
