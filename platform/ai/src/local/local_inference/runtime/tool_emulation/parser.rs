use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag};

#[allow(dead_code)]
pub(crate) enum EmulatorAction {
    Text(String),
    ShellCommand(String),
    ExecuteCode(String),
}

// pulldown-cmark parses complete documents, so fail closed once reparsing would become expensive.
#[allow(dead_code)]
const MAX_MARKDOWN_CONTEXT_BYTES: usize = 256 * 1024;
#[allow(dead_code)]
const MAX_MARKDOWN_PARSE_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum ParserState {
    Normal,
    InExecuteBlock { fence_len: usize },
}

#[allow(dead_code)]
pub(crate) struct StreamingEmulatorParser {
    buffer: String,
    document: String,
    markdown_context_available: bool,
    markdown_parse_bytes_remaining: usize,
    state: ParserState,
    code_mode_enabled: bool,
}

#[allow(dead_code)]
fn execute_fence_len(line: &str) -> Option<usize> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return None;
    }

    let rest = line.get(indent..)?;
    let fence_len = rest.bytes().take_while(|byte| *byte == b'`').count();
    if fence_len < 3 {
        return None;
    }

    rest.get(fence_len..)?
        .trim_matches([' ', '\t'])
        .eq("execute_typescript")
        .then_some(fence_len)
}

#[allow(dead_code)]
fn is_closing_fence(line: &str, minimum_len: usize) -> bool {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return false;
    }

    let Some(rest) = line.get(indent..) else {
        return false;
    };
    let fence_len = rest.bytes().take_while(|byte| *byte == b'`').count();
    fence_len >= minimum_len
        && rest
            .get(fence_len..)
            .is_some_and(|suffix| suffix.bytes().all(|byte| matches!(byte, b' ' | b'\t')))
}

#[allow(dead_code)]
fn is_top_level_execute_fence(markdown: &str, line_start: usize) -> bool {
    let mut depth = 0;
    for (event, range) in Parser::new(markdown).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0
                    && range.start == line_start
                    && matches!(
                        tag,
                        Tag::CodeBlock(CodeBlockKind::Fenced(ref info))
                            if info.as_ref() == "execute_typescript"
                    )
                {
                    return true;
                }
                depth += 1;
            }
            Event::End(_) => depth -= 1,
            _ => {}
        }
    }
    false
}

#[allow(dead_code)]
fn is_top_level_paragraph_line(markdown: &str, line_start: usize) -> bool {
    let mut depth = 0;
    for (event, range) in Parser::new(markdown).into_offset_iter() {
        match event {
            Event::Start(tag) => {
                if depth == 0
                    && matches!(tag, Tag::Paragraph)
                    && range.start <= line_start
                    && line_start < range.end
                {
                    return true;
                }
                depth += 1;
            }
            Event::End(_) => depth -= 1,
            _ => {}
        }
    }
    false
}

#[allow(dead_code)]
fn closing_fence_range(
    input: &str,
    minimum_len: usize,
    allow_end_of_stream: bool,
) -> Option<(usize, usize)> {
    let mut line_start = 0;
    loop {
        let remaining = input
            .get(line_start..)
            .expect("line start must be a character boundary");
        let newline_offset = remaining.find('\n');
        if newline_offset.is_none() && !allow_end_of_stream {
            return None;
        }
        let line_end = newline_offset
            .map(|offset| line_start + offset)
            .unwrap_or(input.len());
        if is_closing_fence(
            input
                .get(line_start..line_end)
                .expect("line range must be on character boundaries"),
            minimum_len,
        ) {
            let consumed = if line_end < input.len() {
                line_end + 1
            } else {
                line_end
            };
            return Some((line_start, consumed));
        }
        if line_end == input.len() {
            return None;
        }
        line_start = line_end + 1;
    }
}

#[allow(dead_code)]
impl StreamingEmulatorParser {
    pub(crate) fn new(code_mode_enabled: bool) -> Self {
        Self {
            buffer: String::new(),
            document: String::new(),
            markdown_context_available: true,
            markdown_parse_bytes_remaining: MAX_MARKDOWN_PARSE_BYTES,
            state: ParserState::Normal,
            code_mode_enabled,
        }
    }

    fn append_markdown_context(&mut self, chunk: &str) {
        if !self.markdown_context_available {
            return;
        }

        if self.document.len().saturating_add(chunk.len()) > MAX_MARKDOWN_CONTEXT_BYTES {
            self.disable_markdown_emulation();
        } else {
            self.document.push_str(chunk);
        }
    }

    fn consume_markdown_parse_budget(&mut self, bytes: usize) -> bool {
        if bytes > self.markdown_parse_bytes_remaining {
            self.disable_markdown_emulation();
            false
        } else {
            self.markdown_parse_bytes_remaining -= bytes;
            true
        }
    }

