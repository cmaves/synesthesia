pub mod audio;
#[cfg(feature = "control")]
pub mod control;
#[cfg(feature = "jack-source")]
pub mod jack_src;
pub mod midi;

#[cfg(feature = "jack-source")]
use jack;
#[cfg(feature = "control")]
use ecp;

#[derive(Debug)]
pub enum Error {
    Unrecoverable(String),
    Timeout(String),
    #[cfg(feature = "jack-source")]
    Jack(jack::Error),
	#[cfg(feature = "control")]
	Ecp(ecp::Error)
}

#[cfg(feature = "jack-source")]
impl From<jack::Error> for Error {
    fn from(err: jack::Error) -> Self {
        Error::Jack(err)
    }
}

#[cfg(feature = "control")]
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
