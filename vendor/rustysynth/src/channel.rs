#![allow(dead_code)]

#[derive(Debug, PartialEq, Eq)]
enum DataType {
    None,
    Rpn,
    Nrpn,
}

#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct Channel {
    pub(crate) is_percussion_channel: bool,

    bank_number: i32,
    patch_number: i32,

    modulation: i16,
    volume: i16,
    pan: i16,
    expression: i16,
    hold_pedal: bool,

    reverb_send: u8,
    chorus_send: u8,

    rpn: i16,
    pitch_bend_range: i16,
    coarse_tune: i16,
    fine_tune: i16,

    pitch_bend: f32,

    last_data_type: DataType,

    // yahaha: the GM2/XG sound controllers (#246), 64 = the voice's own: CC74 cutoff,
    // CC71 resonance, CC73 attack, CC75 decay, CC72 release, CC76 vibrato rate, CC77
    // depth, CC78 delay (`SOUND_*`); and what they come to, worked out when they change.
    sound: [u8; SOUND_CONTROLLERS],
    cutoff_factor: f32,
    resonance_db: f32,
    envelope_factors: [f32; 3],
    vibrato_rate_factor: f32,
    vibrato_depth: f32,
    vibrato_delay: f32,
}

/// yahaha: `Channel::sound` indices.
pub(crate) const SOUND_CUTOFF: usize = 0;
pub(crate) const SOUND_RESONANCE: usize = 1;
pub(crate) const SOUND_ATTACK: usize = 2;
pub(crate) const SOUND_DECAY: usize = 3;
pub(crate) const SOUND_RELEASE: usize = 4;
pub(crate) const SOUND_VIBRATO_RATE: usize = 5;
pub(crate) const SOUND_VIBRATO_DEPTH: usize = 6;
pub(crate) const SOUND_VIBRATO_DELAY: usize = 7;
pub(crate) const SOUND_CONTROLLERS: usize = 8;

/// yahaha: steps of a sound controller per octave of cutoff, envelope time or vibrato rate:
/// 16, so the -64..+63 range spans about 4 octaves each way (the XG Drum Setup's scale,
/// yahaha's `drum_setup.rs`).
const STEPS_PER_OCTAVE: f32 = 16_f32;
/// yahaha: resonance, in decibels per step (about +-12 dB over the range).
const RESONANCE_DB_PER_STEP: f32 = 0.2_f32;
/// yahaha: vibrato depth, in semitones per step: +63 adds about half a semitone (the mod
/// wheel's full depth), -64 takes as much away.
const VIBRATO_DEPTH_PER_STEP: f32 = 0.5_f32 / 64_f32;
/// yahaha: vibrato delay, in seconds per step.
const VIBRATO_DELAY_PER_STEP: f32 = 0.02_f32;

impl Channel {
    pub(crate) fn new(is_percussion_channel: bool) -> Self {
        let mut channel = Self {
            is_percussion_channel,
            bank_number: 0,
            patch_number: 0,
            modulation: 0,
            volume: 0,
            pan: 0,
            expression: 0,
            hold_pedal: false,
            reverb_send: 0,
            chorus_send: 0,
            rpn: 0,
            pitch_bend_range: 0,
            coarse_tune: 0,
            fine_tune: 0,
            pitch_bend: 0_f32,
            last_data_type: DataType::None,
            sound: [64; SOUND_CONTROLLERS],
            cutoff_factor: 1_f32,
            resonance_db: 0_f32,
            envelope_factors: [1_f32; 3],
            vibrato_rate_factor: 1_f32,
            vibrato_depth: 0_f32,
            vibrato_delay: 0_f32,
        };

        channel.reset();

        channel
    }

    pub(crate) fn reset(&mut self) {
        self.bank_number = if self.is_percussion_channel { 128 } else { 0 };
        self.patch_number = 0;

        self.modulation = 0;
        self.volume = 100 << 7;
        self.pan = 64 << 7;
        self.expression = 127 << 7;
        self.hold_pedal = false;

        self.reverb_send = 40;
        self.chorus_send = 0;

        self.rpn = -1;
        self.pitch_bend_range = 2 << 7;
        self.coarse_tune = 0;
        self.fine_tune = 8192;

        self.pitch_bend = 0_f32;

        for i in 0..SOUND_CONTROLLERS {
            self.set_sound(i, 64);
        }
    }

