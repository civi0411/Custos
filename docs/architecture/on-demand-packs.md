# Dynamic Pack Registration & On-Demand Lifecycle

> **Status:** Canonical SSOT  
> **Crates:** `custos-packs`, `custos-daemon`, `custos-sdk`

---

## 1. Motivation & Paradigm

In earlier iterations, Domain Packs were statically compiled into the composition root without dynamic query or modular lifecycle methods. The On-Demand Pack Registry enables:
1. **Dynamic Registration / Unregistration:** Packs can be loaded, swapped, or disabled at runtime via `DomainPackRegistry::register()` and `unregister()`.
2. **Modular Manifest Queries:** Clients inspect capabilities and metadata without instantiating heavy subagent runtimes.
3. **Verifier Oracle Discovery:** All independent proof-closure oracles across registered packs are aggregated via `list_all_verifiers()`.

---

## 2. Registry API Surface

```rust
pub struct DomainPackRegistry {
    packs: HashMap<String, Arc<dyn DomainPack>>,
}

impl DomainPackRegistry {
    pub fn new() -> Self;
    pub fn register(&mut self, pack: Arc<dyn DomainPack>);
    pub fn unregister(&mut self, pack_id: &str) -> Option<Arc<dyn DomainPack>>;
    pub fn has_pack(&self, pack_id: &str) -> bool;
    pub fn get_pack(&self, pack_id: &str) -> Option<Arc<dyn DomainPack>>;
    pub fn get_pack_for_lens(&self, lens: WorkbenchLens) -> Option<Arc<dyn DomainPack>>;
    pub fn get_pack_manifest(&self, pack_id: &str) -> Option<DomainPackManifest>;
    pub fn list_packs(&self) -> Vec<DomainPackManifest>;
    pub fn list_all_jobs(&self) -> Vec<WorkbenchJob>;
    pub fn list_all_roles(&self) -> Vec<WorkerRole>;
    pub fn list_all_verifiers(&self) -> Vec<Arc<dyn PackVerifier>>;
}
```

---

## 3. Local API Integration

The daemon exposes on-demand pack capabilities via standard local RPC endpoints:
- `v1.packs.manifests.list` (`METHOD_PACKS_MANIFESTS_LIST`): Returns array of all active `DomainPackManifest` items.
- `v1.packs.jobs.list` (`METHOD_PACKS_JOBS_LIST`): Lists all 24 standardized Workbench Jobs, filtered by `lens` or `pack_id`.
- `v1.packs.jobs.get` (`METHOD_PACKS_JOBS_GET`): Fetches detailed job catalog item including `required_inputs`, `expected_artifacts`, and `acceptance_rubric`.
- `v1.packs.roles.list` (`METHOD_PACKS_ROLES_LIST`): Enumerates specialized worker roles (16 across 4 packs).
- `v1.packs.jobs.run` (`METHOD_PACKS_JOBS_RUN`): Spawns a tracked task bound to the requested job and primary role.
