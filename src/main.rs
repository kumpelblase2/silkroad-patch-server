mod handler;
mod patches;

use crate::patches::PatchProvider;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};
use std::sync::mpsc::channel;
use std::time::Duration;
use clap::Parser;
use log::{debug, error, info};
use notify::{RecursiveMode, watcher, Watcher, DebouncedEvent};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Root directory for patch files
    #[clap(short, long, default_value = "./patch-files")]
    dir: String,

    /// Port to listen on
    #[clap(short, long, default_value_t = 80)]
    port: u16,
}

async fn shutdown_signal() {
    // Wait for the CTRL+C signal
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let args = Args::parse();

    let mut patch_provider = PatchProvider::new(args.dir.clone()).expect("Given patch location is not a directory.");
    patch_provider.load_patches().expect("Could not load patches.");

    info!("Loaded {} patches.", patch_provider.get_patch_count());
    let patch_ref = Arc::new(RwLock::new(patch_provider));

    let (tx, rx) = channel();
    let mut watcher = watcher(tx, Duration::from_secs(5)).expect("Could not setup file watcher for patch location.");
    watcher.watch(args.dir, RecursiveMode::Recursive).expect("Could not create a watcher on patch path.");

    let patcher_watch_ref = patch_ref.clone();
    tokio::spawn(async move {
        loop {
            match rx.recv() {
                Ok(res) => {
                    match res {
                        DebouncedEvent::Chmod(_) | DebouncedEvent::Write(_) | DebouncedEvent::NoticeWrite(_) => {
                            // we don't care about chmod or writing for our purposes
                        },
                        _ => {
                            debug!("Detected file system change, refreshing patches.");
                            let mut patcher = patcher_watch_ref.write().unwrap();
                            patcher.load_patches().unwrap();
                        }
                    }
                },
                Err(_) => break
            }
        }
    });

    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));

    let make_svc = make_service_fn(move |_| {
        let patch_ref = patch_ref.clone();
        async {
            Ok::<_, anyhow::Error>(service_fn(move |req| {
                handler::serve(req, patch_ref.clone())
            }))
        }
    });
    info!("Server listening on port {}.", args.port);

    let server = Server::bind(&addr).serve(make_svc);
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    if let Err(e) = graceful.await {
        error!("server error: {}", e);
    }
}
