//! Provider Contract Conformance Tests

use custos_adapter_provider_fake::FakeProvider;
use custos_provider_sdk::assert_provider_conformance;

#[tokio::test]
async fn test_fake_provider_conformance() {
    let provider = FakeProvider::new("conformance-fake");
    assert_provider_conformance(&provider).await;
}
