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

class NexusMcpServer:
    def __init__(self):
        self.tool_server = NexusToolServer()
        self.tools = self.tool_server.get_tool_definitions()

    def handle_request(self, request: Dict[str, Any]) -> Optional[Dict[str, Any]]:
        req_id = request.get("id")
        method = request.get("method")
        params = request.get("params", {})

        if method == "initialize":
            return {
                "jsonrpc": "2.0",
                "id": req_id,
                "result": {
                    "protocolVersion": "2024-11-05",
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

            try:
                if name == "nexus_read":
                    repo = arguments.get("repo", "custos")
                    path = arguments.get("path", "")
                    start = arguments.get("start_line", 1)
                    end = arguments.get("end_line", 100)
                    text = tool_read_file(repo=repo, path=path, start_line=start, end_line=end)
                else:
                    text = self.tool_server.call_tool(name, arguments)

                return {
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "result": {
                        "content": [
                            {
                                "type": "text",
                                "text": str(text)
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
