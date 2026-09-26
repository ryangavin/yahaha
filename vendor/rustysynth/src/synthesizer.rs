#![allow(dead_code)]

use std::cmp;
use std::collections::HashMap;
use std::sync::Arc;

use crate::array_math::ArrayMath;
use crate::channel::Channel;
use crate::chorus::Chorus;
use crate::error::SynthesizerError;
use crate::region_pair::RegionPair;
use crate::reverb::Reverb;
use crate::soundfont::SoundFont;
use crate::soundfont_math::SoundFontMath;
use crate::synthesizer_settings::SynthesizerSettings;
use crate::voice_collection::VoiceCollection;

/// An instance of the SoundFont synthesizer.
#[derive(Debug)]
#[non_exhaustive]
pub struct Synthesizer {
    pub(crate) sound_font: Arc<SoundFont>,
    pub(crate) sample_rate: i32,
    pub(crate) block_size: usize,
    pub(crate) maximum_polyphony: usize,

    preset_lookup: HashMap<i32, usize>,
    default_preset: usize,

    channels: Vec<Channel>,

    voices: VoiceCollection,

    block_left: Vec<f32>,
    block_right: Vec<f32>,

    inverse_block_size: f32,

    block_read: usize,

    master_volume: f32,

    effects: Option<Effects>,

    // yahaha: each channel's dry mix of the block (channel c at c * block_size), for
    // `channel_peaks`.
    channel_left: Vec<f32>,
    channel_right: Vec<f32>,
    channel_peaks: [f32; 16],
    // yahaha: the send buses (yahaha's shared effects): each channel's gain into each bus
    // (`set_channel_sends`), and the block's buses (bus b, side s at (2 * b + s) *
    // block_size), filled from the channel rows above. `internal_effects` turns the
    // synthesizer's own reverb and chorus on or off.
    send_gains: [[f32; SEND_BUSES]; 16],
    send_block: Vec<f32>,
    send_used: bool,
    used_rows: u16,
    internal_effects: bool,
}

/// yahaha: the number of send buses `render_with_sends` fills.
pub const SEND_BUSES: usize = 3;

impl Synthesizer {
    /// The number of channels.
    pub const CHANNEL_COUNT: usize = 16;
    /// The percussion channel.
    pub const PERCUSSION_CHANNEL: usize = 9;

    /// Initializes a new synthesizer using a specified SoundFont and settings.
    ///
    /// # Arguments
    ///
    /// * `sound_font` - The SoundFont instance.
    /// * `settings` - The settings for synthesis.
    pub fn new(
        sound_font: &Arc<SoundFont>,
        settings: &SynthesizerSettings,
    ) -> Result<Self, SynthesizerError> {
        settings.validate()?;

        let mut preset_lookup: HashMap<i32, usize> = HashMap::new();

        let mut min_preset_id = i32::MAX;
        let mut default_preset: usize = 0;
        for i in 0..sound_font.presets.len() {
            let preset = &sound_font.presets[i];

            // The preset ID is Int32, where the upper 16 bits represent the bank number
            // and the lower 16 bits represent the patch number.
            // This ID is used to search for presets by the combination of bank number
            // and patch number.
            let preset_id = (preset.bank_number << 16) | preset.patch_number;
            preset_lookup.insert(preset_id, i);

            // The preset with the minimum ID number will be default.
            // If the SoundFont is GM compatible, the piano will be chosen.
            if preset_id < min_preset_id {
                default_preset = i;
                min_preset_id = preset_id;
            }
        }

        let mut channels: Vec<Channel> = Vec::new();
        for i in 0..Synthesizer::CHANNEL_COUNT {
            channels.push(Channel::new(i == Synthesizer::PERCUSSION_CHANNEL));
        }

        let voices = VoiceCollection::new(settings);

        let block_left: Vec<f32> = vec![0_f32; settings.block_size];
        let block_right: Vec<f32> = vec![0_f32; settings.block_size];

        let inverse_block_size = 1_f32 / settings.block_size as f32;

        let block_read = settings.block_size;

        let master_volume = 0.5_f32;

        let effects = if settings.enable_reverb_and_chorus {
            Some(Effects::new(settings))
        } else {
            None
        };

        let channel_left: Vec<f32> = vec![0_f32; Synthesizer::CHANNEL_COUNT * settings.block_size];
        let channel_right: Vec<f32> = vec![0_f32; Synthesizer::CHANNEL_COUNT * settings.block_size];

        Ok(Self {
            sound_font: Arc::clone(sound_font),
            sample_rate: settings.sample_rate,
            block_size: settings.block_size,
            maximum_polyphony: settings.maximum_polyphony,
            preset_lookup,
            default_preset,
            channels,
            voices,
            block_left,
            block_right,
            inverse_block_size,
            block_read,
            master_volume,
            effects,
            channel_left,
            channel_right,
            channel_peaks: [0_f32; Synthesizer::CHANNEL_COUNT],
            send_gains: [[0_f32; SEND_BUSES]; Synthesizer::CHANNEL_COUNT],
            send_block: vec![0_f32; 2 * SEND_BUSES * settings.block_size],
            send_used: false,
            used_rows: 0,
            internal_effects: true,
        })
    }

