use crate::synthesizer::SEND_BUSES;

/// yahaha: per-note settings a voice takes at note-on (`Synthesizer::note_on_with`), for an
/// XG Drum Setup: each drum note of a kit tuned on its own. They are fixed for the voice's
/// life, so a later note-on with other settings never moves a voice already sounding.
/// [`NoteParams::NEUTRAL`] plays exactly as `note_on`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NoteParams {
    /// Gain (linear) on the voice.
    pub gain: f32,
    /// Semitones added to the voice's pitch (whatever the region's scale tuning).
    pub tune: f32,
    /// The voice's pan (SoundFont units, -50 left .. 50 right) in place of the region's;
    /// `None` keeps the region's.
    pub pan: Option<f32>,
    /// Each send bus's gain (`set_channel_sends`) scaled for this voice; with the
    /// synthesizer's own effects, bus 0 scales its reverb send and bus 1 its chorus send.
    pub sends: [f32; SEND_BUSES],
    /// Filter cutoff scaled by this factor.
    pub cutoff: f32,
    /// Decibels added to the filter's resonance.
    pub resonance_db: f32,
    /// The volume envelope's attack, decay and release times scaled by these factors.
    pub attack: f32,
    pub decay: f32,
    pub release: f32,
}

impl NoteParams {
    /// A note as `note_on` plays it.
    pub const NEUTRAL: NoteParams = NoteParams {
        gain: 1_f32,
        tune: 0_f32,
        pan: None,
        sends: [1_f32; SEND_BUSES],
        cutoff: 1_f32,
        resonance_db: 0_f32,
        attack: 1_f32,
        decay: 1_f32,
        release: 1_f32,
    };
}

impl Default for NoteParams {
    fn default() -> Self {
        NoteParams::NEUTRAL
    }
}
