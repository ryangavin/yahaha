#![allow(dead_code)]

use std::f32::consts;

use crate::bi_quad_filter::BiQuadFilter;
use crate::channel::Channel;
use crate::lfo::Lfo;
use crate::note_params::NoteParams;
use crate::modulation_envelope::ModulationEnvelope;
use crate::oscillator::Oscillator;
use crate::region_ex::RegionEx;
use crate::region_pair::RegionPair;
use crate::soundfont_math::SoundFontMath;
use crate::synthesizer::SEND_BUSES;
use crate::synthesizer_settings::SynthesizerSettings;
use crate::volume_envelope::VolumeEnvelope;

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
enum VoiceState {
    Playing = 0,
    ReleaseRequested = 1,
    Released = 2,
}

#[derive(Debug)]
#[non_exhaustive]
pub(crate) struct Voice {
    vol_env: VolumeEnvelope,
    mod_env: ModulationEnvelope,

    vib_lfo: Lfo,
    mod_lfo: Lfo,

    oscillator: Oscillator,
    filter: BiQuadFilter,

    block: Vec<f32>,

    // A sudden change in the mix gain will cause pop noise.
    // To avoid this, we save the mix gain of the previous block,
    // and smooth out the gain if the gap between the current and previous gain is too large.
    // The actual smoothing process is done in the WriteBlock method of the Synthesizer class.
    pub(crate) previous_mix_gain_left: f32,
    pub(crate) previous_mix_gain_right: f32,
    pub(crate) current_mix_gain_left: f32,
    pub(crate) current_mix_gain_right: f32,

    pub(crate) previous_reverb_send: f32,
    pub(crate) previous_chorus_send: f32,
    pub(crate) current_reverb_send: f32,
    pub(crate) current_chorus_send: f32,

    exclusive_class: i32,
    channel: i32,
    key: i32,
    velocity: i32,

    note_gain: f32,

    cutoff: f32,
    resonance: f32,

    vib_lfo_to_pitch: f32,
    mod_lfo_to_pitch: f32,
    mod_env_to_pitch: f32,

    mod_lfo_to_cutoff: i32,
    mod_env_to_cutoff: i32,
    dynamic_cutoff: bool,

    mod_lfo_to_volume: f32,
    dynamic_volume: bool,

    instrument_pan: f32,
    instrument_reverb: f32,
    instrument_chorus: f32,

    // Some instruments require fast cutoff change, which can cause pop noise.
    // This is used to smooth out the cutoff frequency.
    smoothed_cutoff: f32,

    // yahaha: apply velocity -> filter cutoff modulators (`SynthesizerSettings`).
    velocity_to_filter: bool,

    // yahaha: the channel's sound controllers (#246) as this voice plays them: the cutoff
    // factor (CC74) and resonance offset (CC71) gliding towards the channel's (the filter
    // moves live, as a Genos part's does), the filter's own resonance in dB before them,
    // and the resonance they make.
    channel_cutoff: f32,
    channel_resonance_db: f32,
    resonance_db: f32,
    live_resonance: f32,

    // yahaha: portamento (#246): semitones the pitch is still away from the key, and how
    // far it moves each block. A mono channel's new note ends this voice whatever the
    // hold pedal (`end_now`).
    glide: f32,
    glide_step: f32,
    forced_end: bool,
    block_seconds: f32,

    // yahaha: the voice's own scale of each send bus (`NoteParams::sends`), and whether
    // any differs from 1 (the synthesizer then adds the difference to the buses).
    pub(crate) note_sends: [f32; SEND_BUSES],
    pub(crate) own_sends: bool,

    voice_state: VoiceState,
    /// Time elapsed in samples
    voice_length: usize,
    min_voice_length: usize,
}

