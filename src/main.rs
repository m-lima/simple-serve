#![deny(warnings)]
#![deny(clippy::pedantic)]
#![warn(rust_2018_idioms)]

use warp::Filter;

mod config;

fn print_options(options: &config::Options) {
    println!("Launching on http://0.0.0.0:{}", options.port());
    for route in options.routes() {
        println!("  {} -> {}", route.path, route.action);
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

fn to_route(route: Vec<config::Route>) -> warp::filters::BoxedFilter<(BoxedReply,)> {
    route
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

fn to_filter(route: config::Route) -> warp::filters::BoxedFilter<(BoxedReply,)> {
    let filter = to_path_filter(route.path);

    match route.action {
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
    let options = config::Options::init();
    print_options(&options);

    let (port, routes) = options.decompose();
    let routes = to_route(routes);

    warp::serve(routes).run(([0, 0, 0, 0], port)).await;
}
