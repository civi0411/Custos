use crate::local_inference::model::ModelSettings;
use crate::local_inference::native_tool_parsing::message_from_native_tool_text;
use crate::local_inference::thinking_output::ThinkingOutputFilter;
use crate::local_inference::tool_emulation::{
    message_for_emulator_action, StreamingEmulatorParser,
};
use custos_provider::conversation::message::{Message, MessageContent};
use custos_provider::conversation::token_usage::{DraftStats, ProviderUsage};
use custos_provider::errors::ProviderError;
use safemlx::transforms::eval;
use safemlx::{random, Array, Stream};
use safemlx_lm::models::LoadedModel;
use safemlx_lm_utils::tokenizer::Tokenizer;

use super::prompt::ToolMode;

pub(crate) struct MlxGeneration {
    pub(crate) generated_ids: Vec<u32>,
    pub(crate) generated_text: String,
    pub(crate) draft_stats: Option<DraftStats>,
    pub(crate) time_to_first_token_ms: Option<u64>,
    pub(crate) streamed_response: bool,
}

pub(crate) fn mlx_error(error: impl std::fmt::Display) -> ProviderError {
    ProviderError::ExecutionError(format!("MLX backend error: {}", error))
}

pub(crate) fn generate_single_model(
    model: &mut LoadedModel,
    tokenizer: &Tokenizer,
    prompt_array: &Array,
    eos_token_ids: &[u32],
    max_tokens: usize,
    temp: f32,
    prng_key: Option<Array>,
    stream: &Stream,
    generation_started: std::time::Instant,
    mut emitter: MlxStreamEmitter<'_>,
) -> Result<MlxGeneration, ProviderError> {
    let mut cache = model.new_cache();
    let mut generated_ids = Vec::new();
    let mut streamed_text = String::new();
    let mut time_to_first_token_ms = None;
    let stream_generation = emitter.can_stream();
    let mut decode_stream = tokenizer.decode_stream(true);
    {
        let generator = model
            .generate_with_cache(&mut cache, temp, prompt_array, prng_key, stream)
            .take(max_tokens);
        for token in generator {
            let token = token.map_err(mlx_error)?;
            eval([&token]).map_err(mlx_error)?;
            let token_id = token.item::<u32>(stream);
            time_to_first_token_ms.get_or_insert_with(|| {
                u64::try_from(generation_started.elapsed().as_millis()).unwrap_or(u64::MAX)
            });
            if eos_token_ids.contains(&token_id) {
                break;
            }
            generated_ids.push(token_id);
            if stream_generation {
                if let Some(piece) = decode_stream.step(token_id).map_err(mlx_error)? {
                    if !piece.is_empty() {
                        let should_continue = emitter.push_text(&piece)?;
                        streamed_text.push_str(&piece);
                        if !should_continue {
                            break;
                        }
                    }
                }
            }
        }
    }
    let generated_text = tokenizer.decode(&generated_ids, true).map_err(mlx_error)?;
    let streamed_response = if stream_generation {
        match final_stream_suffix(&generated_text, &streamed_text)? {
            Some(suffix) => {
                if !suffix.is_empty() {
                    emitter.push_text(suffix)?;
                }
                true
            }
            None => false,
        }
    } else {
        false
    };
    if streamed_response {
        emitter.finish()?;
    }
    Ok(MlxGeneration {
        generated_ids,
        generated_text,
        draft_stats: None,
        time_to_first_token_ms,
        streamed_response,
    })
}

pub(crate) fn final_stream_suffix<'a>(
    generated_text: &'a str,
    streamed_text: &str,
) -> Result<Option<&'a str>, ProviderError> {
    if streamed_text.is_empty() {
        return Ok(None);
    }

    generated_text
        .strip_prefix(streamed_text)
        .map(Some)
        .ok_or_else(|| mlx_error("streamed MLX decode did not match final tokenizer decode"))
}

