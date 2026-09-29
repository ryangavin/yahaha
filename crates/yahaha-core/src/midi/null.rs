//! The no-op MIDI backend (Linux, agentic development only): the CoreMIDI wrapper's API
//! with no devices. Clients, ports and virtual endpoints are created and never carry
//! anything; input handlers are dropped unused.

use super::InputHandler;
use anyhow::Result;

pub type Endpoint = u32;
/// An output port.
pub type OutPort = u32;

pub struct Client {
    pub client: u32,
}

pub fn name_of(_obj: u32) -> String {
    String::from("?")
}

pub fn sources() -> Vec<(Endpoint, String)> {
    Vec::new()
}

/// The endpoint is online. There are none here.
pub fn is_online(_e: Endpoint) -> bool {
    false
}

/// The sources that are online.
pub fn online_sources() -> Vec<(Endpoint, String)> {
    Vec::new()
}

/// The destinations that are online.
pub fn online_destinations() -> Vec<(Endpoint, String)> {
    Vec::new()
}

pub fn destinations() -> Vec<(Endpoint, String)> {
    Vec::new()
}

impl Client {
    pub fn new(_name: &str) -> Result<Client> {
        Ok(Client { client: 0 })
    }

    pub fn virtual_destination<H: InputHandler + 'static>(
        &self,
        _name: &str,
        _handler: H,
    ) -> Result<Endpoint> {
        Ok(0)
    }

    pub fn dispose_endpoint(&self, _e: Endpoint) {}

    pub fn dispose(&self) {}

    pub fn virtual_source(&self, _name: &str) -> Result<Endpoint> {
        Ok(0)
    }

    pub fn output_port(&self, _name: &str) -> Result<OutPort> {
        Ok(0)
    }

    pub fn input_port<H: InputHandler + 'static>(
        &self,
        _name: &str,
        _handler: H,
    ) -> Result<InputPort> {
        Ok(InputPort { _port: 0 })
    }
}

/// Nothing to start.
pub fn init() {}

/// The MIDI setup never changes.
pub fn setup_generation() -> u64 {
    0
}

/// An input port that never receives anything.
#[derive(Clone, Copy, Debug)]
pub struct InputPort {
    _port: u32,
}

impl InputPort {
    pub fn connect(&self, _src: Endpoint, _tag: usize) -> Result<()> {
        Ok(())
    }

    pub fn disconnect(&self, _src: Endpoint) -> Result<()> {
        Ok(())
    }
}
