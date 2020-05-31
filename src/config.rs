#[derive(Debug)]
pub struct Options {
    pub port: u16,
    pub paths: std::collections::HashSet<PathAction>,
}

#[derive(Debug)]
pub struct PathAction {
    pub path: String,
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