impl Voice {
    pub(crate) fn new(settings: &SynthesizerSettings) -> Self {
        Self {
            vol_env: VolumeEnvelope::new(settings),
            mod_env: ModulationEnvelope::new(settings),
            vib_lfo: Lfo::new(settings),
            mod_lfo: Lfo::new(settings),
            oscillator: Oscillator::new(settings),
            filter: BiQuadFilter::new(settings),
            block: vec![0_f32; settings.block_size],
            previous_mix_gain_left: 0_f32,
            previous_mix_gain_right: 0_f32,
            current_mix_gain_left: 0_f32,
            current_mix_gain_right: 0_f32,
            previous_reverb_send: 0_f32,
            previous_chorus_send: 0_f32,
            current_reverb_send: 0_f32,
            current_chorus_send: 0_f32,
            exclusive_class: 0,
            channel: 0,
            key: 0,
            velocity: 0,
            note_gain: 0_f32,
            cutoff: 0_f32,
            resonance: 0_f32,
            vib_lfo_to_pitch: 0_f32,
            mod_lfo_to_pitch: 0_f32,
            mod_env_to_pitch: 0_f32,
            mod_lfo_to_cutoff: 0,
            mod_env_to_cutoff: 0,
            dynamic_cutoff: false,
            mod_lfo_to_volume: 0_f32,
            dynamic_volume: false,
            instrument_pan: 0_f32,
            instrument_reverb: 0_f32,
            instrument_chorus: 0_f32,
            smoothed_cutoff: 0_f32,
            velocity_to_filter: settings.velocity_to_filter,
            channel_cutoff: 1_f32,
            channel_resonance_db: 0_f32,
            resonance_db: 0_f32,
            live_resonance: 0_f32,
            glide: 0_f32,
            glide_step: 0_f32,
            forced_end: false,
            block_seconds: settings.block_size as f32 / settings.sample_rate as f32,
            note_sends: [1_f32; SEND_BUSES],
            own_sends: false,
            voice_state: VoiceState::Playing,
            voice_length: 0,
            min_voice_length: (settings.sample_rate / 500) as usize,
        }
    }