pub(crate) fn mlx_max_tokens(
    settings: &ModelSettings,
    request_max_tokens: Option<i32>,
    context_limit: usize,
    prompt_tokens: usize,
) -> usize {
    let configured_max = settings
        .max_output_tokens
        .or_else(|| request_max_tokens.and_then(|tokens| usize::try_from(tokens).ok()));
    if context_limit == 0 {
        return configured_max.unwrap_or(4096);
    }

    let context_headroom = context_limit.saturating_sub(prompt_tokens);
    configured_max
        .map(|max| max.min(context_headroom))
        .unwrap_or(context_headroom)
}

pub(crate) fn emit_generated_response(
    generated_text: &str,
    generation_prompt: &str,
    enable_thinking: bool,
    message_id: &str,
    tool_mode: ToolMode,
    tx: &tokio::sync::mpsc::Sender<
        Result<(Option<Message>, Option<ProviderUsage>), ProviderError>,
    >,
) -> Result<(), ProviderError> {
    if generated_text.is_empty() {
        return Ok(());
    }

    let (content, thinking) =
        split_generated_thinking(generated_text, generation_prompt, enable_thinking);

    match tool_mode {
        ToolMode::None => {
            emit_assistant_message(message_id, &thinking, &content, tx)?;
        }
        ToolMode::Native => {
            if let Some(mut message) = message_from_native_tool_text(&content, message_id)? {
                prepend_thinking(&mut message, &thinking);
                tx.blocking_send(Ok((Some(message), None))).map_err(|_| {
                    ProviderError::ExecutionError("Failed to stream MLX response".to_string())
                })?;
            } else {
                emit_assistant_message(message_id, &thinking, &content, tx)?;
            }
        }
        ToolMode::Emulated { code_mode_enabled } => {
            emit_assistant_message(message_id, &thinking, "", tx)?;
            let mut parser = StreamingEmulatorParser::new(code_mode_enabled);
            let mut actions = parser.process_chunk(&content);
            actions.extend(parser.flush());

            for action in actions {
                let (message, _) = message_for_emulator_action(&action, message_id);
                tx.blocking_send(Ok((Some(message), None))).map_err(|_| {
                    ProviderError::ExecutionError("Failed to stream MLX response".to_string())
                })?;
            }
        }
    }
    Ok(())
}

pub(crate) struct MlxStreamEmitter<'a> {
    message_id: &'a str,
    tool_mode: ToolMode,
    tx: &'a tokio::sync::mpsc::Sender<
        Result<(Option<Message>, Option<ProviderUsage>), ProviderError>,
    >,
    output_filter: ThinkingOutputFilter,
    emulator_parser: Option<StreamingEmulatorParser>,
    stop_after_tool_call: bool,
}

impl<'a> MlxStreamEmitter<'a> {
    pub(crate) fn new(
        message_id: &'a str,
        tool_mode: ToolMode,
        enable_thinking: bool,
        generation_prompt: &str,
        tx: &'a tokio::sync::mpsc::Sender<
            Result<(Option<Message>, Option<ProviderUsage>), ProviderError>,
        >,
    ) -> Self {
        let emulator_parser = match tool_mode {
            ToolMode::Emulated { code_mode_enabled } => {
                Some(StreamingEmulatorParser::new(code_mode_enabled))
            }
            ToolMode::None | ToolMode::Native => None,
        };
        Self {
            message_id,
            tool_mode,
            tx,
            output_filter: ThinkingOutputFilter::new(enable_thinking, generation_prompt),
            emulator_parser,
            stop_after_tool_call: false,
        }
    }

    pub(crate) fn can_stream(&self) -> bool {
        !matches!(self.tool_mode, ToolMode::Native)
    }

    pub(crate) fn push_text(&mut self, text: &str) -> Result<bool, ProviderError> {
        let filtered = self.output_filter.push_text(text);
        if !filtered.content.is_empty() {
            self.emit_content(&filtered.content)?;
        }
        Ok(!self.stop_after_tool_call)
    }

