mod handler;
mod patches;

use crate::patches::PatchProvider;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::net::SocketAddr;
use std::sync::Arc;
use clap::Parser;
use log::error;

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let patches = PatchProvider::load_patches("./patch-files").expect("Could not load patches.");
    let patch_ref = Arc::new(patches);

    let addr = SocketAddr::from(([0, 0, 0, 0], 80));

    let make_svc = make_service_fn(move |_| {
        let patch_ref = patch_ref.clone();
        async {
            Ok::<_, anyhow::Error>(service_fn(move |req| {
                handler::serve(req, patch_ref.clone())
            }))
        }
    });

    let server = Server::bind(&addr).serve(make_svc);
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        error!("server error: {}", e);
    }
}