    // yahaha: `note` fixes the voice's own gain, tuning, pan, sends, filter and envelope
    // times (`NoteParams::NEUTRAL`: upstream's voice); `channel_info`'s sound controllers
    // (#246) scale the filter, envelope and vibrato on top (all at 64: upstream's voice).
    pub(crate) fn start(
        &mut self,
        region: &RegionPair,
        channel: i32,
        key: i32,
        velocity: i32,
        note: &NoteParams,
        channel_info: &Channel,
    ) {
        self.exclusive_class = region.get_exclusive_class();
        self.channel = channel;
        self.key = key;
        self.velocity = velocity;

        if velocity > 0 {
            // According to the Polyphone's implementation, the initial attenuation should be reduced to 40%.
            // I'm not sure why, but this indeed improves the loudness variability.
            let sample_attenuation = 0.4_f32 * region.get_initial_attenuation();
            let filter_attenuation = 0.5_f32 * region.get_initial_filter_q();
            let decibels = 2_f32 * SoundFontMath::linear_to_decibels(velocity as f32 / 127_f32)
                - sample_attenuation
                - filter_attenuation;
            self.note_gain = SoundFontMath::decibels_to_linear(decibels) * note.gain;
        } else {
            self.note_gain = 0_f32;
        }

        self.cutoff = region.get_initial_filter_cutoff_frequency();
        // yahaha: velocity -> filter cutoff (the SF2 default modulator and the SoundFont's
        // own, modulator.rs). Not below the spec's lowest cutoff (1500 cents, 19.45 Hz).
        if self.velocity_to_filter && velocity > 0 {
            let cents = region.velocity_to_filter_cents(velocity);
            if cents != 0_f32 {
                let floor = self.cutoff.min(19.45_f32);
                self.cutoff =
                    (self.cutoff * SoundFontMath::cents_to_multiplying_factor(cents)).max(floor);
            }
        }
        // yahaha: the note's own cutoff (within the spec's range) and resonance.
        if note.cutoff != 1_f32 {
            self.cutoff = SoundFontMath::clamp(self.cutoff * note.cutoff, 19.45_f32, 20000_f32);
        }
        let q = region.get_initial_filter_q();
        let q = if note.resonance_db != 0_f32 {
            SoundFontMath::clamp(q + note.resonance_db, 0_f32, 96_f32)
        } else {
            q
        };
        self.resonance = SoundFontMath::decibels_to_linear(q);
        self.resonance_db = q;
        self.channel_cutoff = channel_info.get_cutoff_factor();
        self.channel_resonance_db = channel_info.get_resonance_db();
        self.live_resonance = self.channel_resonance();

        self.vib_lfo_to_pitch = 0.01_f32 * region.get_vibrato_lfo_to_pitch() as f32;
        self.mod_lfo_to_pitch = 0.01_f32 * region.get_modulation_lfo_to_pitch() as f32;
        self.mod_env_to_pitch = 0.01_f32 * region.get_modulation_envelope_to_pitch() as f32;

        self.mod_lfo_to_cutoff = region.get_modulation_lfo_to_filter_cutoff_frequency();
        self.mod_env_to_cutoff = region.get_modulation_envelope_to_filter_cutoff_frequency();
        self.dynamic_cutoff = self.mod_lfo_to_cutoff != 0 || self.mod_env_to_cutoff != 0;

        self.mod_lfo_to_volume = region.get_modulation_lfo_to_volume();
        self.dynamic_volume = self.mod_lfo_to_volume > 0.05_f32;

        self.instrument_pan = SoundFontMath::clamp(note.pan.unwrap_or(region.get_pan()), -50_f32, 50_f32);
        self.note_sends = note.sends;
        self.own_sends = note.sends.iter().any(|&s| s != 1_f32);
        self.instrument_reverb = 0.01_f32 * region.get_reverb_effects_send();
        self.instrument_chorus = 0.01_f32 * region.get_chorus_effects_send();

        let eg = channel_info.get_envelope_factors();
        let times = [note.attack * eg[0], note.decay * eg[1], note.release * eg[2]];
        RegionEx::start_volume_envelope(&mut self.vol_env, region, key, velocity, times);
        RegionEx::start_modulation_envelope(&mut self.mod_env, region, key, velocity);
        RegionEx::start_vibrato(
            &mut self.vib_lfo,
            region,
            key,
            velocity,
            channel_info.get_vibrato_rate_factor(),
            channel_info.get_vibrato_delay(),
        );
        RegionEx::start_modulation(&mut self.mod_lfo, region, key, velocity);
        RegionEx::start_oscillator(&mut self.oscillator, region);
        if note.tune != 0_f32 {
            self.oscillator.add_tune(note.tune);
        }
        self.filter.clear_buffer();
        let cutoff = self.channel_cutoff_of(self.cutoff);
        self.filter.set_low_pass_filter(cutoff, self.live_resonance);

        self.smoothed_cutoff = cutoff;

        // yahaha: portamento from the latest key played on the channel.
        (self.glide, self.glide_step) = match channel_info.glide(key) {
            Some((from, rate)) => (from, rate * self.block_seconds),
            None => (0_f32, 0_f32),
        };
        self.forced_end = false;

        self.voice_state = VoiceState::Playing;
        self.voice_length = 0;
    }

    /// yahaha: end the note even if the hold pedal is down (a mono channel's next note).
    pub(crate) fn end_now(&mut self) {
        self.end();
        self.forced_end = true;
    }

    pub(crate) fn end(&mut self) {
        if self.voice_state == VoiceState::Playing {
            self.voice_state = VoiceState::ReleaseRequested;
        }
    }

    pub(crate) fn kill(&mut self) {
        self.note_gain = 0_f32;
    }

