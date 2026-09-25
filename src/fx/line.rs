//! Delay lines and the small filters the effect blocks are built from. Allocation only in
//! the constructors.

/// A delay line: a power-of-two ring written one sample at a time, read at any delay up
/// to its capacity.
pub struct Line {
    buf: Vec<f32>,
    mask: usize,
    pos: usize,
}

impl Line {
    /// A line that can delay by up to `max` samples (plus a few for interpolation).
    pub fn new(max: usize) -> Line {
        let size = (max + 4).next_power_of_two();
        Line { buf: vec![0.0; size], mask: size - 1, pos: 0 }
    }

    /// The longest delay it can read.
    pub fn capacity(&self) -> usize {
        self.buf.len() - 4
    }

    #[inline]
    pub fn write(&mut self, x: f32) {
        self.pos = (self.pos + 1) & self.mask;
        self.buf[self.pos] = x;
    }

    /// The sample written `d` samples ago (0 = the last one written).
    #[inline]
    pub fn read(&self, d: usize) -> f32 {
        self.buf[self.pos.wrapping_sub(d) & self.mask]
    }

    /// The signal `d` samples ago, between samples by linear interpolation.
    #[inline]
    pub fn read_frac(&self, d: f32) -> f32 {
        let d = d.max(0.0);
        let i = d as usize;
        let f = d - i as f32;
        let a = self.read(i);
        let b = self.read(i + 1);
        a + (b - a) * f
    }

    pub fn clear(&mut self) {
        self.buf.fill(0.0);
    }
}

/// A Schroeder allpass diffuser: smears an impulse into a dense burst with a flat
/// spectrum.
pub struct Allpass {
    line: Line,
    len: usize,
    pub g: f32,
}

impl Allpass {
    pub fn new(len: usize, g: f32) -> Allpass {
        Allpass { line: Line::new(len), len: len.max(1), g }
    }

    #[inline]
    pub fn tick(&mut self, x: f32) -> f32 {
        let d = self.line.read(self.len - 1);
        let v = x - self.g * d;
        self.line.write(v);
        d + self.g * v
    }

    pub fn clear(&mut self) {
        self.line.clear();
    }
}

/// A one-pole lowpass (6 dB/octave).
#[derive(Clone, Copy)]
pub struct OnePole {
    a: f32,
    y: f32,
}

impl OnePole {
    /// Cut-off `hz` at `rate`.
    pub fn new(hz: f32, rate: f32) -> OnePole {
        let mut p = OnePole { a: 0.0, y: 0.0 };
        p.set(hz, rate);
        p
    }

    pub fn set(&mut self, hz: f32, rate: f32) {
        self.a = (-std::f32::consts::TAU * hz.min(rate * 0.49) / rate).exp();
    }

    #[inline]
    pub fn tick(&mut self, x: f32) -> f32 {
        self.y = x + self.a * (self.y - x);
        self.y
    }

    pub fn clear(&mut self) {
        self.y = 0.0;
    }
}

/// A sine LFO as a rotating phasor: two multiplies a sample, no `sin`.
#[derive(Clone, Copy)]
pub struct Lfo {
    c: f32,
    s: f32,
    rc: f32,
    rs: f32,
    n: u32,
}

impl Lfo {
    /// `hz` at `rate`, starting at `phase` (radians).
    pub fn new(hz: f32, rate: f32, phase: f32) -> Lfo {
        let w = std::f32::consts::TAU * hz / rate;
        Lfo { c: phase.cos(), s: phase.sin(), rc: w.cos(), rs: w.sin(), n: 0 }
    }

    /// (sin, cos) now, then step on.
    #[inline]
    pub fn tick(&mut self) -> (f32, f32) {
        let (s, c) = (self.s, self.c);
        self.c = c * self.rc - s * self.rs;
        self.s = s * self.rc + c * self.rs;
        self.n = self.n.wrapping_add(1);
        // Keep it on the unit circle (rounding would drift it).
        if self.n & 1023 == 0 {
            let k = (3.0 - (self.c * self.c + self.s * self.s)) * 0.5;
            self.c *= k;
            self.s *= k;
        }
        (s, c)
    }
}
