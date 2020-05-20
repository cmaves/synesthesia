pub mod audio;
pub mod control;
#[cfg(feature = "jack")]
pub mod jack_src;
pub mod midi;

#[cfg(any(feature = "ecp", feature = "rpi"))]
use ecp;
#[cfg(feature = "jack")]
use jack;

#[derive(Debug)]
pub enum Error {
    Unrecoverable(String),
    Timeout(String),
    #[cfg(feature = "jack")]
    Jack(jack::Error),
    Ecp(ecp::Error),
}

#[cfg(feature = "jack")]
impl From<jack::Error> for Error {
    fn from(err: jack::Error) -> Self {
        Error::Jack(err)
    }
}

impl From<ecp::Error> for Error {
    fn from(err: ecp::Error) -> Self {
        Error::Ecp(err)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
