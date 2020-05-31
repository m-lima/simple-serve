use clap::{ArgGroup, Clap};

#[derive(Clap, Debug)]
#[clap(name = "Simple Server", about = "A simple multi-purpose server", group = ArgGroup::with_name("routes").required(true).multiple(true))]
struct RawOptions {
    /// Port that the the server should bind to
    #[clap(short, long, default_value = "3030")]
    port: u16,

    /// Serve from filesystem path
    #[clap(short, long, value_name = "path", group = "routes")]
    file: Vec<RawRoute<std::path::PathBuf>>,

    /// Respond with a redirect
    #[clap(short, long, value_name = "URL", group = "routes")]
    redirect: Vec<RawRoute<warp::http::Uri>>,

    /// Respond with status code
    #[clap(short, long, value_name = "status_code", group = "routes")]
    status: Vec<RawRoute<warp::http::StatusCode>>,
}

pub struct Options {
    port: u16,
    routes: Vec<Route>,
}

fn into_route_iter<A>(routes: Vec<RawRoute<A>>) -> impl Iterator<Item = Route>
where
    A: RawAction,
{
    routes.into_iter().map(RawRoute::into_route)
}

impl Options {
    pub fn init() -> Self {
        let raw_options: RawOptions = RawOptions::parse();
        let max_size =
            raw_options.file.len() + raw_options.redirect.len() + raw_options.status.len();

        Self {
            port: raw_options.port,
            routes: into_route_iter(raw_options.status)
                .chain(into_route_iter(raw_options.redirect))
                .chain(into_route_iter(raw_options.file))
                .fold(Vec::with_capacity(max_size), |mut acc, curr| {
                    if acc.contains(&curr) {
                        eprintln!("Ignoring repeated path: {} -> {}", curr.path, curr.action);
                    } else {
                        acc.push(curr);
                    }
                    acc
                }),
        }
    }

    #[inline]
    pub fn port(&self) -> u16 {
        self.port
    }

    #[inline]
    pub fn routes(&self) -> &[Route] {
        &self.routes
    }

    #[inline]
    pub fn decompose(self) -> (u16, Vec<Route>) {
        (self.port, self.routes)
    }
}

#[derive(Debug)]
pub struct Route {
    pub path: Path,
    pub action: Action,
}

impl std::cmp::Eq for Route {}

impl std::cmp::PartialEq for Route {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Path(String);

impl Path {
    pub fn from(path: &str) -> Self {
        Self(path.to_lowercase())
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
struct RawRoute<A>
where
    A: RawAction,
{
    path: Path,
    action: A,
}

impl<A> RawRoute<A>
where
    A: RawAction,
{
    fn into_route(self) -> Route {
        Route {
            path: self.path,
            action: self.action.to_action(),
        }
    }
}

impl<A> std::str::FromStr for RawRoute<A>
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
