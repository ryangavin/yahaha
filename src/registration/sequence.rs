//! The Registration Sequence (RM p.114-115): a programmed order of the bank's ten
//! buttons, stepped with Regist +/- (pedals, Assignable buttons, DEC/INC on Home; here the
//! Launchkey pads, the terminal keys and the app).

use serde::{Deserialize, Serialize};

/// What happens when the sequence is advanced past its last step.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SequenceEnd {
    /// Advancing does nothing more.
    #[default]
    Stop,
    /// Start again at the first step.
    Top,
    /// Go to the first step of the next bank file in the same folder.
    Next,
}

/// A bank's Registration Sequence: saved in the bank file.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    /// The sequence is in use (Regist +/- step it).
    #[serde(default)]
    pub on: bool,
    /// Button indices (0-9) in order; a button may appear more than once.
    #[serde(default)]
    pub steps: Vec<u8>,
    #[serde(default)]
    pub end: SequenceEnd,
}

/// Where a step of the sequence goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SeqMove {
    /// Recall step `pos` (its button is `steps[pos]`).
    Step(usize),
    /// Nothing to do (the sequence is empty, or stopped at its end).
    Stay,
    /// Load the next bank in the folder and recall its first step (End = Next, advancing).
    NextBank,
    /// Load the previous bank and recall its last step (End = Next, going back).
    PrevBank,
}

/// The most steps a sequence keeps (the Genos display holds a long list; this is a sanity
/// bound on files, not a performance limit).
pub const MAX_STEPS: usize = 128;

impl Sequence {
    /// Where Regist + (`delta` > 0) or Regist - (`delta` < 0) goes from `pos` (None: not
    /// started, so + goes to the first step and - to the last).
    ///
    /// Past the end: `Stop` stays, `Top` wraps to the first step, `Next` asks for the next
    /// bank. Before the start (Regist - on the first step), which the manual does not
    /// describe: `Stop` stays, `Top` wraps to the last step, `Next` asks for the previous
    /// bank (docs/registration.md).
    pub fn step(&self, pos: Option<usize>, delta: i8) -> SeqMove {
        let n = self.steps.len();
        if n == 0 || delta == 0 {
            return SeqMove::Stay;
        }
        let pos = pos.filter(|&p| p < n);
        if delta > 0 {
            match pos {
                None => SeqMove::Step(0),
                Some(p) if p + 1 < n => SeqMove::Step(p + 1),
                Some(_) => match self.end {
                    SequenceEnd::Stop => SeqMove::Stay,
                    SequenceEnd::Top => SeqMove::Step(0),
                    SequenceEnd::Next => SeqMove::NextBank,
                },
            }
        } else {
            match pos {
                None => SeqMove::Step(n - 1),
                Some(p) if p > 0 => SeqMove::Step(p - 1),
                Some(_) => match self.end {
                    SequenceEnd::Stop => SeqMove::Stay,
                    SequenceEnd::Top => SeqMove::Step(n - 1),
                    SequenceEnd::Next => SeqMove::PrevBank,
                },
            }
        }
    }

    /// The sequence position after button `button` was recalled by hand: its next
    /// occurrence from `pos` on (wrapping), so Regist + carries on from there; unchanged if
    /// the button is not in the sequence.
    pub fn follow(&self, pos: Option<usize>, button: u8) -> Option<usize> {
        let n = self.steps.len();
        if n == 0 {
            return pos;
        }
        if pos.is_some_and(|p| self.steps.get(p) == Some(&button)) {
            return pos;
        }
        let start = pos.map_or(0, |p| p + 1);
        (0..n).map(|i| (start + i) % n).find(|&i| self.steps[i] == button).or(pos)
    }

    /// Keep only valid steps (buttons 0-9, at most `MAX_STEPS`).
    pub fn clean(mut self) -> Sequence {
        self.steps.retain(|&b| (b as usize) < super::BUTTONS);
        self.steps.truncate(MAX_STEPS);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seq(steps: &[u8], end: SequenceEnd) -> Sequence {
        Sequence { on: true, steps: steps.to_vec(), end }
    }

    #[test]
    fn advancing_and_the_end_actions() {
        let s = seq(&[2, 0, 5], SequenceEnd::Stop);
        assert_eq!(s.step(None, 1), SeqMove::Step(0));
        assert_eq!(s.step(Some(0), 1), SeqMove::Step(1));
        assert_eq!(s.step(Some(2), 1), SeqMove::Stay);
        assert_eq!(seq(&[2, 0, 5], SequenceEnd::Top).step(Some(2), 1), SeqMove::Step(0));
        assert_eq!(seq(&[2, 0, 5], SequenceEnd::Next).step(Some(2), 1), SeqMove::NextBank);
    }

    #[test]
    fn going_back() {
        let s = seq(&[2, 0, 5], SequenceEnd::Stop);
        assert_eq!(s.step(None, -1), SeqMove::Step(2));
        assert_eq!(s.step(Some(1), -1), SeqMove::Step(0));
        assert_eq!(s.step(Some(0), -1), SeqMove::Stay);
        assert_eq!(seq(&[2, 0, 5], SequenceEnd::Top).step(Some(0), -1), SeqMove::Step(2));
        assert_eq!(seq(&[2, 0, 5], SequenceEnd::Next).step(Some(0), -1), SeqMove::PrevBank);
    }

    #[test]
    fn empty_sequence_and_stale_positions() {
        assert_eq!(seq(&[], SequenceEnd::Top).step(None, 1), SeqMove::Stay);
        // A position past the end (the sequence was shortened) counts as not started.
        assert_eq!(seq(&[1], SequenceEnd::Stop).step(Some(7), 1), SeqMove::Step(0));
    }

    #[test]
    fn a_manual_recall_moves_the_cursor_to_that_button() {
        let s = seq(&[2, 0, 5, 0], SequenceEnd::Stop);
        assert_eq!(s.follow(None, 0), Some(1));
        assert_eq!(s.follow(Some(1), 0), Some(1));
        assert_eq!(s.follow(Some(2), 0), Some(3));
        assert_eq!(s.follow(Some(3), 2), Some(0));
        assert_eq!(s.follow(Some(2), 9), Some(2));
    }

    #[test]
    fn clean_drops_bad_steps() {
        let s = Sequence { on: false, steps: vec![1, 10, 9, 200], end: SequenceEnd::Stop }.clean();
        assert_eq!(s.steps, [1, 9]);
    }
}
