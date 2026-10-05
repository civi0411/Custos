"""
Custos Nexus: Model Context Protocol (MCP) stdio server
Enables any MCP-compliant agent (Claude Desktop, Cursor, Goose, Custos)
to query codebase AST, Call Graph, FTS5 search, and Neighborhood slices.
"""

import sys
import json
import logging
from typing import Dict, Any, Optional

from src.llm_interface.tool_server import NexusToolServer
from src.tools import tool_read_file

logging.basicConfig(level=logging.INFO, stream=sys.stderr, format="%(asctime)s - %(name)s - %(levelname)s - %(message)s")
logger = logging.getLogger("nexus-mcp")

import subprocess
from src.workspace import resolve_repo_path

SUPPORTED_PROTOCOL_VERSIONS = ["2024-11-05", "2024-10-07", "0.1.0"]

def get_repo_snapshot_id(repo: str) -> str:
    """Resolves HEAD commit hash or fallback identifier for repo snapshot."""
    try:
        base = resolve_repo_path(repo)
        res = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=base,
            capture_output=True,
            text=True,
            timeout=2
        )
        if res.returncode == 0 and res.stdout.strip():
            return res.stdout.strip()
    except Exception:
        pass
    return "snapshot_untracked"

class NexusMcpServer:
    def __init__(self):
        self.tool_server = NexusToolServer()
        self.tools = self.tool_server.get_tool_definitions()

    def handle_request(self, request: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        req_id = request.get("id")
        method = request.get("method")
        params = request.get("params", {})

        if method == "initialize":
            client_version = params.get("protocolVersion")
            protocol_version = (
                client_version
                if client_version in SUPPORTED_PROTOCOL_VERSIONS
                else SUPPORTED_PROTOCOL_VERSIONS[0]
            )
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": protocol_version,
                    "serverInfo": {
                        "name": "custos-repo-intelligent",
                        "version": "2.0.0"
                    },
                    "capabilities": {
                        "tools": {
                            "listChanged": False
                        }
                    }
                }
            }

        elif method == "notifications/initialized":
            return None

        elif method == "ping":
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {}
            }

        elif method == "tools/list":
            # Map tool definitions to MCP format
            mcp_tools = []
            for t in self.tools:
                mcp_tools.append({
                    "name": t["name"],
                    "description": t["description"],
                    "inputSchema": t["parameters"]
                })
            # Also add direct read tool
            mcp_tools.append({
                "name": "nexus_read",
                "description": "Read exact lines of code from a file with line numbers.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "repo": {"type": "string", "enum": ["custos", "goose"], "default": "custos"},
                        "path": {"type": "string", "description": "Relative file path"},
                        "start_line": {"type": "integer", "default": 1},
                        "end_line": {"type": "integer", "default": 100}
                    },
                    "required": ["path"]
                }
            })
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "tools": mcp_tools
                }
            }

        elif method == "tools/call":
            name = params.get("name")
            arguments = params.get("arguments", {})
            repo = arguments.get("repo", "custos")
            snapshot_id = get_repo_snapshot_id(repo)

            try:
                if name == "nexus_read":
                    path = arguments.get("path", "")
                    start = arguments.get("start_line", 1)
                    end = arguments.get("end_line", 100)
                    text = tool_read_file(repo=repo, path=path, start_line=start, end_line=end)
                    locator = {"repo": repo, "path": path, "start_line": start, "end_line": end}
                else:
                    text = self.tool_server.call_tool(name, arguments)
                    locator = {"repo": repo, "tool": name, "args": arguments}

                formatted_output = {
                    "snapshot_id": snapshot_id,
                    "locator": locator,
                    "stale_status": "fresh",
                    "payload": text,
                }

                output_text = (
                    json.dumps(formatted_output, indent=2)
                    if not isinstance(text, str)
                    else f"/* snapshot_id: {snapshot_id} | stale_status: fresh */\n{text}"
                )

                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": output_text
                            }
                        ],
                        "isError": False
                    }
                }
            except Exception as e:
                logger.error(f"Error executing tool {name}: {e}", exc_info=True)
                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": f"Error executing tool '{name}': {str(e)}"
                            }
                        ],
                        "isError": True
                    }
                }


        else:
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32601,
                    "message": f"Method not found: {method}"
                }
            }

    def run(self):
        logger.info("Starting Nexus MCP Server on stdio...")
        for line in sys.stdin:
            line = line.strip()
            if not line:
                continue
            try:
                request = json.loads(line)
                response = self.handle_request(request)
                if response is not None:
                    sys.stdout.write(json.dumps(response) + "\n")
                    sys.stdout.flush()
            except Exception as e:
                logger.error(f"Error processing input: {e}", exc_info=True)

if __name__ == "__main__":
    server = NexusMcpServer()
    server.run()
