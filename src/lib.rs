#![allow(dead_code)]
#![deny(missing_docs)]
#![deny(warnings)]
#![no_std]

//! Driver for the DFPlayer using the embedded-hal traits.

use embedded_hal::serial::{Read, Write};
use nb::block;
use num_enum::{IntoPrimitive, TryFromPrimitive};

/// Typealias for a message that is send and recived
type Message = [u8; 10];

/// Constants
const MSG_START: u8 = 0x7e;
const MSG_END: u8 = 0xef;

/// Error used in this crate
#[derive(Debug)]
pub enum Error<TXE, RXE> {
    /// Serial Write Error
    WriteError(TXE),
    /// Serial Read Error
    ReadError(nb::Error<RXE>),
    /// Message not complete
    MessageNotComplete,
    /// Recived more than 8 chars after start byte
    MessageOverrun,
}

/// The DFPlayer Driver
pub struct DFPlayer<TX, RX> {
    rx: RX,
    tx: TX,
    rx_message: Message,
    rx_counter: u8,
}

impl<TX, RX> DFPlayer<TX, RX>
where
    RX: Read<u8>,
    TX: Write<u8>,
{
    /// Creates a new Driver from a serial device
    pub fn new(tx: TX, rx: RX) -> Self {
        Self {
            rx,
            tx,
            rx_message: [0; 10],
            rx_counter: 0,
        }
    }

    /// Pause Playing a track
    pub fn pause(&mut self) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::Pause.into())
    }

    /// Start Plaing a track
    pub fn play(&mut self) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::Playback.into())
    }

    /// Next Track
    pub fn next_track(&mut self) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::Next.into())
    }

    /// Next Track
    pub fn previous_track(&mut self) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::Previous.into())
    }

    /// Increse Volume by one
    pub fn increse_volume(&mut self) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::IncreseVolume.into())
    }

    /// Increse Volume by one
    pub fn decrese_volume(&mut self) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::DecreseVolume.into())
    }

    /// Set the volume to specific value (0-30)
    /// Volume is limited to 0-30
    pub fn set_volume(&mut self, vol: u8) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::SpecifyVolume(vol.max(0).min(30)).into())
    }

    /// Set DFPlayer to standby to reduce power consumption.
    pub fn standby(&mut self) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::Standby.into())
    }

    /// Resets the DFPlayer
    pub fn reset_module(&mut self) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::ResetModule.into())
    }

    /// Wakes DFPlayer from standby
    pub fn wakeup(&mut self) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::NormalWorking.into())
    }

    /// Sets the equilizer
    pub fn set_equilizer(&mut self, eq: Equalizer) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::SpecifyEqualizer(eq).into())
    }

    /// Sets the playback mode
    pub fn set_playback_mode(
        &mut self,
        mode: PlaybackMode,
    ) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::SpecifyPlaybackMode(mode).into())
    }

    /// Play a track from mp3 folder
    pub fn play_mp3(&mut self, track: u16) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::SpecifyMp3Track(track.min(9999).max(0)).into())
    }

    /// Play a track from a folder. Folder is limited from 0-99 and track from 0-9999
    pub fn play_folder_track(
        &mut self,
        folder: u8,
        track: u8,
    ) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::SpecifyFolder(folder.min(99).max(0), track).into())
    }

    /// Pause Plaing, play advertisement, resume playing.
    pub fn advertise(&mut self, ad: u16) -> Result<(), Error<TX::Error, RX::Error>> {
        self.send_message(Command::SpecifyAdvertisement(ad.min(9999).max(0)).into())
    }

    /// Recive a message from dfplayer. Can be called cyclic or in an interrupt. Reads until 10 bytes arrive or timeout occures
    pub fn get_message(&mut self) {}

    /// Send a message
    fn send_message(&mut self, msg: Message) -> Result<(), Error<TX::Error, RX::Error>> {
        for byte in msg.iter() {
            if let Err(err) = block!(self.tx.write(*byte)) {
                return Err(Error::WriteError(err));
            }
        }
        Ok(())
    }

    /// Read a message
    pub fn read_message(&mut self) -> Result<Message, Error<TX::Error, RX::Error>> {
        match self.rx.read() {
            Ok(MSG_START) => {
                self.rx_counter = 1;
                self.rx_message = [0x00; 10];
                self.rx_message[0] = MSG_START;
            }
            Ok(MSG_END) => {
                if self.rx_counter < 10 {
                    self.rx_message[self.rx_counter as usize] = MSG_END;
                    self.rx_counter += 1;
                    return Ok(self.rx_message);
                } else {
                    return Err(Error::MessageOverrun);
                }
            }
            Ok(byte) => {
                if self.rx_counter < 10 {
                    self.rx_message[self.rx_counter as usize] = byte;
                    self.rx_counter += 1;
                }
            }
            Err(err) => return Err(Error::ReadError(err)),
        };

        Err(Error::MessageNotComplete)
    }
}

