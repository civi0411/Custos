mod parser;
mod prompt;

#[cfg(test)]
mod tests;

#[cfg(feature = "local-inference-native")]
pub(crate) use parser::{EmulatorAction, StreamingEmulatorParser};
#[cfg(feature = "mlx")]
pub(crate) use prompt::{
    build_emulator_tool_description, load_tiny_model_prompt, message_for_emulator_action,
    CODE_EXECUTION_TOOL, SHELL_TOOL,
};