    /// yahaha: each MIDI channel's peak (linear, both sides) in what has been rendered
    /// since the last `reset_channel_peaks`: its voices' dry mix after the channel's
    /// volume, expression and pan and the master volume, before reverb and chorus (which
    /// the channels share).
    pub fn channel_peaks(&self) -> &[f32; 16] {
        &self.channel_peaks
    }

    /// yahaha: start the channel peaks again from 0.
    pub fn reset_channel_peaks(&mut self) {
        self.channel_peaks = [0_f32; Synthesizer::CHANNEL_COUNT];
    }

    /// yahaha: channel `channel`'s gain (linear) into each send bus of
    /// `render_with_sends`. The send taps the channel's dry mix (after its volume,
    /// expression and pan and the master volume). Takes effect from the next block.
    pub fn set_channel_sends(&mut self, channel: usize, gains: [f32; SEND_BUSES]) {
        self.send_gains[channel & (Synthesizer::CHANNEL_COUNT - 1)] = gains;
    }

    /// yahaha: whether the synthesizer's own reverb and chorus play (they do by default,
    /// if the settings enabled them). Turning them back on starts them from silence.
    pub fn set_internal_effects(&mut self, on: bool) {
        if on && !self.internal_effects {
            if let Some(effects) = self.effects.as_mut() {
                effects.reverb.mute();
                effects.chorus.mute();
            }
        }
        self.internal_effects = on;
    }

    /// Processes a MIDI message.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to which the message will be sent.
    /// * `command` - The type of the message.
    /// * `data1` - The first data part of the message.
    /// * `data2` - The second data part of the message.
    pub fn process_midi_message(&mut self, channel: i32, command: i32, data1: i32, data2: i32) {
        if !(0 <= channel && channel < self.channels.len() as i32) {
            return;
        }

        let channel_info = &mut self.channels[channel as usize];

        match command {
            0x80 => self.note_off(channel, data1),       // Note Off
            0x90 => self.note_on(channel, data1, data2), // Note On
            0xB0 => match data1 // Controller
            {
                0x00 => channel_info.set_bank(data2), // Bank Selection
                0x01 => channel_info.set_modulation_coarse(data2), // Modulation Coarse
                0x21 => channel_info.set_modulation_fine(data2), // Modulation Fine
                0x06 => channel_info.data_entry_coarse(data2), // Data Entry Coarse
                0x26 => channel_info.data_entry_fine(data2), // Data Entry Fine
                0x07 => channel_info.set_volume_coarse(data2), // Channel Volume Coarse
                0x27 => channel_info.set_volume_fine(data2), // Channel Volume Fine
                0x0A => channel_info.set_pan_coarse(data2), // Pan Coarse
                0x2A => channel_info.set_pan_fine(data2), // Pan Fine
                0x0B => channel_info.set_expression_coarse(data2), // Expression Coarse
                0x2B => channel_info.set_expression_fine(data2), // Expression Fine
                0x40 => channel_info.set_hold_pedal(data2), // Hold Pedal
                0x5B => channel_info.set_reverb_send(data2), // Reverb Send
                0x5D => channel_info.set_chorus_send(data2), // Chorus Send
                0x63 => channel_info.set_nrpn_coarse(data2), // NRPN Coarse
                0x62 => channel_info.set_nrpn_fine(data2), // NRPN Fine
                0x65 => channel_info.set_rpn_coarse(data2), // RPN Coarse
                0x64 => channel_info.set_rpn_fine(data2), // RPN Fine
                0x78 => self.note_off_all_channel(channel, true), // All Sound Off
                0x79 => self.reset_all_controllers_channel(channel), // Reset All Controllers
                0x7B => self.note_off_all_channel(channel, false), // All Note Off
                _ => (),
            },
            0xC0 => channel_info.set_patch(data1), // Program Change
            0xE0 => channel_info.set_pitch_bend(data1, data2), // Pitch Bend
            _ => (),
        }
    }

