use crate::api::search_options::SearchOptions;
use crate::codecs::bgzip;
use crate::indexes::chunk::Chunk;
use crate::stores::StoreService;

use futures::TryStreamExt;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchError {
    #[error("Failed to process data: {0}")]
    DataProcessingError(String),

    #[error("Store Error: {0}")]
    StoreError(#[from] crate::stores::error::StoreError),

    #[error("Object Store Error: {0}")]
    ObjectStoreError(#[from] object_store::Error),

    #[error("BgZip Error: {0}")]
    BgZipError(#[from] bgzip::BgZipError),
}

pub async fn chunk_to_stream(
    chunk: &Chunk,
    store_service: &StoreService,
    path: &object_store::path::Path,

) -> Result<impl futures::Stream<Item = Result<Vec<u8>, object_store::Error>>, object_store::Error> {

    let range = chunk.to_range();

    let get_opts = object_store::GetOptions {
        range: Some(object_store::GetRange::Bounded(range.clone())),
        ..Default::default()
    };

    let store = store_service
        .get_store();

    let stream = store
        .get_opts(path, get_opts)
        .await?
        .into_stream()
        .map_ok(|bytes| bytes.to_vec());

    Ok(stream.map_ok(|get_result| get_result.to_vec()))
}

/// Streams data from the store service and processes it into strings based on the provided closure.
/// # Arguments:
/// * `options` - A `SearchOptions` struct containing the file path and other search parameters.
/// * `chunks` - A slice of `Chunk` objects representing the chunks to be processed.
/// * `data_to_string_closure` - A closure that takes a vector of bytes and returns
///   an optional tuple containing a boolean indicating if the end of the search range has been reached
///   and a vector of strings with the processed lines.
/// # Returns:
/// * A Result containing a vector of strings with the processed lines, or an error message if the processing fails.
pub async fn stream_data_to_strings(
    options: &SearchOptions,
    start_lines: Vec<String>,
    chunks: &[Chunk],
    data_to_string_closure: impl Fn(&Vec<u8>) -> Result<(bool, Vec<String>), String>,
) -> Result<Vec<String>, SearchError> {
    let store_service = StoreService::from_uri(&options.file_path)?;
    let path = StoreService::get_canonical_path(&options.file_path)?;

    let mut overlapping_lines = Vec::new();
    overlapping_lines.extend(start_lines);

    for chunk in chunks.iter() {
        let mut bytes = Vec::new();
        let mut decompressed_slices: Vec<u8> = Vec::new();

        let mut stream = chunk_to_stream(chunk, &store_service, &path).await?;
        let mut decompressed_start_byte = chunk.begin_vp.decompressed_offset as usize;

        while let Ok(Some(byte_chunk)) = stream.try_next().await {
            bytes.extend(byte_chunk.into_iter());

            let block_sizes = bgzip::from_bytes(&bytes)?;

            if !block_sizes.is_empty() {
                let tail_start = block_sizes.iter().map(|b| b).sum::<usize>();
                let remaining_bytes = bytes.split_off(tail_start);
                let decompressed_bytes = bgzip::decompress(&block_sizes, &bytes)?;
                bytes = remaining_bytes; // Adjust bytes to remove processed data

                let decompressed_slice = &decompressed_bytes[decompressed_start_byte..];
                decompressed_slices.extend_from_slice(decompressed_slice);
                
                if let Ok((end, lines)) = data_to_string_closure(&decompressed_slices) {
                    decompressed_start_byte = 0;
                    decompressed_slices.clear();
                    overlapping_lines.extend(lines);
                    if end {
                        break; // Features no longer overlap the search range
                    }
                }
            }
        }
    }
    Ok(overlapping_lines)
}

/// Fetches chunks from the store service based on the provided search options and chunks.
/// Returns a vector of tuples containing the chunk and a join handle for the asynchronous task.
/// # Arguments:
/// * `options` - A `SearchOptions` struct containing the file path and other search parameters.
/// * `chunks` - A slice of `Chunk` objects representing the chunks to be fetched.
/// # Returns:
/// * A vector of tuples where each tuple contains a `Chunk` and a `JoinHandle` that resolves to the fetched data.
pub async fn init_fetch_handles(
    options: &SearchOptions,
    chunks: &[Chunk],
) -> Result<Vec<(Chunk, tokio::task::JoinHandle<Vec<u8>>)>, SearchError> {
    let mut chunk_handles = Vec::new();

    for chunk in chunks.iter() {
        let range = chunk.to_range();
        let file_path = options.file_path.clone();
        let chunk_clone = chunk.clone(); // Clone the chunk before moving it into the async block

        let handle = tokio::spawn(async move {

            match StoreService::from_uri(&file_path) {
                Ok(store_service) => match store_service.get_range(&file_path, range).await {
                    Ok(data) => data,
                    Err(e) => {
                        eprintln!("Error fetching range for chunk {:?}: {}", chunk_clone, e);
                        vec![]
                    }
                },
                Err(e) => {
                    eprintln!("Error creating store service: {}", e);
                    vec![]
                }
            }
        });

        chunk_handles.push((chunk.clone(), handle));
    }
    Ok(chunk_handles)
}
/// Processes the fetched chunks by awaiting their completion,
/// decompressing the data, and extracting the relevant slice based on the chunk's decompressed offset.
/// # Arguments:
/// * `chunk_handles` - A vector of tuples containing the chunk and its associated join handle.
/// # Returns:
/// * A Result containing a vector of vectors of bytes, where each vector corresponds to a chunk's decompressed data.
///   If an error occurs during decompression, it returns an error message.
pub async fn join_fetch_handles(
    chunk_handles: Vec<(Chunk, tokio::task::JoinHandle<Vec<u8>>)>,
) -> Result<Vec<Vec<u8>>, SearchError> {
    let mut raw_data = Vec::new();

    for (chunk, handle) in chunk_handles {
        let compressed_bytes = handle.await.map_err(|_| SearchError::DataProcessingError(format!("Failed to fetch chunk: {:?}", chunk)))?;
        let block_sizes = bgzip::from_bytes(&compressed_bytes)?;
        let decompressed_bytes = bgzip::decompress(&block_sizes, &compressed_bytes)?;
        let decompressed_start_byte = chunk.begin_vp.decompressed_offset as usize;
        let decompressed_slice = &decompressed_bytes[decompressed_start_byte..];
        raw_data.push(decompressed_slice.to_vec());
    }
    Ok(raw_data)
}
