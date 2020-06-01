use clap::{ArgGroup, Clap};

#[derive(Clap, Debug)]
#[clap(name = "Simple Serve", about = "A simple multi-purpose server", group = ArgGroup::with_name("routes").required(true).multiple(true))]
struct RawOptions {
    /// Port that the the server should bind to
    #[clap(short, long, default_value = "[::1]:3030")]
    address: std::net::SocketAddr,

    /// Serve from filesystem path
    #[clap(short, long, value_name = "path", group = "routes")]
    file: Vec<RawRoute<std::path::PathBuf>>,

    /// Respond with a redirect
    #[clap(short, long, value_name = "URL", group = "routes")]
    redirect: Vec<RawRoute<hyper::http::Uri>>,

    /// Respond with status code
    #[clap(short, long, value_name = "status_code", group = "routes")]
    status: Vec<RawRoute<hyper::http::StatusCode>>,
}

pub struct Options {
    pub address: std::net::SocketAddr,
    pub routes: Vec<Route>,
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
            address: raw_options.address,
            routes: into_route_iter(raw_options.status)
                .chain(into_route_iter(raw_options.redirect))
                .chain(into_route_iter(raw_options.file))
                .fold(Vec::with_capacity(max_size), |mut acc, curr| {
                    match acc.binary_search(&curr) {
                        Ok(_) => {
                            eprintln!("Ignoring repeated path: {} -> {}", curr.path, curr.action);
                        }
                        Err(pos) => {
                            acc.insert(pos, curr);
                        }
                    }
                    acc
                }),
        }
    }
}

#[derive(Debug)]
pub struct Route {
    pub path: String,
    pub method: Option<hyper::Method>,
    pub action: Action,
}

impl Route {
    pub fn compare(&self, path: &str, method: Option<&hyper::Method>) -> std::cmp::Ordering {
        match self.path.as_str().cmp(path) {
            std::cmp::Ordering::Equal => {
                if let Some(self_method) = &self.method {
                    if let Some(other_method) = method {
                        self_method.as_str().cmp(other_method.as_str())
                    } else {
                        std::cmp::Ordering::Equal
                    }
                } else {
                    std::cmp::Ordering::Equal
                }
            }
            o => o,
        }
    }
}

impl std::cmp::Eq for Route {}

impl std::cmp::PartialEq for Route {
    fn eq(&self, other: &Self) -> bool {
        self.path.eq(&other.path) && self.method.eq(&other.method)
    }
}

impl std::cmp::PartialOrd for Route {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl std::cmp::Ord for Route {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.compare(&other.path, other.method.as_ref())
    }
}

#[derive(Debug, Clone)]
pub enum Action {
    ServeFile(std::path::PathBuf),
    ServePath(std::path::PathBuf),
    Redirect(hyper::http::Uri),
    StatusCode(hyper::http::StatusCode),
}

impl std::fmt::Display for Action {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Action::ServeFile(path) => write!(fmt, "Serving file {}", path.display()),
            Action::ServePath(path) => write!(fmt, "Serving path {}", path.display()),
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
    path: String,
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
            method: None,
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
            .map(|index| (String::from(&string[..index]), &string[index + 1..]))
            .ok_or(ArgError::Format)?;
        let path = if parts.0.starts_with('/') {
            parts.0
        } else {
            format!("/{}", parts.0)
        };
        let action = A::from_str(parts.1).map_err(|_| ArgError::Convert)?;
        if action.valid() {
            Ok(Self { path, action })
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
        if self.is_dir() {
            Action::ServePath(self)
        } else {
            Action::ServeFile(self)
        }
    }

    fn valid(&self) -> bool {
        self.exists()
    }
}

impl RawAction for hyper::http::Uri {
    fn to_action(self) -> Action {
        Action::Redirect(self)
    }
}

impl RawAction for hyper::http::StatusCode {
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