    /// Stops a note.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel of the note.
    /// * `key` - The key of the note.
    pub fn note_off(&mut self, channel: i32, key: i32) {
        if !(0 <= channel && channel < self.channels.len() as i32) {
            return;
        }

        for voice in self.voices.get_active_voices().iter_mut() {
            if voice.channel() == channel && voice.key() == key {
                voice.end();
            }
        }
    }

    /// Starts a note.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel of the note.
    /// * `key` - The key of the note.
    /// * `velocity` - The velocity of the note.
    pub fn note_on(&mut self, channel: i32, key: i32, velocity: i32) {
        if velocity == 0 {
            self.note_off(channel, key);
            return;
        }

        if !(0 <= channel && channel < self.channels.len() as i32) {
            return;
        }

        let channel_info = &self.channels[channel as usize];

        let preset_id = (channel_info.get_bank_number() << 16) | channel_info.get_patch_number();

        let mut preset = self.default_preset;
        match self.preset_lookup.get(&preset_id) {
            Some(value) => preset = *value,
            None => {
                // Try fallback to the GM sound set.
                // Normally, the given patch number + the bank number 0 will work.
                // For drums (bank number >= 128), it seems to be better to select the standard set (128:0).
                let gm_preset_id = if channel_info.get_bank_number() < 128 {
                    channel_info.get_patch_number()
                } else {
                    128 << 16
                };

                // If no corresponding preset was found. Use the default one...
                if let Some(value) = self.preset_lookup.get(&gm_preset_id) {
                    preset = *value
                }
            }
        }

        let preset = &self.sound_font.presets[preset];
        for preset_region in preset.regions.iter() {
            if preset_region.contains(key, velocity) {
                let instrument = &self.sound_font.instruments[preset_region.instrument];
                for instrument_region in instrument.regions.iter() {
                    if instrument_region.contains(key, velocity) {
                        let region_pair = RegionPair::new(preset_region, instrument_region);

                        if let Some(value) = self.voices.request_new(instrument_region, channel) {
                            value.start(&region_pair, channel, key, velocity)
                        }
                    }
                }
            }
        }
    }

    /// Stops all the notes in the specified channel.
    ///
    /// # Arguments
    ///
    /// * `immediate` - If `true`, notes will stop immediately without the release sound.
    pub fn note_off_all(&mut self, immediate: bool) {
        if immediate {
            self.voices.clear();
        } else {
            for voice in self.voices.get_active_voices().iter_mut() {
                voice.end();
            }
        }
    }

    /// Stops all the notes in the specified channel.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel in which the notes will be stopped.
    /// * `immediate` - If `true`, notes will stop immediately without the release sound.
    pub fn note_off_all_channel(&mut self, channel: i32, immediate: bool) {
        if immediate {
            for voice in self.voices.get_active_voices().iter_mut() {
                if voice.channel() == channel {
                    voice.kill();
                }
            }
        } else {
            for voice in self.voices.get_active_voices().iter_mut() {
                if voice.channel() == channel {
                    voice.end();
                }
            }
        }
    }

