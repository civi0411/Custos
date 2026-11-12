use crate::api::*;

impl LocalApiDispatcher {
    pub(crate) async fn handle_research_recipes(&self, req: ApiRequest) -> ApiResponse {
        let research = match self.research_repo() {
            Some(r) => r,
            None => return ApiResponse::error(req.id, "ResearchRepository not configured"),
        };

        match req.method.as_str() {
            "v1.research.recipes.list" => match research.list_recipes() {
                Ok(list) => match serde_json::to_value(&list) {
                    Ok(val) => ApiResponse::success(req.id, val),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                },
                Err(e) => ApiResponse::error(req.id, e.to_string()),
            },
            "v1.research.recipes.save" => {
                let recipe: custos_domain::Recipe = match serde_json::from_value(req.params) {
                    Ok(r) => r,
                    Err(e) => {
                        return ApiResponse::error(req.id, format!("Invalid recipe payload: {e}"))
                    }
                };
                match research.save_recipe(&recipe) {
                    Ok(()) => ApiResponse::success(
                        req.id,
                        serde_json::json!({ "saved": true, "id": recipe.id }),
                    ),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            "v1.research.recipes.get" => {
                let recipe_id = match req.params.get("recipe_id").and_then(|v| v.as_str()) {
                    Some(id) => id,
                    None => return ApiResponse::error(req.id, "Missing recipe_id parameter"),
                };
                match research.get_recipe(recipe_id) {
                    Ok(Some(recipe)) => match serde_json::to_value(&recipe) {
                        Ok(val) => ApiResponse::success(req.id, val),
                        Err(e) => ApiResponse::error(req.id, e.to_string()),
                    },
                    Ok(None) => ApiResponse::error(req.id, format!("Recipe {recipe_id} not found")),
                    Err(e) => ApiResponse::error(req.id, e.to_string()),
                }
            }
            unknown => ApiResponse::error(req.id, format!("Unknown research recipes method: {unknown}")),
        }
    }
}
