use std::fmt::Display;
use std::str::FromStr;
use std::{error::Error, fmt};
use tracing::debug;
use url::Url;

mod scheme;

pub use crate::scheme::Scheme;

/// GitUrl represents an input url that is a url used by git
/// Internally during parsing the url is sanitized and uses the `url` crate to perform
/// the majority of the parsing effort, and with some extra handling to expose
/// metadata used my many git hosting services
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GitUrl {
    /// The fully qualified domain name (FQDN) or IP of the repo
    pub host: Option<String>,
    /// The name of the repo
    pub name: String,
    /// The owner/account/project name
    pub owner: Option<String>,
    /// The organization name. Supported by Azure DevOps
    pub organization: Option<String>,
    /// The full name of the repo, formatted as "owner/name"
    pub fullname: String,
    /// The git url scheme
    pub scheme: Scheme,
    /// The authentication user
    pub user: Option<String>,
    /// The oauth token (could appear in the https urls)
    pub token: Option<String>,
    /// The non-conventional port where git service is hosted
    pub port: Option<u16>,
    /// The path to repo w/ respect to user + hostname
    pub path: String,
    /// Indicate if url uses the .git suffix
    pub git_suffix: bool,
    /// Indicate if url explicitly uses its scheme
    pub scheme_prefix: bool,
}

/// Build the printable GitUrl from its components
impl fmt::Display for GitUrl {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let scheme_prefix = match self.scheme_prefix {
            true => format!("{}://", self.scheme),
            false => String::new(),
        };

        let auth_info = match self.scheme {
            Scheme::Ssh | Scheme::Git | Scheme::GitSsh => {
                if let Some(user) = &self.user {
                    format!("{}@", user)
                } else {
                    String::new()
                }
            }
            Scheme::Http | Scheme::Https => match (&self.user, &self.token) {
                (Some(user), Some(token)) => format!("{}:{}@", user, token),
                (Some(user), None) => format!("{}@", user),
                (None, Some(token)) => format!("{}@", token),
                (None, None) => String::new(),
            },
            _ => String::new(),
        };

        let host = match &self.host {
            Some(host) => host.to_string(),
            None => String::new(),
        };

        let port = match &self.port {
            Some(p) => format!(":{}", p),
            None => String::new(),
        };

        let path = match &self.scheme {
            Scheme::Ssh => {
                if self.port.is_some() {
                    format!("/{}", &self.path)
                } else {
                    format!(":{}", &self.path)
                }
            }
            _ => (&self.path).to_string(),
        };

        let git_url_str = format!("{}{}{}{}{}", scheme_prefix, auth_info, host, port, path);

        write!(f, "{}", git_url_str)
    }
}

impl Default for GitUrl {
    fn default() -> Self {
        GitUrl {
            host: None,
            name: "".to_string(),
            owner: None,
            organization: None,
            fullname: "".to_string(),
            scheme: Scheme::Unspecified,
            user: None,
            token: None,
            port: None,
            path: "".to_string(),
            git_suffix: false,
            scheme_prefix: false,
        }
    }
}

#[derive(Debug)]
#[non_exhaustive]
pub struct FromStrError {
    url: String,
    kind: FromStrErrorKind,
}

impl Display for FromStrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            FromStrErrorKind::NormalizeUrl(_) => {
                write!(f, "unable to normalize URL `{}`", self.url)
            }
            FromStrErrorKind::UrlHost => {
                write!(f, "could not isolate host from URL `{}`", self.url)
            }
            FromStrErrorKind::UnsupportedScheme => {
                write!(f, "unsupported scheme`",)
            }
            FromStrErrorKind::MalformedGitUrl => {
                write!(f, "unknown format of git URL `{}`", self.url)
            }
        }
    }
}

