//! Provider Contract Conformance Tests

use custos_integrations::cloud::fake::FakeProvider;
use custos_integrations::assert_provider_conformance;

#[tokio::test]
async fn test_fake_provider_conformance() {
    let provider = FakeProvider::new("conformance-fake");
    assert_provider_conformance(&provider).await;
}
