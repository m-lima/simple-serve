use clap::{ArgGroup, Clap};

#[derive(Clap, Debug)]
#[clap(name = "Simple Server", about = "A simple multi-purpose server", group = ArgGroup::with_name("routes").required(true))]
struct RawOptions {
    /// Port that the the server should bind to
    #[clap(short, long, default_value = "3030")]
    port: u16,

    /// Serve from filesystem path
    #[clap(short, long, value_name = "path", group = "routes")]
    file: Vec<RawPathAction<std::path::PathBuf>>,

    /// Respond with a redirect
    #[clap(short, long, value_name = "URL", group = "routes")]
    redirect: Vec<RawPathAction<warp::http::Uri>>,

    /// Respond with status code
    #[clap(short, long, value_name = "status_code", group = "routes")]
    status: Vec<RawPathAction<warp::http::StatusCode>>,
}

pub struct Options {
    pub port: u16,
    pub paths: std::collections::HashSet<PathAction>,
}

impl Options {
    pub fn init() -> Self {
        let raw_options: RawOptions = RawOptions::parse();
        let mut paths = std::collections::HashSet::with_capacity(
            raw_options.file.len() + raw_options.redirect.len() + raw_options.status.len(),
        );
        raw_options
            .file
            .into_iter()
            .map(RawPathAction::to_path_action)
            .for_each(|p| {
                paths.insert(p);
            });
        raw_options
            .redirect
            .into_iter()
            .map(RawPathAction::to_path_action)
            .for_each(|p| {
                paths.insert(p);
            });
        raw_options
            .status
            .into_iter()
            .map(RawPathAction::to_path_action)
            .for_each(|p| {
                paths.insert(p);
            });
        Self {
            port: raw_options.port,
            paths,
        }
    }
}

#[derive(Debug)]
pub struct PathAction {
    pub path: Path,
    pub action: Action,
}

impl std::cmp::Eq for PathAction {}

impl std::cmp::PartialEq for PathAction {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

impl std::hash::Hash for PathAction {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.path.hash(state)
    }
}

#[derive(Debug, Eq, PartialEq, Hash)]
pub struct Path(String);

impl Path {
    pub fn from(path_string: &str) -> Self {
        Self(path_string.to_lowercase())
    }

    #[inline]
    pub fn into_string(self) -> String {
        self.0
    }
}

impl std::convert::AsRef<str> for Path {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl std::fmt::Display for Path {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "/{}", self.0)
    }
}

#[derive(Debug)]
pub enum Action {
    ServePath(std::path::PathBuf),
    Redirect(warp::http::Uri),
    StatusCode(warp::http::StatusCode),
}

impl std::fmt::Display for Action {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::ServePath(path) => write!(fmt, "Serving {}", path.display()),
            Action::Redirect(uri) => write!(fmt, "Redirecting to {}", uri),
            Action::StatusCode(status) => write!(fmt, "Responding {}", status),
        }
    }
}

#[derive(Debug)]
struct RawPathAction<A>
where
    A: RawAction,
{
    path: Path,
    action: A,
}

impl<A> RawPathAction<A>
where
    A: RawAction,
{
    fn to_path_action(self) -> PathAction {
        PathAction {
            path: self.path,
            action: self.action.to_action(),
        }
    }
}

impl<A> std::str::FromStr for RawPathAction<A>
where
    A: RawAction,
{
    type Err = ArgError;

    fn from_str(string: &str) -> Result<Self, Self::Err> {
        let parts = string
            .find(':')
            .map(|index| (Path::from(&string[..index]), &string[index + 1..]))
            .ok_or(ArgError::Format)?;
        let action = A::from_str(parts.1).map_err(|_| ArgError::Convert)?;
        if action.valid() {
            Ok(Self {
                path: parts.0,
                action,
            })
        } else {
            Err(ArgError::Invalid(parts.1.to_string()))
        }
    }
}

trait RawAction: std::str::FromStr {
    fn to_action(self) -> Action;
    fn valid(&self) -> bool {
        true
    }
}

impl RawAction for std::path::PathBuf {
    fn to_action(self) -> Action {
        Action::ServePath(self)
    }

    fn valid(&self) -> bool {
        self.exists()
    }
}

impl RawAction for warp::http::Uri {
    fn to_action(self) -> Action {
        Action::Redirect(self)
    }
}

impl RawAction for warp::http::StatusCode {
    fn to_action(self) -> Action {
        Action::StatusCode(self)
    }
}

#[derive(Debug)]
pub enum ArgError {
    Format,
    Convert,
    Invalid(String),
}

impl std::error::Error for ArgError {}

impl std::fmt::Display for ArgError {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ArgError::Format => write!(fmt, "Expected format [URL_PATH]:<VALUE>"),
            ArgError::Convert => write!(fmt, "Failed to convert value"),
            ArgError::Invalid(value) => write!(fmt, "Value passed is not valid: {}", value),
        }
    }
}
