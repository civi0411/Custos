use super::*;

#[test]
fn test_split_think_blocks_extracts_inline_reasoning() {
    assert_eq!(
        split_think_blocks("<think>x</think>y"),
        ("y".to_string(), "x".to_string())
    );
}

#[test]
fn test_split_think_blocks_is_case_insensitive() {
    assert_eq!(
        split_think_blocks("<THINK>x</think>y"),
        ("y".to_string(), "x".to_string())
    );
}

#[test]
fn test_split_think_blocks_handles_multiple_blocks() {
    assert_eq!(
        split_think_blocks("<think>a</think>b<think>c</think>d"),
        ("bd".to_string(), "ac".to_string())
    );
}

#[test]
fn test_split_think_blocks_without_tags() {
    assert_eq!(
        split_think_blocks("plain content"),
        ("plain content".to_string(), String::new())
    );
}

#[test]
fn test_split_think_blocks_handles_attributes() {
    assert_eq!(
        split_think_blocks(r#"<think class="x">a</think>b"#),
        ("b".to_string(), "a".to_string())
    );
}

#[test]
fn test_split_think_blocks_handles_quoted_gt_in_self_closing_attributes() {
    for input in [
        r#"<think data="a>b"/>Visible"#,
        "<think data='a>b'/>Visible",
    ] {
        assert_eq!(
            split_think_blocks(input),
            ("Visible".to_string(), String::new()),
            "mismatch for {input:?}"
        );
    }
}

#[test]
fn test_split_think_blocks_handles_quoted_gt_in_open_attributes() {
    assert_eq!(
        split_think_blocks(r#"<think data="a>b">Hidden</think>Visible"#),
        ("Visible".to_string(), "Hidden".to_string())
    );
}

#[test]
fn test_split_think_blocks_handles_thinking_variant() {
    assert_eq!(
        split_think_blocks("<thinking>a</thinking>b"),
        ("b".to_string(), "a".to_string())
    );
}

#[test]
fn test_think_filter_streaming_across_partial_tags() {
    let mut filter = ThinkFilter::new();
    let mut out = FilterOut::default();

    for chunk in ["<thi", "nk>x</thi", "nk>y"] {
        let partial = filter.push(chunk);
        out.content.push_str(&partial.content);
        out.thinking.push_str(&partial.thinking);
    }

    let final_out = filter.finish();
    out.content.push_str(&final_out.content);
    out.thinking.push_str(&final_out.thinking);

    assert_eq!(out.content, "y");
    assert_eq!(out.thinking, "x");
}

#[test]
fn test_think_filter_preserves_non_think_tags() {
    let mut filter = ThinkFilter::new();
    let mut out = filter.push("<table>");
    let final_out = filter.finish();
    out.content.push_str(&final_out.content);
    out.thinking.push_str(&final_out.thinking);

    assert_eq!(out.content, "<table>");
    assert!(out.thinking.is_empty());
}

#[test]
fn test_think_filter_finish_treats_unterminated_think_as_thinking() {
    let mut filter = ThinkFilter::new();
    let mut out = filter.push("<think>unfinished");
    let final_out = filter.finish();
    out.content.push_str(&final_out.content);
    out.thinking.push_str(&final_out.thinking);

    assert!(out.content.is_empty());
    assert_eq!(out.thinking, "unfinished");
}

#[test]
fn test_think_filter_tracks_generation_prompt_open_block() {
    let mut filter = ThinkFilter::new();
    let _ = filter.push("<|assistant|><think>\n");
    let mut out = filter.push("hidden reasoning</think>visible answer");
    let final_out = filter.finish();
    out.content.push_str(&final_out.content);
    out.thinking.push_str(&final_out.thinking);

    assert_eq!(out.content, "visible answer");
    assert_eq!(out.thinking, "hidden reasoning");
}

#[test]
fn test_think_filter_preserves_tags_with_think_prefix() {
    for input in [
        "<thinking-mode>hello</thinking-mode>",
        "<thinking123>payload</thinking123>",
        "<thinker>note</thinker>",
    ] {
        let mut filter = ThinkFilter::new();
        let mut out = filter.push(input);
        let final_out = filter.finish();
        out.content.push_str(&final_out.content);
        out.thinking.push_str(&final_out.thinking);

        assert_eq!(out.content, input, "content mismatch for {input:?}");
        assert!(
            out.thinking.is_empty(),
            "unexpected thinking for {input:?}: {:?}",
            out.thinking
        );
    }
}

#[test]
fn test_think_filter_accepts_think_with_attributes() {
    let mut filter = ThinkFilter::new();
    let mut out = filter.push("<think data-source=\"x\">hidden</think>visible");
    let final_out = filter.finish();
    out.content.push_str(&final_out.content);
    out.thinking.push_str(&final_out.thinking);

    assert_eq!(out.content, "visible");
    assert_eq!(out.thinking, "hidden");
}

#[test]
fn test_think_filter_treats_self_closing_as_noop() {
    // `<think/>` carries no reasoning payload. It must not flip the filter
    // into "inside_think" mode, and the tag itself must not leak into
    // visible content.
    for input in [
        "before <think/> after",
        "before <think /> after",
        "before <thinking/> after",
        "before <think data-source=\"x\"/> after",
    ] {
        let mut filter = ThinkFilter::new();
        let mut out = filter.push(input);
        let final_out = filter.finish();
        out.content.push_str(&final_out.content);
        out.thinking.push_str(&final_out.thinking);

        assert_eq!(
            out.content, "before  after",
            "content mismatch for {input:?}"
        );
        assert!(
            out.thinking.is_empty(),
            "unexpected thinking for {input:?}: {:?}",
            out.thinking
        );
    }
}

#[test]
fn test_think_filter_self_closing_does_not_swallow_following_content() {
    // Regression: a self-closing `<think/>` used to be classified as an
    // Open tag, which incremented think_depth and routed everything after
    // it into the thinking bucket for the rest of the stream.
    let mut filter = ThinkFilter::new();
    let mut out = filter.push("<think/>visible chunk 1");
    let final_out = filter.push("visible chunk 2");
    let tail_out = filter.finish();
    out.content.push_str(&final_out.content);
    out.thinking.push_str(&final_out.thinking);
    out.content.push_str(&tail_out.content);
    out.thinking.push_str(&tail_out.thinking);

    assert_eq!(out.content, "visible chunk 1visible chunk 2");
    assert!(out.thinking.is_empty());
}

#[test]
fn test_think_filter_streaming_across_self_closing_boundary() {
    // Regression: a chunk boundary between `<think` and `>` in a
    // self-closing `<think/>` used to fall out of the partial-tag regex
    // (which only allowed `<think<ws>...`), so the `<think/` prefix leaked
    // into visible content before the `>` arrived.
    for (a, b) in [
        ("before <think/", "> after"),
        ("before <thinking/", "> after"),
        ("head <think ", "/> tail"),
    ] {
        let mut filter = ThinkFilter::new();
        let mut out = filter.push(a);
        let second = filter.push(b);
        let final_out = filter.finish();
        out.content.push_str(&second.content);
        out.content.push_str(&final_out.content);
        out.thinking.push_str(&second.thinking);
        out.thinking.push_str(&final_out.thinking);

        assert!(
            !out.content.contains('<'),
            "partial tag leaked into content for ({a:?}, {b:?}): {:?}",
            out.content
        );
        assert!(
            out.thinking.is_empty(),
            "unexpected thinking for ({a:?}, {b:?}): {:?}",
            out.thinking
        );
    }
}

#[test]
fn test_think_filter_streaming_across_quoted_attribute_boundary() {
    let mut filter = ThinkFilter::new();
    let mut out = filter.push(r#"<think data="a>b"#);
    assert!(out.content.is_empty());
    assert!(out.thinking.is_empty());

    let second = filter.push(r#""/>Visible"#);
    let final_out = filter.finish();
    out.content.push_str(&second.content);
    out.content.push_str(&final_out.content);
    out.thinking.push_str(&second.thinking);
    out.thinking.push_str(&final_out.thinking);

    assert_eq!(out.content, "Visible");
    assert!(out.thinking.is_empty());
}

#[test]
fn test_think_filter_self_closing_inside_open_block_closes_nothing() {
    // `<think/>` inside an open `<think>` block is still a no-op: depth
    // should stay at 1 until the real `</think>` arrives.
    let mut filter = ThinkFilter::new();
    let mut out = filter.push("before <think>hidden1 <think/> hidden2</think>visible");
    let final_out = filter.finish();
    out.content.push_str(&final_out.content);
    out.thinking.push_str(&final_out.thinking);

    assert_eq!(out.content, "before visible");
    assert_eq!(out.thinking, "hidden1  hidden2");
}

#[test]
fn test_think_filter_bounds_unterminated_quoted_tag_candidates() {
    for quote in ['"', '\''] {
        let payload = format!(
            "<think data={quote}{}",
            "a".repeat(MAX_BUFFERED_THINK_TAG_BYTES * 2)
        );
        let mut filter = ThinkFilter::new();
        let mut content = String::new();

        for chunk in payload.as_bytes().chunks(17) {
            let out = filter.push(std::str::from_utf8(chunk).unwrap());
            content.push_str(&out.content);
            assert!(filter.buffer.len() <= MAX_BUFFERED_THINK_TAG_BYTES);
        }

        content.push_str(&filter.finish().content);
        assert_eq!(content, payload);
    }
}

#[test]
fn test_think_filter_bounds_partial_close_tag_candidates() {
    let payload = format!("</think{}", " ".repeat(MAX_BUFFERED_THINK_TAG_BYTES * 2));
    let mut filter = ThinkFilter::new();
    let mut content = String::new();

    for chunk in payload.as_bytes().chunks(19) {
        let out = filter.push(std::str::from_utf8(chunk).unwrap());
        content.push_str(&out.content);
        assert!(filter.buffer.len() <= MAX_BUFFERED_THINK_TAG_BYTES);
    }

    content.push_str(&filter.finish().content);
    assert_eq!(content, payload);
}

#[test]
fn test_think_filter_releases_single_oversized_candidate_allocation() {
    let payload = format!(
        "<think data=\"{}",
        "a".repeat(MAX_BUFFERED_THINK_TAG_BYTES * 16)
    );
    let mut filter = ThinkFilter::new();
    let out = filter.push(&payload);

    assert_eq!(out.content, payload);
    assert!(out.thinking.is_empty());
    assert!(filter.buffer.is_empty());
    assert_eq!(filter.buffer.capacity(), 0);
}

#[test]
fn test_think_filter_preserves_and_releases_completed_oversized_tag() {
    let payload = format!(
        "<think data=\"{}\"/>",
        "a".repeat(MAX_BUFFERED_THINK_TAG_BYTES * 16)
    );
    let mut filter = ThinkFilter::new();
    let out = filter.push(&payload);

    assert_eq!(out.content, payload);
    assert!(out.thinking.is_empty());
    assert!(filter.buffer.is_empty());
    assert_eq!(filter.buffer.capacity(), 0);
}

#[test]
fn test_think_filter_releases_oversized_prefix_capacity() {
    let prefix = "a".repeat(MAX_BUFFERED_THINK_TAG_BYTES * 16);
    let payload = format!("{prefix}<thi");
    let mut filter = ThinkFilter::new();
    let out = filter.push(&payload);

    assert_eq!(out.content, prefix);
    assert!(out.thinking.is_empty());
    assert_eq!(filter.buffer, "<thi");
    assert!(filter.buffer.capacity() <= MAX_BUFFERED_THINK_TAG_BYTES);
}

#[test]
fn test_think_filter_accepts_bounded_streamed_attributes() {
    let prefix = "<think data=\"";
    let attribute = "a".repeat(MAX_BUFFERED_THINK_TAG_BYTES - prefix.len() - 2);
    let mut filter = ThinkFilter::new();
    let first = filter.push(&format!("{prefix}{attribute}"));

    assert!(first.content.is_empty());
    assert!(first.thinking.is_empty());
    assert_eq!(filter.buffer.len(), MAX_BUFFERED_THINK_TAG_BYTES - 2);

    let second = filter.push("\">hidden</think>visible");
    let final_out = filter.finish();
    assert_eq!(second.content, "visible");
    assert_eq!(second.thinking, "hidden");
    assert_eq!(final_out, FilterOut::default());
}