    // yahaha: Reset All Controllers leaves the sound controllers as they are (GM2, XG).
    pub(crate) fn reset_all_controllers(&mut self) {
        self.modulation = 0;
        self.expression = 127 << 7;
        self.hold_pedal = false;

        self.rpn = -1;

        self.pitch_bend = 0_f32;
    }

    pub(crate) fn set_bank(&mut self, value: i32) {
        self.bank_number = value;

        if self.is_percussion_channel {
            self.bank_number += 128;
        }
    }

    pub(crate) fn set_patch(&mut self, value: i32) {
        self.patch_number = value;
    }

    pub(crate) fn set_modulation_coarse(&mut self, value: i32) {
        self.modulation = (self.modulation & 0x7F) | (value << 7) as i16;
    }

    pub(crate) fn set_modulation_fine(&mut self, value: i32) {
        self.modulation = (((self.modulation as i32) & 0xFF80) | value) as i16;
    }

    pub(crate) fn set_volume_coarse(&mut self, value: i32) {
        self.volume = (self.volume & 0x7F) | (value << 7) as i16;
    }

    pub(crate) fn set_volume_fine(&mut self, value: i32) {
        self.volume = (((self.volume as i32) & 0xFF80) | value) as i16;
    }

    pub(crate) fn set_pan_coarse(&mut self, value: i32) {
        self.pan = (self.pan & 0x7F) | (value << 7) as i16;
    }

    pub(crate) fn set_pan_fine(&mut self, value: i32) {
        self.pan = (((self.pan as i32) & 0xFF80) | value) as i16;
    }

    pub(crate) fn set_expression_coarse(&mut self, value: i32) {
        self.expression = (self.expression & 0x7F) | (value << 7) as i16;
    }

    pub(crate) fn set_expression_fine(&mut self, value: i32) {
        self.expression = (((self.expression as i32) & 0xFF80) | value) as i16;
    }

    pub(crate) fn set_hold_pedal(&mut self, value: i32) {
        self.hold_pedal = value >= 64;
    }

    pub(crate) fn set_reverb_send(&mut self, value: i32) {
        self.reverb_send = value as u8;
    }

    pub(crate) fn set_chorus_send(&mut self, value: i32) {
        self.chorus_send = value as u8;
    }

    pub(crate) fn set_rpn_coarse(&mut self, value: i32) {
        self.rpn = (self.rpn & 0x7F) | (value << 7) as i16;
        self.last_data_type = DataType::Rpn;
    }

    pub(crate) fn set_rpn_fine(&mut self, value: i32) {
        self.rpn = (((self.rpn as i32) & 0xFF80) | value) as i16;
        self.last_data_type = DataType::Rpn;
    }

    pub(crate) fn set_nrpn_coarse(&mut self, _value: i32) {
        self.last_data_type = DataType::Nrpn;
    }

    pub(crate) fn set_nrpn_fine(&mut self, _value: i32) {
        self.last_data_type = DataType::Nrpn;
    }

    pub(crate) fn data_entry_coarse(&mut self, value: i32) {
        if self.last_data_type != DataType::Rpn {
            return;
        }

        if self.rpn == 0 {
            self.pitch_bend_range = (self.pitch_bend_range & 0x7F) | (value << 7) as i16;
        } else if self.rpn == 1 {
            self.fine_tune = (self.fine_tune & 0x7F) | (value << 7) as i16;
        } else if self.rpn == 2 {
            self.coarse_tune = (value - 64) as i16;
        }
    }

    pub(crate) fn data_entry_fine(&mut self, value: i32) {
        if self.last_data_type != DataType::Rpn {
            return;
        }

        if self.rpn == 0 {
            self.pitch_bend_range = (((self.pitch_bend_range as i32) & 0xFF80) | value) as i16;
        } else if self.rpn == 1 {
            self.fine_tune = (((self.fine_tune as i32) & 0xFF80) | value) as i16;
        }
    }

    pub(crate) fn set_pitch_bend(&mut self, value1: i32, value2: i32) {
        self.pitch_bend = (1_f32 / 8192_f32) * ((value1 | (value2 << 7)) - 8192) as f32;
    }