/// Representing a message to the tag
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Command {
    Next,
    Previous,
    /// Set a track (0-2999)
    SpecifyTrack(u16),
    IncreseVolume,
    DecreseVolume,
    /// Set Volume (0-30)
    SpecifyVolume(u8),
    SpecifyEqualizer(Equalizer),
    SpecifyPlaybackMode(PlaybackMode),
    Standby,
    NormalWorking,
    ResetModule,
    Playback,
    Pause,
    /// Specify a folder for playback (0-99)
    SpecifyFolder(u8, u8),
    /// Specify a track in mp3 folder (0-9999)
    SpecifyMp3Track(u16),
    /// Specify a track in advertisement folder (0-9999)
    SpecifyAdvertisement(u16),
    StopAdvertisement,
    Stop,
}

impl core::convert::From<Command> for Message {
    fn from(command: Command) -> Message {
        let mut msg = [0; 10];

        // Create static elements
        add_static_bytes(&mut msg);

        msg[3] = match command {
            Command::Next => 0x01,
            Command::Previous => 0x02,
            Command::SpecifyTrack(_) => 0x03,
            Command::IncreseVolume => 0x04,
            Command::DecreseVolume => 0x05,
            Command::SpecifyVolume(_) => 0x06,
            Command::SpecifyEqualizer(_) => 0x07,
            Command::SpecifyPlaybackMode(_) => 0x08,
            Command::Standby => 0x0A,
            Command::NormalWorking => 0x0B,
            Command::ResetModule => 0x0C,
            Command::Playback => 0x0D,
            Command::Pause => 0x0E,
            Command::SpecifyFolder(_, _) => 0x0F,
            Command::SpecifyMp3Track(_) => 0x12,
            Command::SpecifyAdvertisement(_) => 0x13,
            Command::StopAdvertisement => 0x15,
            Command::Stop => 0x16,
        };

        msg[4] = 0x00; // Is a command --> We want no feedback
        let data = match command {
            Command::SpecifyTrack(track) => track.to_be_bytes(),
            Command::SpecifyVolume(vol) => [0x00, vol],
            Command::SpecifyEqualizer(equ) => [0x00, equ.into()],
            Command::SpecifyPlaybackMode(mode) => [0x00, mode.into()],
            Command::SpecifyFolder(folder, track) => [folder, track],
            Command::SpecifyMp3Track(track) => track.to_be_bytes(),
            Command::SpecifyAdvertisement(track) => track.to_be_bytes(),
            _ => [0x00, 0x00],
        };
        msg[5] = data[0];
        msg[6] = data[1];

        add_checksum(&mut msg);

        msg
    }
}
enum Querry {
    Status,
    Volume,
    Equalizer,
    PlaybackMode,
    SoftwareVersion,
    FileCountInFolder,
    FolderCount,
}

impl core::convert::From<Querry> for Message {
    fn from(querry: Querry) -> Self {
        let mut msg = [0; 10];

        // Create static elements
        add_static_bytes(&mut msg);

        msg[3] = match querry {
            Querry::Volume => 0x01,
            _ => unimplemented!(),
        };

        msg[4] = 0x00; // Is a command --> We want no feedback

        msg[5] = 0x00;
        msg[6] = 0x00;

        add_checksum(&mut msg);

        msg
    }
}

/// Calculate the checksum
fn add_checksum(msg: &mut [u8]) {
    let mut sum: u16 = 0;
    for &byte in msg[1..7].iter() {
        sum += byte as u16;
    }

    let checksum = (0_u16.wrapping_sub(sum)).to_be_bytes();
    msg[7..9].copy_from_slice(&checksum);
}

/// Adds static bytes to message
fn add_static_bytes(msg: &mut Message) {
    msg[0] = 0x7e; // Start Byte
    msg[1] = 0xff; // Version
    msg[2] = 0x06; // Length after this byte.... always 6...

    msg[9] = 0xEF; // End Byte
}

/// Specify Playback mode for DFPlayer
#[derive(IntoPrimitive, TryFromPrimitive, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum PlaybackMode {
    /// Repeat the curren track
    Repeat = 0x00,
    /// Repeat all tracks in the folder
    FolderRepeat = 0x01,
    /// Repeat the curren track???
    SingleRepeat = 0x02,
    /// Random Track in folder???
    Random = 0x03,
}

/// Enum describes possible Values for the equalizer
#[allow(missing_docs)]
#[derive(IntoPrimitive, TryFromPrimitive, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
pub enum Equalizer {
    Normal = 0x00,
    Pop = 0x01,
    Rock = 0x02,
    Jazz = 0x03,
    Classic = 0x04,
    Bass = 0x05,
}

/// Enum discribes possible states of the dfplayer
#[derive(Debug, Copy, Clone, Eq, PartialEq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum State {
    /// Player is playing a track
    Busy = 0x01,
    /// Player is in sleep mode
    Sleeping = 0x02,
    /// Something is wring with the serial interface
    SerialWrongStack = 0x03,
    /// The checksum in the message is not ok
    CheckSumNotMatch = 0x04,
    /// Something is wrong with the fileindex
    FileIndexOut = 0x05,
    /// Something is wrong with the File
    FileMismatch = 0x06,
    /// Playing an advertisement
    Advertise = 0x07,
}

/// Devices that can be used for playback
#[derive(TryFromPrimitive, IntoPrimitive, Debug, Copy, Clone, Eq, PartialEq)]
#[repr(u8)]
#[allow(missing_docs)]
pub enum Device {
    UDisk = 0x01,
    SD = 0x02,
    Aux = 0x03,
    Sleep = 0x04,
    Flash = 0x05,
}