impl Error for FromStrError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            FromStrErrorKind::NormalizeUrl(err) => Some(err),
            FromStrErrorKind::UrlHost => None,
            FromStrErrorKind::UnsupportedScheme => None,
            FromStrErrorKind::MalformedGitUrl => None,
        }
    }
}

#[derive(Debug)]
pub enum FromStrErrorKind {
    #[non_exhaustive]
    NormalizeUrl(NormalizeUrlError),
    #[non_exhaustive]
    UrlHost,
    #[non_exhaustive]
    UnsupportedScheme,
    #[non_exhaustive]
    MalformedGitUrl,
}

impl FromStr for GitUrl {
    type Err = FromStrError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        GitUrl::parse(s)
    }
}

impl GitUrl {
    /// Returns `GitUrl` after removing `user` and `token` values
    /// Intended use-case is for non-destructive printing GitUrl excluding any embedded auth info
    pub fn trim_auth(&self) -> GitUrl {
        let mut new_giturl = self.clone();
        new_giturl.user = None;
        new_giturl.token = None;
        new_giturl
    }

    /// Normalizes and parses `url` for metadata
    pub fn parse(url: &str) -> Result<GitUrl, FromStrError> {
        // Normalize the url so we can use Url crate to process ssh urls
        let normalized = normalize_url(url).map_err(|err| FromStrError {
            url: url.to_owned(),
            kind: FromStrErrorKind::NormalizeUrl(err),
        })?;

        // Some pre-processing for paths
        // REFACTOR: write Scheme::from_str explicitly and include that error in the chain
        let scheme = Scheme::from_str(normalized.scheme()).map_err(|_err| FromStrError {
            url: url.to_owned(),
            kind: FromStrErrorKind::UnsupportedScheme,
        })?;

        // Normalized ssh urls can always have their first '/' removed
        let urlpath = match &scheme {
            Scheme::Ssh => {
                // At the moment, we're relying on url::Url's parse() behavior to not duplicate
                // the leading '/' when we normalize
                normalized.path()[1..].to_string()
            }
            _ => normalized.path().to_string(),
        };

        let git_suffix_check = &urlpath.ends_with(".git");

        // Parse through path for name,owner,organization
        // Support organizations for Azure Devops
        debug!("The urlpath: {:?}", &urlpath);

        // Most git services use the path for metadata in the same way, so we're going to separate
        // the metadata
        // ex. github.com/accountname/reponame
        // owner = accountname
        // name = reponame
        //
        // organizations are going to be supported on a per-host basis
        let splitpath = &urlpath.rsplit_terminator('/').collect::<Vec<&str>>();
        debug!("rsplit results for metadata: {:?}", splitpath);

        let name = splitpath[0].trim_end_matches(".git").to_string();

        let (owner, organization, fullname) = match &scheme {
            // We're not going to assume anything about metadata from a filepath
            Scheme::File => (None::<String>, None::<String>, name.clone()),
            _ => {
                let mut fullname: Vec<&str> = Vec::new();

                // TODO: Add support for parsing out orgs from these urls
                let hosts_w_organization_in_path = vec!["dev.azure.com", "ssh.dev.azure.com"];
                //vec!["dev.azure.com", "ssh.dev.azure.com", "visualstudio.com"];

                let host_str = normalized.host_str().ok_or_else(|| FromStrError {
                    url: url.to_owned(),
                    kind: FromStrErrorKind::UrlHost,
                })?;

                match hosts_w_organization_in_path.contains(&host_str) {
                    true => {
                        debug!("Found a git provider with an org");

                        // The path differs between git:// and https:// schemes

                        match &scheme {
                            // Example: "git@ssh.dev.azure.com:v3/CompanyName/ProjectName/RepoName",
                            Scheme::Ssh => {
                                // Organization
                                fullname.push(splitpath[2]);
                                // Project/Owner name
                                fullname.push(splitpath[1]);
                                // Repo name
                                fullname.push(splitpath[0]);

                                (
                                    Some(splitpath[1].to_string()),
                                    Some(splitpath[2].to_string()),
                                    fullname.join("/"),
                                )
                            }
                            // Example: "https://CompanyName@dev.azure.com/CompanyName/ProjectName/_git/RepoName",
                            Scheme::Https => {
                                // Organization
                                fullname.push(splitpath[3]);
                                // Project/Owner name
                                fullname.push(splitpath[2]);
                                // Repo name
                                fullname.push(splitpath[0]);

                                (
                                    Some(splitpath[2].to_string()),
                                    Some(splitpath[3].to_string()),
                                    fullname.join("/"),
                                )
                            }
                            _ => {
                                return Err(FromStrError {
                                    url: url.to_owned(),
                                    kind: FromStrErrorKind::UnsupportedScheme,
                                });
                            }
                        }
                    }
                    false => {
                        if !url.starts_with("ssh") && splitpath.len() < 2 {
                            return Err(FromStrError {
                                url: url.to_owned(),
                                kind: FromStrErrorKind::MalformedGitUrl,
                            });
                        }

                        let position = match splitpath.len() {
                            0 => {
                                return Err(FromStrError {
                                    url: url.to_owned(),
                                    kind: FromStrErrorKind::MalformedGitUrl,
                                })
                            }
                            1 => 0,
                            _ => 1,
                        };

                        // push owner
                        fullname.push(splitpath[position]);
                        // push name
                        fullname.push(name.as_str());

                        (
                            Some(splitpath[position].to_string()),
                            None::<String>,
                            fullname.join("/"),
                        )
                    }
                }
            }
        };

        let final_host = match scheme {
            Scheme::File => None,
            _ => normalized.host_str().map(|h| h.to_string()),
        };

        let final_path = match scheme {
            Scheme::File => {
                if let Some(host) = normalized.host_str() {
                    format!("{}{}", host, urlpath)
                } else {
                    urlpath
                }
            }
            _ => urlpath,
        };

        Ok(GitUrl {
            host: final_host,
            name,
            owner,
            organization,
            fullname,
            scheme,
            user: match normalized.username().to_string().len() {
                0 => None,
                _ => Some(normalized.username().to_string()),
            },
            token: normalized.password().map(|p| p.to_string()),
            port: normalized.port(),
            path: final_path,
            git_suffix: *git_suffix_check,
            scheme_prefix: url.contains("://") || url.starts_with("git:"),
        })
    }
}

