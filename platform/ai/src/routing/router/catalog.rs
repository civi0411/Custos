//! Registry of standard OpenAI-compatible inference providers and endpoints.
//!
//! Replaces sprawling ad-hoc match blocks across streaming, classification,
//! and unary execution with a single, type-safe Strategy registry.

#[derive(Debug, Clone, Copy)]
pub struct OpenAiCompatibleEndpoint {
    pub service: &'static str,
    pub default_base_url: &'static str,
    pub prefixes: &'static [&'static str],
}

pub const OPENAI_COMPATIBLE_ENDPOINTS: &[OpenAiCompatibleEndpoint] = &[
    OpenAiCompatibleEndpoint {
        service: "groq",
        default_base_url: "https://api.groq.com/openai/v1",
        prefixes: &["groq"],
    },
    OpenAiCompatibleEndpoint {
        service: "openrouter",
        default_base_url: "https://openrouter.ai/api/v1",
        prefixes: &["openrouter"],
    },
    OpenAiCompatibleEndpoint {
        service: "together",
        default_base_url: "https://api.together.xyz/v1",
        prefixes: &["together"],
    },
    OpenAiCompatibleEndpoint {
        service: "mistral",
        default_base_url: "https://api.mistral.ai/v1",
        prefixes: &["mistral"],
    },
    OpenAiCompatibleEndpoint {
        service: "cerebras",
        default_base_url: "https://api.cerebras.ai/v1",
        prefixes: &["cerebras"],
    },
    OpenAiCompatibleEndpoint {
        service: "perplexity",
        default_base_url: "https://api.perplexity.ai",
        prefixes: &["perplexity"],
    },
    OpenAiCompatibleEndpoint {
        service: "nvidia",
        default_base_url: "https://integrate.api.nvidia.com/v1",
        prefixes: &["nvidia"],
    },
    OpenAiCompatibleEndpoint {
        service: "moonshot",
        default_base_url: "https://api.moonshot.cn/v1",
        prefixes: &["moonshot", "kimi"],
    },
    OpenAiCompatibleEndpoint {
        service: "zhipu",
        default_base_url: "https://open.bigmodel.cn/api/paas/v4",
        prefixes: &["zhipu", "zai"],
    },
    OpenAiCompatibleEndpoint {
        service: "minimax",
        default_base_url: "https://api.minimax.chat/v1",
        prefixes: &["minimax"],
    },
    OpenAiCompatibleEndpoint {
        service: "alibaba",
        default_base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
        prefixes: &["alibaba", "qwen"],
    },
    OpenAiCompatibleEndpoint {
        service: "xai",
        default_base_url: "https://api.x.ai/v1",
        prefixes: &["xai", "grok"],
    },
    OpenAiCompatibleEndpoint {
        service: "sambanova",
        default_base_url: "https://api.sambanova.ai/v1",
        prefixes: &["sambanova"],
    },
    OpenAiCompatibleEndpoint {
        service: "fireworks",
        default_base_url: "https://api.fireworks.ai/inference/v1",
        prefixes: &["fireworks"],
    },
    OpenAiCompatibleEndpoint {
        service: "siliconflow",
        default_base_url: "https://api.siliconflow.cn/v1",
        prefixes: &["siliconflow"],
    },
    OpenAiCompatibleEndpoint {
        service: "nebius",
        default_base_url: "https://api.studio.nebius.ai/v1",
        prefixes: &["nebius"],
    },
    OpenAiCompatibleEndpoint {
        service: "hyperbolic",
        default_base_url: "https://api.hyperbolic.xyz/v1",
        prefixes: &["hyperbolic"],
    },
    OpenAiCompatibleEndpoint {
        service: "cohere",
        default_base_url: "https://api.cohere.com/v2",
        prefixes: &["cohere"],
    },
];

pub fn find_by_service(service: &str) -> Option<&'static OpenAiCompatibleEndpoint> {
    OPENAI_COMPATIBLE_ENDPOINTS
        .iter()
        .find(|e| e.service == service || e.prefixes.contains(&service))
}

pub fn find_by_model(model: &str) -> Option<(&'static OpenAiCompatibleEndpoint, &'static str)> {
    let lower = model.to_lowercase();
    for endpoint in OPENAI_COMPATIBLE_ENDPOINTS {
        for &prefix in endpoint.prefixes {
            if lower == prefix {
                return Some((endpoint, endpoint.service));
            }
            if let Some(rest) = lower.strip_prefix(prefix) {
                if rest.starts_with('/') || rest.starts_with(':') {
                    return Some((endpoint, endpoint.service));
                }
            }
        }
    }
    None
}

pub fn strip_model_prefix<'a>(model: &'a str, endpoint: &OpenAiCompatibleEndpoint) -> &'a str {
    for &prefix in endpoint.prefixes {
        if let Some(stripped) = model.strip_prefix(prefix) {
            if let Some(after_slash) = stripped.strip_prefix('/') {
                return after_slash;
            }
            if let Some(after_colon) = stripped.strip_prefix(':') {
                return after_colon;
            }
        }
    }
    model
}