    fn markdown_matches(
        &mut self,
        range: Option<(usize, usize)>,
        predicate: fn(&str, usize) -> bool,
    ) -> bool {
        let Some((line_start, markdown_end)) = range else {
            return false;
        };
        if !self.consume_markdown_parse_budget(markdown_end) {
            return false;
        }

        let markdown = self
            .document
            .get(..markdown_end)
            .expect("markdown end must be a character boundary");
        predicate(markdown, line_start)
    }

    fn disable_markdown_emulation(&mut self) {
        self.document.clear();
        self.markdown_context_available = false;
        self.markdown_parse_bytes_remaining = 0;
    }

    pub(crate) fn process_chunk(&mut self, chunk: &str) -> Vec<EmulatorAction> {
        self.buffer.push_str(chunk);
        self.append_markdown_context(chunk);
        let mut results = Vec::new();

        loop {
            match self.state {
                ParserState::InExecuteBlock { fence_len } => {
                    let Some((closing_start, consumed)) =
                        closing_fence_range(&self.buffer, fence_len, false)
                    else {
                        break;
                    };
                    let code_end = closing_start
                        .checked_sub(1)
                        .filter(|index| self.buffer.as_bytes()[*index] == b'\n')
                        .unwrap_or(closing_start);
                    let code = self
                        .buffer
                        .get(..code_end)
                        .expect("code boundary must be a character boundary");
                    let code = code.strip_suffix('\r').unwrap_or(code).to_string();
                    self.buffer.replace_range(..consumed, "");
                    self.state = ParserState::Normal;
                    if !code.trim().is_empty() {
                        results.push(EmulatorAction::ExecuteCode(code));
                    }
                }
                ParserState::Normal => {
                    let Some(line_end) = self.buffer.find('\n') else {
                        break;
                    };
                    let markdown_range = self.markdown_context_available.then(|| {
                        let line_start = self.document.len() - self.buffer.len();
                        (line_start, line_start + line_end + 1)
                    });
                    let line = self
                        .buffer
                        .get(..line_end)
                        .expect("line end must be a character boundary")
                        .to_string();
                    let line_with_newline = self
                        .buffer
                        .get(..=line_end)
                        .expect("newline must be a character boundary")
                        .to_string();
                    self.buffer.replace_range(..=line_end, "");

                    if self.code_mode_enabled {
                        if let Some(fence_len) = execute_fence_len(&line) {
                            if self.markdown_matches(markdown_range, is_top_level_execute_fence) {
                                self.state = ParserState::InExecuteBlock { fence_len };
                                continue;
                            }
                        }
                    }

                    let line_without_cr = line.strip_suffix('\r').unwrap_or(&line);
                    if let Some(command) = line_without_cr.strip_prefix('$') {
                        if self.markdown_matches(markdown_range, is_top_level_paragraph_line) {
                            let command = command.trim();
                            if !command.is_empty() {
                                results.push(EmulatorAction::ShellCommand(command.to_string()));
                            }
                            continue;
                        }
                    }

                    results.push(EmulatorAction::Text(line_with_newline));
                }
            }
        }

        results
    }

    pub(crate) fn flush(&mut self) -> Vec<EmulatorAction> {
        let mut results = self.process_chunk("");

        match self.state {
            ParserState::InExecuteBlock { fence_len } => {
                let code_end = closing_fence_range(&self.buffer, fence_len, true)
                    .map(|(closing_start, _)| {
                        closing_start
                            .checked_sub(1)
                            .filter(|index| self.buffer.as_bytes()[*index] == b'\n')
                            .unwrap_or(closing_start)
                    })
                    .unwrap_or(self.buffer.len());
                let code = self
                    .buffer
                    .get(..code_end)
                    .expect("code boundary must be a character boundary");
                let code = code.strip_suffix('\r').unwrap_or(code).trim();
                if !code.is_empty() {
                    results.push(EmulatorAction::ExecuteCode(code.to_string()));
                }
            }
            ParserState::Normal if !self.buffer.is_empty() => {
                let line = self
                    .buffer
                    .strip_suffix('\r')
                    .unwrap_or(&self.buffer)
                    .to_string();
                let markdown_range = self.markdown_context_available.then(|| {
                    let line_start = self.document.len() - self.buffer.len();
                    (line_start, self.document.len())
                });

                if self.code_mode_enabled
                    && execute_fence_len(&line).is_some()
                    && self.markdown_matches(markdown_range, is_top_level_execute_fence)
                {
                } else if let Some(command) = line.strip_prefix('$') {
                    if self.markdown_matches(markdown_range, is_top_level_paragraph_line) {
                        let command = command.trim();
                        if !command.is_empty() {
                            results.push(EmulatorAction::ShellCommand(command.to_string()));
                        }
                    } else {
                        results.push(EmulatorAction::Text(self.buffer.clone()));
                    }
                } else {
                    results.push(EmulatorAction::Text(self.buffer.clone()));
                }
            }
            ParserState::Normal => {}
        }

        self.buffer.clear();
        self.document.clear();
        self.markdown_context_available = true;
        self.markdown_parse_bytes_remaining = MAX_MARKDOWN_PARSE_BYTES;
        self.state = ParserState::Normal;
        results
    }
}

