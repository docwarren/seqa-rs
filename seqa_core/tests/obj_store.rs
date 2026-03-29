use seqa_core::stores::StoreService;

#[tokio::test]
async fn obj_store_s3() {
    
    let s3_path = "s3://com.gmail.docarw/test.txt";
    let store = StoreService::from_uri(s3_path).expect("Failed to create S3 store");
    let data = store.get_object(s3_path).await.expect("Failed to get data from S3 store");
    assert_eq!("Hello world".to_string(), String::from_utf8_lossy(&data));
}

#[tokio::test]
async fn obj_store_gcs() {
    let gcs_path = "gs://genre_test_bucket/test.txt";
    let store = StoreService::from_uri(gcs_path).expect("Failed to create GCS store");
    let data = store.get_object(gcs_path).await.expect("Failed to get data from GCS store");
    assert_eq!("Hello world".to_string(), String::from_utf8_lossy(&data));
}

#[tokio::test]
async fn obj_store_azure() {
    let azure_path = "az://genreblobs/genre-test-data/test.txt";
    let store = StoreService::from_uri(azure_path).expect("Failed to create Azure store");
    let data = store.get_object(azure_path).await.expect("Failed to get data from Azure store");
    assert_eq!("Hello world".to_string(), String::from_utf8_lossy(&data));
}

#[tokio::test]
async fn obj_store_http() {
    let http_path = "https://s3.us-west-1.amazonaws.com/com.gmail.docarw/test.txt";
    let store = StoreService::from_uri(http_path).expect("Failed to create HTTP store");
    let data = store.get_object(http_path).await.expect("Failed to get data from HTTP store");
    assert_eq!("Hello world".to_string(), String::from_utf8_lossy(&data));
}

#[tokio::test]
async fn obj_store_s3_put() {
    let path = "s3://com.gmail.docarw/test_data/put_test.txt";
    let store = StoreService::from_uri(path).expect("Failed to create S3 store");
    let data = b"hello world";
    store.put_object(path, data).await.expect("Failed to put object");
    let object = store.get_object(path).await.expect("Failed to get object");
    assert_eq!(object, data);
}

#[tokio::test]
async fn obj_store_get_file_size() {
    let path = "s3://com.gmail.docarw/test_data/density.bw";
    let store = StoreService::from_uri(path).expect("Failed to create S3 store");
    let size = store.get_file_size(path).await.expect("Failed to get file size");
    let expected = 2_600_000;
    assert!(size > expected, "Expected file size > {}; got {}", expected, size);
    assert!(size < 3_000_000, "Expected file size < 3MB; got {}", size);
}

#[test]
fn get_canonical_path_s3() {
    let path = "s3://com.gmail.docarw/test_data/density.bw";
    let canonical = StoreService::get_canonical_path(path).expect("Failed to get canonical path");
    assert_eq!(canonical.as_ref(), "test_data/density.bw");
}
