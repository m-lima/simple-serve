#[derive(Debug)]
pub struct Options {
    pub port: u16,
    pub paths: std::collections::HashSet<PathAction>,
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
