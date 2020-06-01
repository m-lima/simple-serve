#![deny(warnings)]
#![deny(clippy::pedantic)]
#![warn(rust_2018_idioms)]

mod config;

type Request = hyper::Request<hyper::Body>;
type Response = hyper::Response<hyper::Body>;

static NOT_FOUND: &[u8] = b"Not found";

#[derive(Copy, Clone)]
pub struct Sender<T>(*const T);

impl<T> std::ops::Deref for Sender<T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

unsafe impl<T> Send for Sender<T> {}

unsafe impl<T> Sync for Sender<T> {}

fn print_options(options: &config::Options) {
    println!("Launching on {}", options.address);
    for route in &options.routes {
        println!("  {} -> {}", route.path, route.action);
    }
}

#[tokio::main]
async fn main() {
    let options = config::Options::init();
    print_options(&options);

    let (address, routes) = (options.address, options.routes);

    let make_svc = hyper::service::make_service_fn(move |_| {
        let sender = Sender(&routes);
        async {
            Ok::<_, std::convert::Infallible>(hyper::service::service_fn(
                move |request: Request| {
                    let path = request.uri().path();
                    let method = request.method();

                    let action = sender
                        .binary_search_by(|r| r.compare(path, Some(method)))
                        .map(|i| sender[i].action.clone())
                        .ok();

                    async move {
                        match action {
                            Some(config::Action::StatusCode(status)) => Ok::<Response, std::convert::Infallible>(
                                hyper::Response::builder()
                                    .status(status)
                                    .body(format!("{}", status).into())
                                    .unwrap(),
                            ),
                            Some(config::Action::Redirect(uri)) => Ok::<Response, std::convert::Infallible>(
                                hyper::Response::builder()
                                    .status(hyper::StatusCode::MOVED_PERMANENTLY)
                                    .header(hyper::header::LOCATION, uri.to_string())
                                    .body(format!("{}", uri).into())
                                    .unwrap(),
                            ),
                            Some(config::Action::ServePath(_)) => Ok::<Response, std::convert::Infallible>(
                                hyper::Response::new(NOT_FOUND.into()),
                            ),
                            None => Ok::<Response, std::convert::Infallible>(
                                hyper::Response::builder()
                                    .status(hyper::StatusCode::NOT_FOUND)
                                    .body(NOT_FOUND.into())
                                    .unwrap(),
                            ),
                        }
                    }
                },
            ))
        }
    });

    let server = hyper::Server::bind(&address).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
