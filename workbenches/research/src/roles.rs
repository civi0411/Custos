//! Research Worker Roles

use custos_core::contracts::pack::WorkerRole;

pub fn research_roles() -> Vec<WorkerRole> {
    vec![
        WorkerRole::new(
            "literature_scout",
            "research",
            "Literature Scout",
            "Academic paper retriever, PDF parser, and metadata extractor across ArXiv, CrossRef, and PubMed",
            "You are an academic literature researcher. Your objective is to discover relevant publications, fetch papers, extract abstracts, and map authors and publication dates without hallucinating non-existent papers.",
            10_000,
            &["http_fetch", "doi_verify", "file_read"],
            true,
        ),
        WorkerRole::new(
            "citation_oracle",
            "research",
            "Citation Oracle",
            "Factual and atomic claim extractor that verifies text support against ground-truth paper passages",
            "You are a rigorous scientific fact-checker. Deconstruct research findings into atomic proposition claims, anchor them to exact page/paragraph passages, and detect contradictory claims.",
            8_000,
            &["doi_verify", "claim_extract", "file_read"],
            true,
        ),
        WorkerRole::new(
            "dataset_auditor",
            "research",
            "Dataset Auditor",
            "Data volume inspector that validates splits, schema integrity, and checks for test-set leakage",
            "You are a data integrity and evaluation auditor. Inspect datasets, verify split distributions, check for sample leakage between train and test, and produce canonical DatasetCards.",
            15_000,
            &["file_read", "lexical_search"],
            true,
        ),
        WorkerRole::new(
            "experiment_worker",
            "research",
            "Experiment Worker",
            "Jupyter notebook and Python execution coordinator for training and benchmark reproduction",
            "You are an experimental machine learning engineer. Execute reproducible benchmarks, log metric curves, save artifact checkpoints, and generate figures with traceable seeds and configs.",
            25_000,
            &["file_read"],
            false,
        ),
    ]
}
