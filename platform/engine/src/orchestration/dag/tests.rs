use super::*;

#[test]
fn test_valid_dag_topological_waves() {
    // Diamond Graph:
    //      A
    //     / \
    //    B   C
    //     \ /
    //      D
    let mut graph = DagGraph::new("Diamond Test");
    graph
        .add_node(DagNode::new(
            "A",
            "Root",
            "read",
            serde_json::json!({}),
            vec![],
        ))
        .unwrap();
    graph
        .add_node(DagNode::new(
            "B",
            "Branch B",
            "process",
            serde_json::json!({}),
            vec!["A".into()],
        ))
        .unwrap();
    graph
        .add_node(DagNode::new(
            "C",
            "Branch C",
            "process",
            serde_json::json!({}),
            vec!["A".into()],
        ))
        .unwrap();
    graph
        .add_node(DagNode::new(
            "D",
            "Join D",
            "merge",
            serde_json::json!({}),
            vec!["B".into(), "C".into()],
        ))
        .unwrap();

    assert!(graph.validate_dag().is_ok());

    let waves = graph.compute_execution_waves().unwrap();
    assert_eq!(waves.len(), 3);
    assert_eq!(waves[0].node_ids, vec!["A"]);
    assert_eq!(waves[1].node_ids, vec!["B", "C"]);
    assert_eq!(waves[2].node_ids, vec!["D"]);
}

#[test]
fn test_gate_5_cycle_detection_rejected() {
    // Cycle: A -> B -> C -> A
    let mut graph = DagGraph::new("Cycle Test");
    graph
        .add_node(DagNode::new(
            "A",
            "Node A",
            "exec",
            serde_json::json!({}),
            vec!["C".into()],
        ))
        .unwrap();
    graph
        .add_node(DagNode::new(
            "B",
            "Node B",
            "exec",
            serde_json::json!({}),
            vec!["A".into()],
        ))
        .unwrap();
    graph
        .add_node(DagNode::new(
            "C",
            "Node C",
            "exec",
            serde_json::json!({}),
            vec!["B".into()],
        ))
        .unwrap();

    let err = graph.validate_dag().unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
    assert!(err.to_string().contains("Cycle detected"));
}

#[test]
fn test_gate_5_self_dependency_rejected() {
    let mut graph = DagGraph::new("Self Loop Test");
    graph
        .add_node(DagNode::new(
            "A",
            "Node A",
            "exec",
            serde_json::json!({}),
            vec!["A".into()],
        ))
        .unwrap();

    let err = graph.validate_dag().unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
    assert!(err.to_string().contains("Self-dependency"));
}

#[test]
fn test_gate_5_missing_dependency_rejected() {
    let mut graph = DagGraph::new("Missing Dep Test");
    graph
        .add_node(DagNode::new(
            "A",
            "Node A",
            "exec",
            serde_json::json!({}),
            vec!["non_existent".into()],
        ))
        .unwrap();

    let err = graph.validate_dag().unwrap_err();
    assert!(matches!(err, DomainError::Validation(_)));
    assert!(err.to_string().contains("non-existent prerequisite"));
}

#[test]
fn test_failure_cascades_skip_to_downstream_dependents() {
    // A -> B -> D
    // A -> C -> E
    let mut graph = DagGraph::new("Cascade Test");
    graph
        .add_node(DagNode::new(
            "A",
            "A",
            "exec",
            serde_json::json!({}),
            vec![],
        ))
        .unwrap();
    graph
        .add_node(DagNode::new(
            "B",
            "B",
            "exec",
            serde_json::json!({}),
            vec!["A".into()],
        ))
        .unwrap();
    graph
        .add_node(DagNode::new(
            "C",
            "C",
            "exec",
            serde_json::json!({}),
            vec!["A".into()],
        ))
        .unwrap();
    graph
        .add_node(DagNode::new(
            "D",
            "D",
            "exec",
            serde_json::json!({}),
            vec!["B".into()],
        ))
        .unwrap();
    graph
        .add_node(DagNode::new(
            "E",
            "E",
            "exec",
            serde_json::json!({}),
            vec!["C".into()],
        ))
        .unwrap();

    // 1. Initial ready nodes is just A
    assert_eq!(graph.ready_nodes(), vec!["A"]);
    graph.mark_completed("A", None).unwrap();

    // 2. Ready nodes are now B and C
    assert_eq!(graph.ready_nodes(), vec!["B", "C"]);

    // 3. B fails!
    graph.mark_failed("B", "Network timeout").unwrap();
    assert_eq!(graph.get_node("B").unwrap().status, NodeStatus::Failed);

    // 4. D must be automatically marked as Skipped!
    assert_eq!(graph.get_node("D").unwrap().status, NodeStatus::Skipped);

    // 5. C is unaffected and still ready
    assert_eq!(graph.ready_nodes(), vec!["C"]);
    graph.mark_completed("C", None).unwrap();

    // 6. E is now ready
    assert_eq!(graph.ready_nodes(), vec!["E"]);
    graph.mark_completed("E", None).unwrap();

    // 7. Graph finished
    assert!(graph.is_finished());
    assert!(!graph.is_all_completed()); // because B failed and D skipped
}

#[test]
fn test_typed_node_ids() {
    let node = DagNode::new(
        "node_compile",
        "Compile target",
        "cargo_build",
        serde_json::json!({}),
        vec!["node_fetch".to_string(), "node_check".to_string()],
    );

    assert_eq!(node.typed_id().as_str(), "node_compile");
    let deps = node.typed_dependencies();
    assert_eq!(deps.len(), 2);
    assert_eq!(deps[0].as_str(), "node_fetch");
    assert_eq!(deps[1].as_str(), "node_check");
}
