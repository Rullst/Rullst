use rullst::storage::{LocalDriver, StorageDriver};
use std::env;

#[tokio::test]
async fn test_local_driver() {
    let temp_dir = env::temp_dir().join("rullst_storage_test");
    let driver = LocalDriver::new(temp_dir.clone());

    // Clean up before test just in case
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;

    // Test put
    let payload = b"hello storage";
    driver.put("test_file.txt", payload).await.unwrap();

    // Test exists
    assert!(driver.exists("test_file.txt").await.unwrap());

    // Test get
    let read_payload = driver.get("test_file.txt").await.unwrap();
    assert_eq!(read_payload, payload);

    // Test url
    let url = driver.url("test_file.txt").await.unwrap();
    assert_eq!(url, "/storage/test_file.txt");

    // Test path traversal protection
    assert!(driver.put("../outside.txt", b"hack").await.is_err());
    assert!(driver.get("/etc/passwd").await.is_err());
    assert!(driver.get("C:\\Windows\\system32").await.is_err());

    // Test delete
    driver.delete("test_file.txt").await.unwrap();
    assert!(!driver.exists("test_file.txt").await.unwrap());

    // Clean up
    let _ = tokio::fs::remove_dir_all(&temp_dir).await;
}