    /// Resets all the controllers.
    pub fn reset_all_controllers(&mut self) {
        for channel in &mut self.channels {
            channel.reset_all_controllers();
        }
    }

    /// Resets all the controllers of the specified channel.
    ///
    /// # Arguments
    ///
    /// * `channel` - The channel to be reset.
    pub fn reset_all_controllers_channel(&mut self, channel: i32) {
        if !(0 <= channel && channel < self.channels.len() as i32) {
            return;
        }

        self.channels[channel as usize].reset_all_controllers();
    }

    /// Resets the synthesizer.
    pub fn reset(&mut self) {
        self.voices.clear();

        for channel in &mut self.channels {
            channel.reset();
        }

        if let Some(effects) = self.effects.as_mut() {
            effects.reverb.mute();
            effects.chorus.mute();
        }

        self.block_read = self.block_size;
    }

    /// Renders the waveform.
    ///
    /// # Arguments
    ///
    /// * `left` - The buffer of the left channel to store the rendered waveform.
    /// * `right` - The buffer of the right channel to store the rendered waveform.
    ///
    /// # Remarks
    ///
    /// The output buffers for the left and right must be the same length.
    pub fn render(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.render_inner(left, right, None);
    }

    /// yahaha: `render`, and **add** the send buses to `sends`: bus b's left side at
    /// `sends[2 * b * n..]`, its right side at `sends[(2 * b + 1) * n..]`, `n` =
    /// `left.len()` (so `sends` holds at least `2 * SEND_BUSES * n`).
    pub fn render_with_sends(&mut self, left: &mut [f32], right: &mut [f32], sends: &mut [f32]) {
        self.render_inner(left, right, Some(sends));
    }

    fn render_inner(&mut self, left: &mut [f32], right: &mut [f32], mut sends: Option<&mut [f32]>) {
        if left.len() != right.len() {
            panic!("The output buffers for the left and right must be the same length.");
        }
        let n = left.len();
        if let Some(s) = sends.as_deref() {
            if s.len() < 2 * SEND_BUSES * n {
                panic!("The send buffer must hold every bus.");
            }
        }

        let left_length = left.len();

        let mut wrote = 0;
        while wrote < left_length {
            if self.block_read == self.block_size {
                self.render_block();
                self.block_read = 0;
            }

            let src_rem = self.block_size - self.block_read;
            let dst_rem = left_length - wrote;
            let rem = cmp::min(src_rem, dst_rem);

            for t in 0..rem {
                left[wrote + t] = self.block_left[self.block_read + t];
                right[wrote + t] = self.block_right[self.block_read + t];
            }

            // yahaha: the send buses, when the block has any.
            if let (Some(sends), true) = (sends.as_deref_mut(), self.send_used) {
                let bs = self.block_size;
                for row in 0..2 * SEND_BUSES {
                    let src = &self.send_block[row * bs + self.block_read..row * bs + self.block_read + rem];
                    let dst = &mut sends[row * n + wrote..row * n + wrote + rem];
                    for (d, x) in dst.iter_mut().zip(src) {
                        *d += *x;
                    }
                }
            }

            self.block_read += rem;
            wrote += rem;
        }
    }

