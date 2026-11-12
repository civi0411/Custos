use custos_domain::{Recipe, RecipeEnvironmentSpec};
use custos_persistence::SqliteTaskStore;

#[tokio::test]
async fn reviews_group_test_without_dispatcher() {
    let store = SqliteTaskStore::new_in_memory().unwrap();
    let repo = custos_workbench_research::ResearchRepository::from_raw(store.db().writer_arc());

    let initial = repo.list_recipes().unwrap();
    assert_eq!(initial.len(), 0);

    let mut recipe = Recipe::new(
        "Train Tokenizer",
        "python train.py",
        RecipeEnvironmentSpec::new(),
    )
    .with_description("Train BPE tokenizer");
    recipe.id = "recipe-1".to_string();

    repo.save_recipe(&recipe).unwrap();
    let fetched = repo.get_recipe("recipe-1").unwrap();
    assert!(fetched.is_some());
    assert_eq!(fetched.unwrap().name, "Train Tokenizer");
}
