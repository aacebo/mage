mod consumer;
mod key;
mod producer;
mod socket;

pub use consumer::*;
pub use key::*;
pub use lapin;
pub use producer::*;
pub use socket::*;

pub fn new(uri: &str) -> SocketOptions {
    SocketOptions::new(uri)
}