    pub(crate) fn process(&mut self, data: &[i16], channels: &[Channel]) -> bool {
        if self.note_gain < SoundFontMath::NON_AUDIBLE {
            return false;
        }

        let channel_info = &channels[self.channel as usize];

        self.release_if_necessary(channel_info);

        if !self.vol_env.process(self.block.len()) {
            return false;
        }

        self.mod_env.process(self.block.len());
        self.vib_lfo.process();
        self.mod_lfo.process();

        // yahaha: the channel's vibrato depth (CC77) on top of the voice's, not past 0.
        let vib_depth = match channel_info.get_vibrato_depth() {
            0_f32 => self.vib_lfo_to_pitch,
            d => (self.vib_lfo_to_pitch + d).max(self.vib_lfo_to_pitch.min(0_f32)),
        };
        let vib_pitch_change =
            (0.01_f32 * channel_info.get_modulation() + vib_depth) * self.vib_lfo.get_value();
        let mod_pitch_change = self.mod_lfo_to_pitch * self.mod_lfo.get_value()
            + self.mod_env_to_pitch * self.mod_env.get_value();
        let channel_pitch_change = channel_info.get_tune() + channel_info.get_pitch_bend();
        let mut pitch = self.key as f32 + vib_pitch_change + mod_pitch_change + channel_pitch_change;
        // yahaha: portamento: the pitch moves to the key at a fixed rate.
        if self.glide != 0_f32 {
            pitch += self.glide;
            self.glide = if self.glide.abs() <= self.glide_step {
                0_f32
            } else {
                self.glide - self.glide_step.copysign(self.glide)
            };
        }
        if !self.oscillator.process(data, &mut self.block[..], pitch) {
            return false;
        }

        // yahaha: the channel's cutoff and resonance (CC74, CC71) glide in.
        let moved = self.follow_channel_filter(channel_info);
        if self.dynamic_cutoff {
            let cents = self.mod_lfo_to_cutoff as f32 * self.mod_lfo.get_value()
                + self.mod_env_to_cutoff as f32 * self.mod_env.get_value();
            let factor = SoundFontMath::cents_to_multiplying_factor(cents);
            let new_cutoff = self.channel_cutoff_of(factor * self.cutoff);

            // The cutoff change is limited within x0.5 and x2 to reduce pop noise.
            let lower_limit = 0.5_f32 * self.smoothed_cutoff;
            let upper_limit = 2_f32 * self.smoothed_cutoff;
            self.smoothed_cutoff = SoundFontMath::clamp(new_cutoff, lower_limit, upper_limit);

            self.filter
                .set_low_pass_filter(self.smoothed_cutoff, self.live_resonance);
        } else if moved {
            self.smoothed_cutoff = self.channel_cutoff_of(self.cutoff);
            self.filter
                .set_low_pass_filter(self.smoothed_cutoff, self.live_resonance);
        }
        self.filter.process(&mut self.block[..]);

        self.previous_mix_gain_left = self.current_mix_gain_left;
        self.previous_mix_gain_right = self.current_mix_gain_right;
        self.previous_reverb_send = self.current_reverb_send;
        self.previous_chorus_send = self.current_chorus_send;

        // According to the GM spec, the following value should be squared.
        let ve = channel_info.get_volume() * channel_info.get_expression();
        let channel_gain = ve * ve;

        let mut mix_gain = self.note_gain * channel_gain * self.vol_env.get_value();
        if self.dynamic_volume {
            let decibels = self.mod_lfo_to_volume * self.mod_lfo.get_value();
            mix_gain *= SoundFontMath::decibels_to_linear(decibels);
        }

        let angle =
            (consts::PI / 200_f32) * (channel_info.get_pan() + self.instrument_pan + 50_f32);
        if angle <= 0_f32 {
            self.current_mix_gain_left = mix_gain;
            self.current_mix_gain_right = 0_f32;
        } else if angle >= SoundFontMath::HALF_PI {
            self.current_mix_gain_left = 0_f32;
            self.current_mix_gain_right = mix_gain;
        } else {
            self.current_mix_gain_left = mix_gain * angle.cos();
            self.current_mix_gain_right = mix_gain * angle.sin();
        }

        // yahaha: scaled by the note's own sends (`NoteParams::sends`, bus 0 reverb, bus 1
        // chorus), for the synthesizer's own effects.
        self.current_reverb_send = SoundFontMath::clamp(
            channel_info.get_reverb_send() + self.instrument_reverb,
            0_f32,
            1_f32,
        ) * self.note_sends[0];
        self.current_chorus_send = SoundFontMath::clamp(
            channel_info.get_chorus_send() + self.instrument_chorus,
            0_f32,
            1_f32,
        ) * self.note_sends[1];

        if self.voice_length == 0 {
            self.previous_mix_gain_left = self.current_mix_gain_left;
            self.previous_mix_gain_right = self.current_mix_gain_right;
            self.previous_reverb_send = self.current_reverb_send;
            self.previous_chorus_send = self.current_chorus_send;
        }

        self.voice_length += self.block.len();

        true
    }

