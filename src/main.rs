#![deny(warnings)]
#![deny(clippy::pedantic)]
#![warn(rust_2018_idioms)]

use warp::Filter;

mod config;

fn print_options(options: &config::Options) {
    println!("Launching on http://0.0.0.0:{}", options.port);
    for path_action in &options.paths {
        println!("  {} -> {}", path_action.path, path_action.action);
    }
}

struct BoxedReply {
    reply: Box<dyn warp::Reply>,
}

impl BoxedReply {
    fn new(reply: impl 'static + warp::Reply) -> Self {
        Self {
            reply: Box::new(reply),
        }
    }
}

impl warp::Reply for BoxedReply {
    fn into_response(self) -> warp::reply::Response {
        self.reply.into_response()
    }
}

fn to_route(
    path_action: std::collections::HashSet<config::PathAction>,
) -> warp::filters::BoxedFilter<(BoxedReply,)> {
    path_action
        .into_iter()
        .map(to_filter)
        .fold(
            Option::<warp::filters::BoxedFilter<(BoxedReply,)>>::None,
            |routes, filter| {
                if let Some(previous) = routes {
                    Some(previous.or(filter).map(BoxedReply::new).boxed())
                } else {
                    Some(filter)
                }
            },
        )
        .unwrap()
}

// Allowed because filter_map here is dumb
#[allow(clippy::filter_map)]
fn to_path_filter(path: config::Path) -> warp::filters::BoxedFilter<()> {
    path.into_string()
        .split('/')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .map(ToString::to_string)
        .fold(warp::any().boxed(), |filter, path| {
            filter.and(warp::path(path)).boxed()
        })
}

fn to_filter(path_action: config::PathAction) -> warp::filters::BoxedFilter<(BoxedReply,)> {
    let filter = to_path_filter(path_action.path);

    match path_action.action {
        config::Action::ServePath(path) => {
            if path.is_dir() {
                filter.and(warp::fs::dir(path)).map(BoxedReply::new).boxed()
            } else {
                filter
                    .and(warp::fs::file(path))
                    .map(BoxedReply::new)
                    .boxed()
            }
        }
        config::Action::Redirect(url) => filter
            .and(warp::path::end())
            .map(move || warp::redirect(url.clone()))
            .map(BoxedReply::new)
            .boxed(),
        config::Action::StatusCode(status) => filter
            .and(warp::path::end())
            .map(warp::reply)
            .map(move |r| warp::reply::with_status(r, status))
            .map(BoxedReply::new)
            .boxed(),
    }
}

#[tokio::main]
async fn main() {
    use std::iter::FromIterator;

    let path = std::env::args().nth(1).map_or_else(
        || {
            eprintln!("No path provided");
            std::process::exit(-1);
        },
        |path_string| {
            let path = std::path::PathBuf::from(path_string.clone());

            if !path.exists() {
                eprintln!("Path provided does not exist: {}", &path_string);
                std::process::exit(-1);
            }

            if !path.is_dir() {
                eprintln!("Path provided is not a directory: {}", &path_string);
                std::process::exit(-1);
            }

            path
        },
    );

    let port = std::env::args()
        .nth(2)
        .map_or(Ok(3030_u16), |arg| arg.parse::<u16>())
        .unwrap_or_else(|err| {
            eprintln!("Invalid port: {}", err);
            std::process::exit(-1);
        });

    let options = config::Options {
        port,
        paths: std::collections::HashSet::from_iter(
            vec![
                config::PathAction {
                    path: config::Path::from("bla"),
                    action: config::Action::ServePath(path.clone()),
                },
                config::PathAction {
                    path: config::Path::from("ble"),
                    action: config::Action::Redirect(warp::http::Uri::from_static(
                        "http://www.google.com",
                    )),
                },
                config::PathAction {
                    path: config::Path::from("bli"),
                    action: config::Action::StatusCode(
                        warp::http::StatusCode::from_u16(550).unwrap(),
                    ),
                },
                config::PathAction {
                    path: config::Path::from("bli/bla"),
                    action: config::Action::StatusCode(
                        warp::http::StatusCode::from_u16(555).unwrap(),
                    ),
                },
            ]
            .into_iter(),
        ),
    };

    print_options(&options);

    let (port, paths) = (options.port, options.paths);
    let routes = to_route(paths);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}
