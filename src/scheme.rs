use std::{
    error::Error,
    fmt::{self, Display},
    str::FromStr,
};

/// Supported URI schemes for parsing
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Scheme {
    /// Represents `file://` url scheme
    File,
    /// Represents `ftp://` url scheme
    Ftp,
    /// Represents `ftps://` url scheme
    Ftps,
    /// Represents `git://` url scheme
    Git,
    /// Represents `git+ssh://` url scheme
    GitSsh,
    /// Represents `http://` url scheme
    Http,
    /// Represents `https://` url scheme
    Https,
    /// Represents `ssh://` url scheme
    Ssh,
    /// Represents No url scheme
    Unspecified,
}

impl Display for Scheme {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Scheme::File => write!(f, "file"),
            Scheme::Ftp => write!(f, "ftp"),
            Scheme::Ftps => write!(f, "ftps"),
            Scheme::Git => write!(f, "git"),
            Scheme::GitSsh => write!(f, "git+ssh"),
            Scheme::Http => write!(f, "http"),
            Scheme::Https => write!(f, "https"),
            Scheme::Ssh => write!(f, "ssh"),
            Scheme::Unspecified => write!(f, "unspecified"),
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct FromStrError {
    kind: FromStrErrorKind,
}

impl Display for FromStrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            FromStrErrorKind::UnsupportedScheme(scheme) => {
                write!(f, "unsupported scheme `{}`", scheme)
            }
        }
    }
}

impl Error for FromStrError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            FromStrErrorKind::UnsupportedScheme(_) => None,
        }
    }
}

#[derive(Debug)]
pub enum FromStrErrorKind {
    #[non_exhaustive]
    UnsupportedScheme(String),
}

impl FromStr for Scheme {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "file" => Ok(Scheme::File),
            "ftp" => Ok(Scheme::Ftp),
            "ftps" => Ok(Scheme::Ftps),
            "git" => Ok(Scheme::Git),
            "git+ssh" => Ok(Scheme::GitSsh),
            "http" => Ok(Scheme::Http),
            "https" => Ok(Scheme::Https),
            "ssh" => Ok(Scheme::Ssh),
            "unspecified" => Ok(Scheme::Unspecified),
            _ => Err(FromStrError {
                kind: FromStrErrorKind::UnsupportedScheme(s.to_owned()),
            }),
        }
    }
}