    fn render_block(&mut self) {
        self.voices
            .process(&self.sound_font.wave_data, &self.channels);

        self.block_left.fill(0_f32);
        self.block_right.fill(0_f32);
        // yahaha: which channels have a voice in this block (their `channel_*` rows are
        // cleared on first use).
        let mut used: u16 = 0;
        let bs = self.block_size;
        for voice in self.voices.get_active_voices().iter_mut() {
            let previous_gain_left = self.master_volume * voice.previous_mix_gain_left;
            let current_gain_left = self.master_volume * voice.current_mix_gain_left;
            Synthesizer::write_block(
                previous_gain_left,
                current_gain_left,
                voice.block(),
                &mut self.block_left[..],
                self.inverse_block_size,
            );
            let previous_gain_right = self.master_volume * voice.previous_mix_gain_right;
            let current_gain_right = self.master_volume * voice.current_mix_gain_right;
            Synthesizer::write_block(
                previous_gain_right,
                current_gain_right,
                voice.block(),
                &mut self.block_right[..],
                self.inverse_block_size,
            );
            // yahaha: the same again into the voice's channel row, for its meter. The mix
            // above is untouched, so the output is exactly upstream's.
            let ch = (voice.channel() as usize) & (Synthesizer::CHANNEL_COUNT - 1);
            let row = ch * bs..(ch + 1) * bs;
            if used & (1 << ch) == 0 {
                used |= 1 << ch;
                self.channel_left[row.clone()].fill(0_f32);
                self.channel_right[row.clone()].fill(0_f32);
            }
            Synthesizer::write_block(
                previous_gain_left,
                current_gain_left,
                voice.block(),
                &mut self.channel_left[row.clone()],
                self.inverse_block_size,
            );
            Synthesizer::write_block(
                previous_gain_right,
                current_gain_right,
                voice.block(),
                &mut self.channel_right[row],
                self.inverse_block_size,
            );
        }
        self.used_rows = used;
        while used != 0 {
            let ch = used.trailing_zeros() as usize;
            used &= used - 1;
            let row = ch * bs..(ch + 1) * bs;
            let peak = self.channel_left[row.clone()]
                .iter()
                .chain(&self.channel_right[row])
                .fold(0_f32, |p, x| p.max(x.abs()));
            self.channel_peaks[ch] = self.channel_peaks[ch].max(peak);
        }

        // yahaha: the send buses, from the channel rows of the channels that sound.
        self.send_used = false;
        let mut sending = self.used_rows & self.send_mask();
        if sending != 0 {
            self.send_block.fill(0_f32);
            self.send_used = true;
        }
        while sending != 0 {
            let ch = sending.trailing_zeros() as usize;
            sending &= sending - 1;
            let row = ch * bs..(ch + 1) * bs;
            for (b, &g) in self.send_gains[ch].iter().enumerate() {
                if g <= 0_f32 {
                    continue;
                }
                let (l, r) = self.send_block.split_at_mut((2 * b + 1) * bs);
                ArrayMath::multiply_add(g, &self.channel_left[row.clone()], &mut l[2 * b * bs..]);
                ArrayMath::multiply_add(g, &self.channel_right[row.clone()], &mut r[..bs]);
            }
        }

        if !self.internal_effects {
            return;
        }
        if let Some(effects) = self.effects.as_mut() {
            let chorus = &mut effects.chorus;
            let chorus_input_left = &mut effects.chorus_input_left[..];
            let chorus_input_right = &mut effects.chorus_input_right[..];
            let chorus_output_left = &mut effects.chorus_output_left[..];
            let chorus_output_right = &mut effects.chorus_output_right[..];
            chorus_input_left.fill(0_f32);
            chorus_input_right.fill(0_f32);
            for voice in self.voices.get_active_voices().iter_mut() {
                let previous_gain_left = voice.previous_chorus_send * voice.previous_mix_gain_left;
                let current_gain_left = voice.current_chorus_send * voice.current_mix_gain_left;
                Synthesizer::write_block(
                    previous_gain_left,
                    current_gain_left,
                    voice.block(),
                    chorus_input_left,
                    self.inverse_block_size,
                );
                let previous_gain_right =
                    voice.previous_chorus_send * voice.previous_mix_gain_right;
                let current_gain_right = voice.current_chorus_send * voice.current_mix_gain_right;
                Synthesizer::write_block(
                    previous_gain_right,
                    current_gain_right,
                    voice.block(),
                    chorus_input_right,
                    self.inverse_block_size,
                );
            }
            chorus.process(
                chorus_input_left,
                chorus_input_right,
                chorus_output_left,
                chorus_output_right,
            );
            ArrayMath::multiply_add(
                self.master_volume,
                chorus_output_left,
                &mut self.block_left[..],
            );
            ArrayMath::multiply_add(
                self.master_volume,
                chorus_output_right,
                &mut self.block_right[..],
            );

            let reverb = &mut effects.reverb;
            let reverb_input = &mut effects.reverb_input[..];
            let reverb_output_left = &mut effects.reverb_output_left[..];
            let reverb_output_right = &mut effects.reverb_output_right[..];
            reverb_input.fill(0_f32);
            for voice in self.voices.get_active_voices().iter_mut() {
                let previous_gain = reverb.get_input_gain()
                    * voice.previous_reverb_send
                    * (voice.previous_mix_gain_left + voice.previous_mix_gain_right);
                let current_gain = reverb.get_input_gain()
                    * voice.current_reverb_send
                    * (voice.current_mix_gain_left + voice.current_mix_gain_right);
                Synthesizer::write_block(
                    previous_gain,
                    current_gain,
                    voice.block(),
                    &mut reverb_input[..],
                    self.inverse_block_size,
                );
            }

            reverb.process(reverb_input, reverb_output_left, reverb_output_right);
            ArrayMath::multiply_add(
                self.master_volume,
                reverb_output_left,
                &mut self.block_left[..],
            );
            ArrayMath::multiply_add(
                self.master_volume,
                reverb_output_right,
                &mut self.block_right[..],
            );
        }
    }