    /// yahaha: `cutoff` scaled by the channel's cutoff factor as this voice has it (within
    /// the spec's range).
    fn channel_cutoff_of(&self, cutoff: f32) -> f32 {
        if self.channel_cutoff == 1_f32 {
            cutoff
        } else {
            SoundFontMath::clamp(cutoff * self.channel_cutoff, 19.45_f32, 20000_f32)
        }
    }

    /// yahaha: the filter's resonance with the channel's offset as this voice has it.
    fn channel_resonance(&self) -> f32 {
        if self.channel_resonance_db == 0_f32 {
            self.resonance
        } else {
            let db = SoundFontMath::clamp(self.resonance_db + self.channel_resonance_db, 0_f32, 96_f32);
            SoundFontMath::decibels_to_linear(db)
        }
    }

    /// yahaha: move this voice's cutoff factor and resonance offset a step towards the
    /// channel's (at most a quarter octave and 1 dB a block, so a jump doesn't click);
    /// true if they moved.
    fn follow_channel_filter(&mut self, channel_info: &Channel) -> bool {
        let (cutoff, db) = (channel_info.get_cutoff_factor(), channel_info.get_resonance_db());
        if cutoff == self.channel_cutoff && db == self.channel_resonance_db {
            return false;
        }
        const STEP: f32 = 1.189_207_1_f32; // 2^(1/4)
        self.channel_cutoff = if cutoff > self.channel_cutoff * STEP {
            self.channel_cutoff * STEP
        } else if cutoff < self.channel_cutoff / STEP {
            self.channel_cutoff / STEP
        } else {
            cutoff
        };
        self.channel_resonance_db = if (db - self.channel_resonance_db).abs() > 1_f32 {
            self.channel_resonance_db + (db - self.channel_resonance_db).signum()
        } else {
            db
        };
        self.live_resonance = self.channel_resonance();
        true
    }

    fn release_if_necessary(&mut self, channel_info: &Channel) {
        if self.voice_length < self.min_voice_length {
            return;
        }

        if self.voice_state == VoiceState::ReleaseRequested
            && (!channel_info.get_hold_pedal() || self.forced_end)
        {
            self.vol_env.release();
            self.mod_env.release();
            self.oscillator.release();

            self.voice_state = VoiceState::Released;
        }
    }

    pub(crate) fn block(&self) -> &Vec<f32> {
        &self.block
    }

    pub(crate) fn voice_length(&self) -> usize {
        self.voice_length
    }

    pub(crate) fn exclusive_class(&self) -> i32 {
        self.exclusive_class
    }

    pub(crate) fn channel(&self) -> i32 {
        self.channel
    }

    pub(crate) fn key(&self) -> i32 {
        self.key
    }

    pub(crate) fn priority(&self) -> f32 {
        if self.note_gain < SoundFontMath::NON_AUDIBLE {
            0_f32
        } else {
            self.vol_env.get_priority()
        }
    }
}
