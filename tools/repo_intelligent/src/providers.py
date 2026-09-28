import os
import time
from typing import Optional, List
import requests
from dotenv import load_dotenv

load_dotenv()

FALLBACK_FREE_MODELS = [
    "cohere/north-mini-code:free",
    "nex-agi/nex-n2.5-pro:free",
    "nex-agi/nex-n2.5-mini:free",
]

class OpenRouterFreeProvider:
    def __init__(self, api_key: str = None, model: str = "cohere/north-mini-code:free"):
        self.api_key = api_key or os.getenv("OPENROUTER_API_KEY")
        if not self.api_key:
            raise ValueError("OPENROUTER_API_KEY must be set in .env or environment variables.")
        self.model = model
        self.base_url = "https://openrouter.ai/api/v1/chat/completions"
        self.headers = {
            "Authorization": f"Bearer {self.api_key}",
            "HTTP-Referer": "https://custos-nexus.local",
            "X-Title": "Custos Nexus",
            "Content-Type": "application/json"
        }

    def _call_with_backoff(self, payload: dict, max_retries: int = 4) -> dict:
        """
        Giao tiếp với OpenRouter Free API có backoff và tự động chuyển sang model fallback nếu bị upstream rate limit.
        """
        models_to_try = [payload["model"]]
        for fb in FALLBACK_FREE_MODELS:
            if fb not in models_to_try:
                models_to_try.append(fb)

        for current_model in models_to_try:
            payload["model"] = current_model
            delay = 2
            for attempt in range(max_retries):
                try:
                    response = requests.post(self.base_url, headers=self.headers, json=payload, timeout=45)
                    if response.status_code == 200:
                        return response.json()
                    elif response.status_code == 429:
                        print(f"[warning] Model {current_model} rate-limited. Trying in {delay}s...")
                        time.sleep(delay)
                        delay *= 2
                    else:
                        print(f"[warning] Model {current_model} returned {response.status_code}. Trying next model...")
                        break
                except Exception as e:
                    print(f"[warning] Request error with {current_model}: {str(e)}")
                    time.sleep(delay)
                    delay *= 2

        raise RuntimeError("Tất cả các free models trên OpenRouter đều đang bận/quá tải. Vui lòng thử lại sau giây lát.")

    def chat(self, messages: list[dict], temperature: float = 0.1) -> str:
        """
        Gửi hội thoại tới LLM bằng ReAct Text format.
        """
        payload = {
            "model": self.model,
            "messages": messages,
            "temperature": temperature,
        }
        
        result = self._call_with_backoff(payload)
        choices = result.get('choices', [])
        if not choices:
            raise RuntimeError(f"OpenRouter response has no choices: {result}")
        msg = choices[0].get('message', {})
        content = msg.get('content') or ""
        reasoning = msg.get('reasoning')
        tool_calls = msg.get('tool_calls')
        
        if tool_calls:
            tc = tool_calls[0]
            func_name = tc.get('function', {}).get('name')
            raw_args = tc.get('function', {}).get('arguments', '{}')
            try:
                import json
                args_dict = json.loads(raw_args)
                args_str = ", ".join(f'{k}="{v}"' if isinstance(v, str) else f"{k}={v}" for k, v in args_dict.items())
            except Exception:
                args_str = raw_args
            
            thought_text = f"Thought: {reasoning}\n" if reasoning else ""
            return f"{thought_text}Action: {func_name}({args_str})"
            
        if not content and reasoning:
            content = f"Thought: {reasoning}"
            
        return content
