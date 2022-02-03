use crate::PatchProvider;
use hyper::{Body, Request, Response, StatusCode};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use xz2::stream::{LzmaOptions, Stream};
use anyhow::Result;

const LZMA_DICT_SIZE: u32 = 33554432;
const HEADER_SIZE_OFFSET: usize = 10;

pub async fn serve(
    req: Request<Body>,
    patch_provider_lock: Arc<RwLock<PatchProvider>>,
) -> anyhow::Result<Response<Body>> {
    let latest_version = match get_matching_patch_location(&req, patch_provider_lock) {
        Some(patch) => patch,
        _ => return Ok(Response::builder().status(StatusCode::NOT_FOUND).body(Body::empty())?)
    };

    let mut file = match File::open(latest_version) {
        Ok(f) => f,
        _ => {
            return Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(Body::empty())?)
        }
    };

    let mut content = Vec::new();
    let size: u64 = file.read_to_end(&mut content)? as u64;
    let mut compressed = compress(&mut content)?;
    fix_header_size(size, &mut compressed);

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(compressed))?)
}

fn get_matching_patch_location(req: &Request<Body>, patch_provider_lock: Arc<RwLock<PatchProvider>>) -> Option<PathBuf> {
    let patch_provider = patch_provider_lock.read().unwrap();
    let url_path = req.uri().path().trim_start_matches('/');
    let path = Path::new(url_path);

    return patch_provider.get_latest_version(path);
}

fn compress(content: &Vec<u8>) -> Result<Vec<u8>> {
    let lzma_stream = Stream::new_lzma_encoder(
        LzmaOptions::new_preset(6)?
            .dict_size(LZMA_DICT_SIZE),
    )?;
    let mut encoder =
        xz2::read::XzEncoder::new_stream(BufReader::new(content.as_slice()), lzma_stream);
    let mut compressed = Vec::new();
    let _ = encoder.read_to_end(&mut compressed);
    Ok(compressed)
}

fn fix_header_size(size: u64, compressed: &mut Vec<u8>) {
    let size_bytes = size.to_le_bytes();
    for i in (0..5).rev() {
        compressed.insert(0, size_bytes[i]);
    }

    // We also have to fix the size in the LZMA header because for some reason it is set to `-1` even
    // though we give it a fixed size buffer ...
    for i in 0..8 {
        compressed[HEADER_SIZE_OFFSET + i] = size_bytes[i];
    }
}
