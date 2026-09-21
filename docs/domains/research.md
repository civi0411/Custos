# Research Domain Pack

> **Status:** Canonical Baseline v4.0  
> **Source:** Part VI (§19) & Part VII (§55) Canonical Specification

The Research Domain Pack transforms open-ended academic inquiries and technical document synthesis into structured, verifiable evidence reports, eliminating hallucinated citations.

---

## 1. Research Execution Pipeline

```text
[ 1. Inquiry & Scope  ] ---> (Define hypothesis, scope parameters, target questions)
        |
[ 2. Source Discovery ] ---> (arXiv, Semantic Scholar, Web API, Zotero integrations)
        |
[ 3. Ingest & Hash    ] ---> (Parse and normalize PDF/HTML, store in content-addressed CAS)
        |
[ 4. Claim Extraction ] ---> (Model structured Claim-Evidence tuples)
        |
[ 5. Cross-Examination] ---> (Detect contradictions and unsupported assertions across sources)
        |
[ 6. Report Export    ] ---> (Emit Markdown report, Obsidian Vault with WikiLinks, BibTeX)
```

---

## 2. Claim-Evidence Schema

Every extracted research assertion is structured as an explicit typed schema:

```yaml
claim_id: "clm_01J8N89X..."
hypothesis_ref: "hyp_01"
statement: "Rust zero-cost abstractions provide memory safety with zero runtime GC overhead."
confidence: 0.98

evidence:
  - source_id: "src_arxiv_2402_12345"
    citation: "Matsakis et al., 2024"
    cas_hash: "sha256:7f83b1657ff1fc53b92dc18148a1d65dfc2d4b1fa3d677284addd200126d9069"
    exact_quote: "The ownership type system statically enforces single-writer or multiple-reader access."
    page_number: 4
    evidence_tier: "PEER_REVIEWED_PAPER"

counter_evidence: []
verification_status: "VERIFIED"
```

---

## 3. Obsidian Vault & Markdown Export

All completed research artifacts can be exported directly into a local **Obsidian Vault**:
- Concepts are bidirectionally linked via standard `[[WikiLinks]]`.
- Claims contain clickable references to local immutable PDF/text files stored in the CAS directory.
- Fully compatible with citation tools such as Zotero and BibTeX managers.
