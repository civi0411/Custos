//! Lazy registry holding workbench-contributed supervision profiles.
//! Unenabled workbenches allocate zero profiles in memory.

use custos_domain::supervision::SupervisionProfile;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug)]
pub struct SupervisionProfileRegistry {
    default_profile: SupervisionProfile,
    profiles: RwLock<HashMap<String, SupervisionProfile>>,
}

impl Default for SupervisionProfileRegistry {
    fn default() -> Self {
        Self {
            default_profile: SupervisionProfile::default(),
            profiles: RwLock::new(HashMap::new()),
        }
    }
}

impl SupervisionProfileRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_profile(&self, workbench_id: &str, profile: SupervisionProfile) {
        if let Ok(mut guard) = self.profiles.write() {
            guard.insert(workbench_id.to_ascii_lowercase(), profile);
        }
    }

    pub fn unregister_profile(&self, workbench_id: &str) {
        if let Ok(mut guard) = self.profiles.write() {
            guard.remove(&workbench_id.to_ascii_lowercase());
        }
    }

    pub fn resolve(&self, workbench_id: Option<&str>) -> SupervisionProfile {
        if let Some(id) = workbench_id {
            let key = id.to_ascii_lowercase();
            if let Ok(guard) = self.profiles.read() {
                if let Some(profile) = guard.get(&key) {
                    return profile.clone();
                }
            }
        }
        self.default_profile.clone()
    }

    pub fn list_profiles(&self) -> Vec<SupervisionProfile> {
        let mut all = vec![self.default_profile.clone()];
        if let Ok(guard) = self.profiles.read() {
            let mut entries: Vec<_> = guard.values().cloned().collect();
            entries.sort_by(|a, b| a.profile_id.cmp(&b.profile_id));
            all.extend(entries);
        }
        all
    }

    pub fn profile_ids(&self) -> Vec<String> {
        self.list_profiles()
            .into_iter()
            .map(|p| p.profile_id)
            .collect()
    }
}

