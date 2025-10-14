#!/usr/bin/env node

import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
  Tool,
} from "@modelcontextprotocol/sdk/types.js";
import { execFile } from "child_process";
import { promisify } from "util";
import { access, readdir } from "fs/promises";
import { join } from "path";

const execFileAsync = promisify(execFile);

// Available themes in ttyvid
const THEMES = [
  "default",
  "windows7",
  "mac",
  "fdwm",
  "fdwm-x",
  "game",
  "bar",
  "opensource",
  "scripted",
  "simple"
];

// Available fonts (subset of most common ones)
const FONTS = [
  "IBM_VGA8",
  "IBM_VGA9",
  "IBM_BIOS",
  "IBM_CGA",
  "IBM_EGA_8x14",
  "IBM_EGA_8x8",
  "Verite_8x16",
  "Verite_9x16",
  "ATI_8x16",
  "Phoenix_EGA_8x16",
  "Compaq_Thin_8x16"
];

/**
 * Find ttyvid binary in PATH or local build
 */
async function findTtyvidBinary(): Promise<string> {
  // Check local build first
  const localBinary = join(process.cwd(), "../target/release/ttyvid");
  try {
    await access(localBinary);
    return localBinary;
  } catch {
    // Fall back to PATH
    return "ttyvid";
  }
}

/**
 * Execute ttyvid command
 */
async function executeTtyvid(args: string[]): Promise<{ stdout: string; stderr: string }> {
  const binary = await findTtyvidBinary();
  try {
    const result = await execFileAsync(binary, args, { maxBuffer: 10 * 1024 * 1024 });
    return result;
  } catch (error: any) {
    throw new Error(`ttyvid execution failed: ${error.message}`);
  }
}

/**
 * MCP Server for ttyvid
 */
class TtyvidServer {
  private server: Server;

  constructor() {
    this.server = new Server(
      {
        name: "ttyvid-mcp",
        version: "0.2.0",
      },
      {
        capabilities: {
          tools: {},
        },
      }
    );

    this.setupHandlers();
  }

  private setupHandlers() {
    // List available tools
    this.server.setRequestHandler(ListToolsRequestSchema, async () => {
      return {
        tools: [
          {
            name: "convert_recording",
            description: "Convert terminal recording (.cast file) to GIF or WebM video",
            inputSchema: {
              type: "object",
              properties: {
                input: {
                  type: "string",
                  description: "Path to input asciicast (.cast) file",
                },
                output: {
                  type: "string",
                  description: "Path to output file (.gif or .webm)",
                },
                theme: {
                  type: "string",
                  description: "Theme name (default, windows7, mac, fdwm, fdwm-x, game, bar, opensource, scripted, simple)",
                  default: "default",
                },
                font: {
                  type: "string",
                  description: "Font name (e.g., IBM_VGA8, Verite_9x16)",
                },
                fps: {
                  type: "number",
                  description: "Frames per second (3-100)",
                  default: 10,
                },
                speed: {
                  type: "number",
                  description: "Speed multiplier",
                  default: 1.0,
                },
                quality: {
                  type: "number",
                  description: "Video quality for WebM (0-100, higher is better)",
                  default: 50,
                },
                no_gaps: {
                  type: "boolean",
                  description: "Remove gaps in recording",
                  default: false,
                },
                trailer: {
                  type: "boolean",
                  description: "Add trailer at end (1.5s pause before loop)",
                  default: false,
                },
                title: {
                  type: "string",
                  description: "Title text to display",
                },
              },
              required: ["input", "output"],
            },
          },
          {
            name: "list_themes",
            description: "List all available ttyvid themes",
            inputSchema: {
              type: "object",
              properties: {},
            },
          },
          {
            name: "list_fonts",
            description: "List all available ttyvid fonts",
            inputSchema: {
              type: "object",
              properties: {},
            },
          },
          {
            name: "get_version",
            description: "Get ttyvid version information",
            inputSchema: {
              type: "object",
              properties: {},
            },
          },
        ] as Tool[],
      };
    });

    // Handle tool execution
    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      try {
        switch (name) {
          case "convert_recording": {
            const {
              input,
              output,
              theme = "default",
              font,
              fps = 10,
              speed = 1.0,
              quality = 50,
              no_gaps = false,
              trailer = false,
              title,
            } = args as any;

            // Build command arguments
            const cmdArgs: string[] = [
              "-i", input,
              "-o", output,
              "--theme", theme,
              "--fps", fps.toString(),
              "--speed", speed.toString(),
              "--quality", quality.toString(),
            ];

            if (font) cmdArgs.push("--font", font);
            if (no_gaps) cmdArgs.push("--no-gaps");
            if (trailer) cmdArgs.push("--trailer");
            if (title) cmdArgs.push("--title", title);

            const result = await executeTtyvid(cmdArgs);

            return {
              content: [
                {
                  type: "text",
                  text: `Successfully converted ${input} to ${output}\n\nSettings:\n- Theme: ${theme}\n- Font: ${font || 'default'}\n- FPS: ${fps}\n- Speed: ${speed}x\n- Quality: ${quality}\n- Remove gaps: ${no_gaps}\n- Trailer: ${trailer}\n${title ? `- Title: ${title}` : ''}`,
                },
              ],
            };
          }

          case "list_themes": {
            return {
              content: [
                {
                  type: "text",
                  text: `Available ttyvid themes:\n\n${THEMES.map(t => `- ${t}`).join('\n')}\n\nUse these theme names with the convert_recording tool.`,
                },
              ],
            };
          }

          case "list_fonts": {
            return {
              content: [
                {
                  type: "text",
                  text: `Available ttyvid fonts (common selection):\n\n${FONTS.map(f => `- ${f}`).join('\n')}\n\nMany more fonts are available. Use these font names with the convert_recording tool.`,
                },
              ],
            };
          }

          case "get_version": {
            const result = await executeTtyvid(["--version"]);
            return {
              content: [
                {
                  type: "text",
                  text: result.stdout.trim(),
                },
              ],
            };
          }

          default:
            throw new Error(`Unknown tool: ${name}`);
        }
      } catch (error: any) {
        return {
          content: [
            {
              type: "text",
              text: `Error: ${error.message}`,
            },
          ],
          isError: true,
        };
      }
    });
  }

  async run() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error("ttyvid MCP server running on stdio");
  }
}

// Start the server
const server = new TtyvidServer();
server.run().catch(console.error);
