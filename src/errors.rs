use std::fmt;

#[derive(Debug)]
pub enum Error {
    Opencv(opencv::Error),
    DetectionError(ErrorKind),
}

#[derive(Debug)]
pub enum ErrorKind {
    AreaBiggerThanScreen,
    TooSmallArea,
}

impl ErrorKind {
    fn as_str(&self) -> &str {
        match *self {
            ErrorKind::AreaBiggerThanScreen => {
                "Detected playing area is larger than projected area. Re-run with '-f' (fullscreen)"
            }
            ErrorKind::TooSmallArea => "Can't detect playing area",
        }
    }
}

impl From<opencv::Error> for Error {
    fn from(error: opencv::Error) -> Self {
        Self::Opencv(error)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Opencv(ref err) => err.fmt(f),
            Error::DetectionError(ref err) => write!(f, "Detection error: {:?}", err.as_str()),
        }
    }
}
