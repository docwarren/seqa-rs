#[tokio::test]
async fn obj_store_local() {
    use soma_core::stores::StoreService;

    let local_path = format!("file://{}/tests/data/test.txt", env!("CARGO_MANIFEST_DIR"));
    let store = StoreService::from_uri(&local_path).expect("Failed to create local store");
    let data = store.get_object(&local_path).await.expect("Failed to get data from Local store");
    assert_eq!("Hello world".to_string(), String::from_utf8_lossy(&data));
}
