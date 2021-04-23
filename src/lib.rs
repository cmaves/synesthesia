pub mod audio;
pub mod control;
#[cfg(feature = "jack")]
pub mod jack_src;
pub mod midi;

#[cfg(feature = "jack")]
use jack;
#[cfg(any(feature = "ecp", feature = "rpi"))]
use lecp;

#[derive(Debug)]
pub enum Error {
    Unrecoverable(String),
    Timeout(String),
    #[cfg(feature = "jack")]
    Jack(jack::Error),
    Lecp(lecp::Error),
}

#[cfg(feature = "jack")]
impl From<jack::Error> for Error {
    fn from(err: jack::Error) -> Self {
        Error::Jack(err)
    }
}

impl From<lecp::Error> for Error {
    fn from(err: lecp::Error) -> Self {
        Error::Lecp(err)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