    /// yahaha: the channels with a send gain above 0 (bit = channel).
    fn send_mask(&self) -> u16 {
        let mut m = 0_u16;
        for (ch, g) in self.send_gains.iter().enumerate() {
            if g.iter().any(|&x| x > 0_f32) {
                m |= 1 << ch;
            }
        }
        m
    }

    fn write_block(
        previous_gain: f32,
        current_gain: f32,
        source: &[f32],
        destination: &mut [f32],
        inverse_block_size: f32,
    ) {
        if SoundFontMath::max(previous_gain, current_gain) < SoundFontMath::NON_AUDIBLE {
            return;
        }

        if (current_gain - previous_gain).abs() < 1.0E-3_f32 {
            ArrayMath::multiply_add(current_gain, source, destination);
        } else {
            let step = inverse_block_size * (current_gain - previous_gain);
            ArrayMath::multiply_add_slope(previous_gain, step, source, destination);
        }
    }

    /// Gets the SoundFont used as the audio source.
    pub fn get_sound_font(&self) -> &SoundFont {
        &self.sound_font
    }

    /// Gets the sample rate for synthesis.
    pub fn get_sample_rate(&self) -> i32 {
        self.sample_rate
    }

    /// Gets the block size for rendering waveform.
    pub fn get_block_size(&self) -> usize {
        self.block_size
    }

    /// Gets the number of maximum polyphony.
    pub fn get_maximum_polyphony(&self) -> usize {
        self.maximum_polyphony
    }

    /// Gets the value indicating whether reverb and chorus are enabled.
    pub fn get_enable_reverb_and_chorus(&self) -> bool {
        self.effects.is_some()
    }

    /// Gets the master volume.
    pub fn get_master_volume(&self) -> f32 {
        self.master_volume
    }

    /// Sets the master volume.
    ///
    /// # Arguments
    ///
    /// * `value` - The new value of the master volume.
    pub fn set_master_volume(&mut self, value: f32) {
        self.master_volume = value;
    }
}

#[derive(Debug)]
struct Effects {
    reverb: Reverb,
    reverb_input: Vec<f32>,
    reverb_output_left: Vec<f32>,
    reverb_output_right: Vec<f32>,

    chorus: Chorus,
    chorus_input_left: Vec<f32>,
    chorus_input_right: Vec<f32>,
    chorus_output_left: Vec<f32>,
    chorus_output_right: Vec<f32>,
}

impl Effects {
    fn new(settings: &SynthesizerSettings) -> Effects {
        Self {
            reverb: Reverb::new(settings.sample_rate),
            reverb_input: vec![0_f32; settings.block_size],
            reverb_output_left: vec![0_f32; settings.block_size],
            reverb_output_right: vec![0_f32; settings.block_size],
            chorus: Chorus::new(settings.sample_rate, 0.002, 0.0019, 0.4),
            chorus_input_left: vec![0_f32; settings.block_size],
            chorus_input_right: vec![0_f32; settings.block_size],
            chorus_output_left: vec![0_f32; settings.block_size],
            chorus_output_right: vec![0_f32; settings.block_size],
        }
    }
}