    /// yahaha: sound controller `i` (`SOUND_*`) to `value` (64 = the voice's own).
    pub(crate) fn set_sound(&mut self, i: usize, value: i32) {
        let value = value.clamp(0, 127) as u8;
        self.sound[i] = value;
        let d = value as f32 - 64_f32;
        let octaves = |d: f32| (d / STEPS_PER_OCTAVE).exp2();
        match i {
            SOUND_CUTOFF => self.cutoff_factor = octaves(d),
            SOUND_RESONANCE => self.resonance_db = d * RESONANCE_DB_PER_STEP,
            SOUND_ATTACK => self.envelope_factors[0] = octaves(d),
            SOUND_DECAY => self.envelope_factors[1] = octaves(d),
            SOUND_RELEASE => self.envelope_factors[2] = octaves(d),
            SOUND_VIBRATO_RATE => self.vibrato_rate_factor = octaves(d),
            SOUND_VIBRATO_DEPTH => self.vibrato_depth = d * VIBRATO_DEPTH_PER_STEP,
            SOUND_VIBRATO_DELAY => self.vibrato_delay = d * VIBRATO_DELAY_PER_STEP,
            _ => {}
        }
    }

    /// yahaha: the filter cutoff scaled by this (CC74; 1 = the voice's own).
    pub(crate) fn get_cutoff_factor(&self) -> f32 {
        self.cutoff_factor
    }

    /// yahaha: decibels added to the filter resonance (CC71).
    pub(crate) fn get_resonance_db(&self) -> f32 {
        self.resonance_db
    }

    /// yahaha: the volume envelope's attack, decay and release times scaled by these
    /// (CC73, CC75, CC72).
    pub(crate) fn get_envelope_factors(&self) -> [f32; 3] {
        self.envelope_factors
    }

    /// yahaha: the vibrato rate scaled by this (CC76).
    pub(crate) fn get_vibrato_rate_factor(&self) -> f32 {
        self.vibrato_rate_factor
    }

    /// yahaha: semitones added to the vibrato depth (CC77).
    pub(crate) fn get_vibrato_depth(&self) -> f32 {
        self.vibrato_depth
    }

    /// yahaha: seconds added to the vibrato delay (CC78).
    pub(crate) fn get_vibrato_delay(&self) -> f32 {
        self.vibrato_delay
    }

    /// yahaha: whether any sound controller differs from 64.
    pub(crate) fn has_sound(&self) -> bool {
        self.sound != [64; SOUND_CONTROLLERS]
    }

    pub(crate) fn get_bank_number(&self) -> i32 {
        self.bank_number
    }

    pub(crate) fn get_patch_number(&self) -> i32 {
        self.patch_number
    }

    pub(crate) fn get_modulation(&self) -> f32 {
        (50_f32 / 16383_f32) * self.modulation as f32
    }

    pub(crate) fn get_volume(&self) -> f32 {
        (1_f32 / 16383_f32) * self.volume as f32
    }

    pub(crate) fn get_pan(&self) -> f32 {
        (100_f32 / 16383_f32) * self.pan as f32 - 50_f32
    }

    pub(crate) fn get_expression(&self) -> f32 {
        (1_f32 / 16383_f32) * self.expression as f32
    }

    pub(crate) fn get_hold_pedal(&self) -> bool {
        self.hold_pedal
    }

    pub(crate) fn get_reverb_send(&self) -> f32 {
        (1_f32 / 127_f32) * self.reverb_send as f32
    }

    pub(crate) fn get_chorus_send(&self) -> f32 {
        (1_f32 / 127_f32) * self.chorus_send as f32
    }

    pub(crate) fn get_pitch_bend_range(&self) -> f32 {
        (self.pitch_bend_range >> 7) as f32 + 0.01_f32 * (self.pitch_bend_range & 0x7F) as f32
    }

    pub(crate) fn get_tune(&self) -> f32 {
        self.coarse_tune as f32 + (1_f32 / 8192_f32) * (self.fine_tune - 8192) as f32
    }

    pub(crate) fn get_pitch_bend(&self) -> f32 {
        self.get_pitch_bend_range() * self.pitch_bend
    }
}
