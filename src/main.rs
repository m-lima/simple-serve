#![deny(warnings)]
#![deny(clippy::pedantic)]
#![warn(rust_2018_idioms)]

mod config;

type Request = hyper::Request<hyper::Body>;
type Response = hyper::Response<hyper::Body>;

fn print_options(options: &config::Options) {
    println!("Launching on {}", options.address);
    for route in &options.routes {
        println!("  {} -> {}", route.path, route.action);
    }
}

async fn hello_world(request: Request) -> Result<Response, std::convert::Infallible> {
    // let path = request.uri().path();
    Ok(hyper::Response::new(request.uri().path().to_string().into()))
}

#[tokio::main]
async fn main() {
    let options = config::Options::init();
    print_options(&options);

    let make_svc = hyper::service::make_service_fn(|_conn| async {
        // service_fn converts our function into a `Service`
        Ok::<_, std::convert::Infallible>(hyper::service::service_fn(hello_world))
    });

    let server = hyper::Server::bind(&options.address).serve(make_svc);

    // Run this server for... forever!
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