    pub(crate) fn finish(&mut self) -> Result<(), ProviderError> {
        self.flush_filtered_output()?;
        let actions = self
            .emulator_parser
            .as_mut()
            .map(StreamingEmulatorParser::flush)
            .unwrap_or_default();
        for action in actions {
            let (message, is_tool) = message_for_emulator_action(&action, self.message_id);
            if is_tool {
                self.flush_filtered_output()?;
            }
            self.send(message)?;
            self.stop_after_tool_call |= is_tool;
            if is_tool {
                break;
            }
        }
        Ok(())
    }

    fn flush_filtered_output(&mut self) -> Result<(), ProviderError> {
        let filtered = self.output_filter.finish();
        if !filtered.thinking.is_empty() {
            let mut message = Message::assistant().with_thinking(filtered.thinking, "");
            message.id = Some(self.message_id.to_string());
            self.send(message)?;
        }
        if !filtered.content.is_empty() {
            self.emit_content(&filtered.content)?;
        }
        Ok(())
    }

    fn emit_content(&mut self, content: &str) -> Result<(), ProviderError> {
        match self.tool_mode {
            ToolMode::None => {
                let mut message = Message::assistant().with_text(content);
                message.id = Some(self.message_id.to_string());
                self.send(message)
            }
            ToolMode::Emulated { .. } => {
                let actions = self
                    .emulator_parser
                    .as_mut()
                    .map(|parser| parser.process_chunk(content))
                    .unwrap_or_default();
                for action in actions {
                    let (message, is_tool) =
                        message_for_emulator_action(&action, self.message_id);
                    if is_tool {
                        self.flush_filtered_output()?;
                    }
                    self.send(message)?;
                    self.stop_after_tool_call |= is_tool;
                    if is_tool {
                        break;
                    }
                }
                Ok(())
            }
            ToolMode::Native => Ok(()),
        }
    }

    fn send(&self, message: Message) -> Result<(), ProviderError> {
        self.tx
            .blocking_send(Ok((Some(message), None)))
            .map_err(|_| {
                ProviderError::ExecutionError("Failed to stream MLX response".to_string())
            })
    }
}

pub(crate) fn split_generated_thinking(
    generated_text: &str,
    generation_prompt: &str,
    enable_thinking: bool,
) -> (String, String) {
    let mut filter = ThinkingOutputFilter::new(enable_thinking, generation_prompt);
    let mut filtered = filter.push_text(generated_text);
    let final_filtered = filter.finish();
    filtered.content.push_str(&final_filtered.content);
    filtered.thinking.push_str(&final_filtered.thinking);
    (filtered.content, filtered.thinking)
}

pub(crate) fn emit_assistant_message(
    message_id: &str,
    thinking: &str,
    content: &str,
    tx: &tokio::sync::mpsc::Sender<
        Result<(Option<Message>, Option<ProviderUsage>), ProviderError>,
    >,
) -> Result<(), ProviderError> {
    if thinking.is_empty() && content.is_empty() {
        return Ok(());
    }

    let mut message = Message::assistant();
    if !thinking.is_empty() {
        message = message.with_thinking(thinking, "");
    }
    if !content.is_empty() {
        message = message.with_text(content);
    }
    message.id = Some(message_id.to_string());
    tx.blocking_send(Ok((Some(message), None)))
        .map_err(|_| ProviderError::ExecutionError("Failed to stream MLX response".to_string()))
}

pub(crate) fn prepend_thinking(message: &mut Message, thinking: &str) {
    if !thinking.is_empty() {
        message
            .content
            .insert(0, MessageContent::thinking(thinking, ""));
    }
}

pub(crate) fn sampling(settings: &ModelSettings) -> (f32, Option<u32>) {
    match &settings.sampling {
        crate::model::SamplingConfig::Greedy => (0.0, None),
        crate::model::SamplingConfig::Temperature {
            temperature, seed, ..
        } => (*temperature, *seed),
        crate::model::SamplingConfig::MirostatV2 { seed, .. } => (0.0, *seed),
    }
}

pub(crate) fn prng_key(temp: f32, seed: Option<u32>) -> Result<Option<Array>, ProviderError> {
    if temp == 0.0 {
        return Ok(None);
    }
    random::key(seed.unwrap_or(0) as u64)
        .map(Some)
        .map_err(mlx_error)
}