/// `normalize_ssh_url` takes in an ssh url that separates the login info
/// from the path into with a `:` and replaces it with `/`.
///
/// Prepends `ssh://` to url
///
/// Supports absolute and relative paths
fn normalize_ssh_url(url: &str) -> Result<Url, NormalizeUrlError> {
    let u = url.split(':').collect::<Vec<&str>>();

    match u.len() {
        2 => {
            debug!("Normalizing ssh url: {:?}", u);
            normalize_url(&format!("ssh://{}/{}", u[0], u[1]))
        }
        3 => {
            debug!("Normalizing ssh url with ports: {:?}", u);
            normalize_url(&format!("ssh://{}:{}/{}", u[0], u[1], u[2]))
        }
        _default => Err(NormalizeUrlError {
            kind: NormalizeUrlErrorKind::UnsupportedSshPattern {
                url: url.to_owned(),
            },
        }),
    }
}

/// `normalize_file_path` takes in a filepath and uses `Url::from_file_path()` to parse
///
/// Prepends `file://` to url
#[cfg(any(unix, windows, target_os = "redox", target_os = "wasi"))]
fn normalize_file_path(filepath: &str) -> Result<Url, NormalizeUrlError> {
    let fp = Url::from_file_path(filepath);

    match fp {
        Ok(path) => Ok(path),
        Err(_e) => normalize_url(&format!("file://{}", filepath)),
    }
}

