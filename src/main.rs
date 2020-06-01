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

impl<T> std::convert::Into<usize> for Sender<T> {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl<T> std::convert::From<usize> for Sender<T> {
    fn from(pointer: usize) -> Self {
        Sender(pointer as *const T)
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

fn not_found() -> Result<Response, std::convert::Infallible> {
    Ok(hyper::Response::builder()
        .status(hyper::StatusCode::NOT_FOUND)
        .body(NOT_FOUND.into())
        .unwrap())
}

#[tokio::main]
async fn main() {
    let options = config::Options::init();
    print_options(&options);

    let (address, routes) = (options.address, options.routes);

    let make_svc = hyper::service::make_service_fn(move |_| {
        let sender: usize = Sender(&routes).into();
        async move {
            Ok::<_, std::convert::Infallible>(hyper::service::service_fn(
                move |request: Request| async move {
                    let routes: Sender<Vec<config::Route>> = sender.into();
                    let path = request.uri().path();
                    let method = request.method();

                    match routes.binary_search_by(|r| r.compare(path, Some(method))) {
                        Ok(index) => {
                            let route = &routes[index];
                            match &route.action {
                                config::Action::StatusCode(status) => {
                                    Ok::<Response, std::convert::Infallible>(
                                        hyper::Response::builder()
                                            .status(status)
                                            .body(format!("{}", status).into())
                                            .unwrap(),
                                    )
                                }
                                config::Action::Redirect(uri) => {
                                    Ok::<Response, std::convert::Infallible>(
                                        hyper::Response::builder()
                                            .status(hyper::StatusCode::MOVED_PERMANENTLY)
                                            .header(hyper::header::LOCATION, uri.to_string())
                                            .body(format!("{}", uri).into())
                                            .unwrap(),
                                    )
                                }
                                config::Action::ServeFile(system_path) => {
                                    if let Ok(file) = tokio::fs::File::open(system_path).await {
                                        let stream = tokio_util::codec::FramedRead::new(
                                            file,
                                            tokio_util::codec::BytesCodec::new(),
                                        );
                                        let body = hyper::Body::wrap_stream(stream);
                                        Ok(Response::new(body))
                                    } else {
                                        not_found()
                                    }
                                }
                                config::Action::ServePath(_system_path) => {
                                    Ok::<Response, std::convert::Infallible>(hyper::Response::new(
                                        NOT_FOUND.into(),
                                    ))
                                }
                            }
                        }
                        Err(_) => not_found(),
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
