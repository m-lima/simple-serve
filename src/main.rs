#![deny(warnings)]
#![deny(clippy::pedantic)]
#![warn(rust_2018_idioms)]

mod config;

type Request = hyper::Request<hyper::Body>;
type Response = hyper::Response<hyper::Body>;

static OK: &[u8] = b"Ok";
static NOT_FOUND: &[u8] = b"Not found";

#[derive(Copy, Clone)]
pub struct Sender<T> {
    data: *const T,
}

impl<T> std::ops::Deref for Sender<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.data }
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
        let sender = Sender { data: &routes };
        async {
            Ok::<_, std::convert::Infallible>(hyper::service::service_fn(
                move |request: Request| {
                    let routes = &sender;

                    let path = request.uri().path();
                    let method = request.method();

                    println!("{} {}", method, path);
                    let response = match routes.binary_search_by(|r| r.compare(path, Some(method)))
                    {
                        Ok(_) => Ok::<Response, std::convert::Infallible>(hyper::Response::new(
                            OK.into(),
                        )),
                        Err(_) => Ok::<Response, std::convert::Infallible>(
                            hyper::Response::builder()
                                .status(hyper::StatusCode::NOT_FOUND)
                                .body(NOT_FOUND.into())
                                .unwrap(),
                        ),
                    };

                    async { response }
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