#[cfg(target_arch = "wasm32")]
fn normalize_file_path(_filepath: &str) -> Result<Url> {
    unreachable!()
}

#[derive(Debug)]
#[non_exhaustive]
pub struct NormalizeUrlError {
    kind: NormalizeUrlErrorKind,
}

impl Display for NormalizeUrlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            NormalizeUrlErrorKind::NullBytes => write!(f, "input URL contains null bytes"),
            NormalizeUrlErrorKind::UrlParse(_) => write!(f, "unable to parse URL"),
            NormalizeUrlErrorKind::UnsupportedSshPattern { url } => {
                write!(f, "unsupported SSH pattern `{}`", url)
            }
            NormalizeUrlErrorKind::UnsupportedScheme => write!(f, "unsupported URL scheme"),
        }
    }
}

impl Error for NormalizeUrlError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.kind {
            NormalizeUrlErrorKind::NullBytes => None,
            NormalizeUrlErrorKind::UrlParse(err) => Some(err),
            NormalizeUrlErrorKind::UnsupportedSshPattern { url: _ } => None,
            NormalizeUrlErrorKind::UnsupportedScheme => None,
        }
    }
}

#[derive(Debug)]
pub enum NormalizeUrlErrorKind {
    #[non_exhaustive]
    NullBytes,
    #[non_exhaustive]
    UrlParse(url::ParseError),
    #[non_exhaustive]
    UnsupportedSshPattern { url: String },
    #[non_exhaustive]
    UnsupportedScheme,
}

/// `normalize_url` takes in url as `&str` and takes an opinionated approach to identify
/// `ssh://` or `file://` urls that require more information to be added so that
/// they can be parsed more effectively by `url::Url::parse()`
pub fn normalize_url(url: &str) -> Result<Url, NormalizeUrlError> {
    debug!("Processing: {:?}", &url);

    // Error if there are null bytes within the url
    // https://github.com/tjtelan/git-url-parse-rs/issues/16
    if url.contains('\0') {
        return Err(NormalizeUrlError {
            kind: NormalizeUrlErrorKind::NullBytes,
        });
    }

    // We're going to remove any trailing slash before running through Url::parse
    let url = url.trim_end_matches('/');

    // Normalize short git url notation: git:host/path.
    // This is the same as matching Regex::new(r"^git:[^/]")
    let url_starts_with_git_but_no_slash = url.starts_with("git:") && url.get(4..5) != Some("/");
    let url_to_parse = if url_starts_with_git_but_no_slash {
        url.replace("git:", "git://")
    } else {
        url.to_string()
    };

    let url_parse = Url::parse(&url_to_parse);

    Ok(match url_parse {
        Ok(u) => match Scheme::from_str(u.scheme()) {
            Ok(_) => u,
            Err(_) => normalize_ssh_url(url)?,
        },
        Err(url::ParseError::RelativeUrlWithoutBase) => {
            // If we're here, we're only looking for Scheme::Ssh or Scheme::File

            // Assuming we have found Scheme::Ssh if we can find an "@" before ":"
            // Otherwise we have Scheme::File
            match string_contains_asperand_before_colon(url) {
                true => {
                    debug!("Scheme::SSH match for normalization");
                    normalize_ssh_url(url)?
                }
                false => {
                    debug!("Scheme::File match for normalization");
                    normalize_file_path(url)?
                }
            }
        }
        Err(err) => {
            return Err(NormalizeUrlError {
                kind: NormalizeUrlErrorKind::UrlParse(err),
            });
        }
    })
}

/// This is the same as matching Regex::new(r"^\S+(@)\S+(:).*$");
fn string_contains_asperand_before_colon(str: &str) -> bool {
    let index_of_asperand = str.find('@');
    let index_of_colon = str.find(':');

    match (index_of_asperand, index_of_colon) {
        (Some(index_of_asperand), Some(index_of_colon)) => index_of_asperand < index_of_colon,
        _ => false,
    }
}
