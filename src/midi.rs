use crate::Error;

pub trait InactiveMidiSource {
    type ActiveType: ActiveMidiSource;
    fn activate(self) -> Result<Self::ActiveType, Error>;
}

pub trait ActiveMidiSource: Iterator<Item = MidiMessage> {
    type InactiveType;
    fn deactivate(self) -> Result<Self::InactiveType, Error>;
    fn cur_time(&self) -> u64;
}

#[derive(Debug)]
pub struct MidiMessage {
    pub time: u64,
    pub event: MidiEvent,
}

#[derive(Debug)]
pub enum MidiEvent {
    NoteOn(u8, u8),
    NoteOff(u8, u8),
    PlyPrs(u8, u8),
    CtrlChg(u8, u8),
    ProgChg(u8),
    ChnlPrs(u8),
    PitchWheel(u16),
    RealTime(u8),
    Unrecongized,
}
