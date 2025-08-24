#!/usr/bin/env node

/**
 * MCP Configuration Server for GeoClue Prometheus Exporter
 * Manages service configuration and settings
 */

const { Server } = require('@modelcontextprotocol/sdk/server/index.js');
const { StdioServerTransport } = require('@modelcontextprotocol/sdk/server/stdio.js');
const { CallToolRequestSchema, ListToolsRequestSchema, ListResourcesRequestSchema, ReadResourceRequestSchema } = require('@modelcontextprotocol/sdk/types.js');
const fs = require('fs').promises;
const path = require('path');
const { exec } = require('child_process');
const { promisify } = require('util');

const execAsync = promisify(exec);

class ConfigServer {
  constructor() {
    this.server = new Server(
      {
        name: 'geoclue-config-server',
        version: '0.5.0',
      },
      {
        capabilities: {
          tools: {},
          resources: {},
        },
      }
    );

    this.configPaths = {
      nixosModule: '/etc/nixos/modules/geoclue-prometheus-exporter.nix',
      serviceConfig: '/etc/systemd/system/geoclue-prometheus-exporter.service',
      mcpConfig: path.join(__dirname, '../config.json'),
    };
    
    this.setupToolHandlers();
    this.setupResourceHandlers();
  }

  setupToolHandlers() {
    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({
      tools: [
        {
          name: 'get_service_config',
          description: 'Get current service configuration',
          inputSchema: {
            type: 'object',
            properties: {
              config_type: {
                type: 'string',
                enum: ['systemd', 'nixos', 'mcp', 'all'],
                description: 'Type of configuration to retrieve',
                default: 'all'
              }
            },
          },
        },
        {
          name: 'update_mcp_config',
          description: 'Update MCP server configuration',
          inputSchema: {
            type: 'object',
            properties: {
              server_name: {
                type: 'string',
                description: 'Name of the MCP server to configure',
              },
              config: {
                type: 'object',
                description: 'Configuration object to merge',
              }
            },
            required: ['server_name', 'config'],
          },
        },
        {
          name: 'get_service_status',
          description: 'Get systemd service status',
          inputSchema: {
            type: 'object',
            properties: {
              service_name: {
                type: 'string',
                description: 'Name of the service',
                default: 'geoclue-prometheus-exporter'
              }
            },
          },
        },
        {
          name: 'get_service_args',
          description: 'Get current service command line arguments',
          inputSchema: {
            type: 'object',
            properties: {},
          },
        },
        {
          name: 'validate_config',
          description: 'Validate service configuration',
          inputSchema: {
            type: 'object',
            properties: {
              config_type: {
                type: 'string',
                enum: ['systemd', 'nixos', 'mcp'],
                description: 'Type of configuration to validate',
                default: 'mcp'
              }
            },
          },
        }
      ],
    }));

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      try {
        switch (name) {
          case 'get_service_config':
            return await this.getServiceConfig(args);
          case 'update_mcp_config':
            return await this.updateMcpConfig(args);
          case 'get_service_status':
            return await this.getServiceStatus(args);
          case 'get_service_args':
            return await this.getServiceArgs(args);
          case 'validate_config':
            return await this.validateConfig(args);
          default:
            throw new Error(`Tool ${name} not found`);
        }
      } catch (error) {
        return {
          content: [
            {
              type: 'text',
              text: `Error: ${error.message}`,
            },
          ],
          isError: true,
        };
      }
    });
  }

  setupResourceHandlers() {
    this.server.setRequestHandler(ListResourcesRequestSchema, async () => ({
      resources: [
        {
          uri: 'geoclue://config/mcp',
          name: 'MCP Configuration',
          description: 'Current MCP server configuration',
          mimeType: 'application/json',
        },
        {
          uri: 'geoclue://config/systemd',
          name: 'Systemd Service Configuration',
          description: 'Systemd service unit configuration',
          mimeType: 'text/plain',
        },
        {
          uri: 'geoclue://config/nixos',
          name: 'NixOS Module Configuration',
          description: 'NixOS module configuration',
          mimeType: 'text/plain',
        },
      ],
    }));

    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;

      try {
        switch (uri) {
          case 'geoclue://config/mcp':
            const mcpConfig = await this.readConfigFile(this.configPaths.mcpConfig);
            return {
              contents: [
                {
                  uri,
                  mimeType: 'application/json',
                  text: mcpConfig,
                },
              ],
            };
          case 'geoclue://config/systemd':
            const systemdConfig = await this.readSystemdConfig();
            return {
              contents: [
                {
                  uri,
                  mimeType: 'text/plain',
                  text: systemdConfig,
                },
              ],
            };
          case 'geoclue://config/nixos':
            const nixosConfig = await this.readNixosConfig();
            return {
              contents: [
                {
                  uri,
                  mimeType: 'text/plain',
                  text: nixosConfig,
                },
              ],
            };
          default:
            throw new Error(`Resource ${uri} not found`);
        }
      } catch (error) {
        throw new Error(`Failed to read resource ${uri}: ${error.message}`);
      }
    });
  }

  async getServiceConfig(args = {}) {
    const { config_type = 'all' } = args;
    
    try {
      let configData = {};

      if (config_type === 'all' || config_type === 'mcp') {
        try {
          const mcpConfig = await this.readConfigFile(this.configPaths.mcpConfig);
          configData.mcp = JSON.parse(mcpConfig);
        } catch (error) {
          configData.mcp = { error: error.message };
        }
      }

      if (config_type === 'all' || config_type === 'systemd') {
        try {
          configData.systemd = await this.readSystemdConfig();
        } catch (error) {
          configData.systemd = { error: error.message };
        }
      }

      if (config_type === 'all' || config_type === 'nixos') {
        try {
          configData.nixos = await this.readNixosConfig();
        } catch (error) {
          configData.nixos = { error: error.message };
        }
      }

      return {
        content: [
          {
            type: 'text',
            text: `Service Configuration (${config_type}):\n\n${JSON.stringify(configData, null, 2)}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to get service config: ${error.message}`);
    }
  }

  async updateMcpConfig(args) {
    const { server_name, config } = args;
    
    try {
      const mcpConfigPath = this.configPaths.mcpConfig;
      const currentConfig = JSON.parse(await this.readConfigFile(mcpConfigPath));
      
      if (!currentConfig.servers[server_name]) {
        throw new Error(`Server ${server_name} not found in MCP configuration`);
      }

      // Merge the configuration
      currentConfig.servers[server_name] = {
        ...currentConfig.servers[server_name],
        ...config
      };

      await this.writeConfigFile(mcpConfigPath, JSON.stringify(currentConfig, null, 2));

      return {
        content: [
          {
            type: 'text',
            text: `Successfully updated MCP configuration for server: ${server_name}\n\nUpdated config:\n${JSON.stringify(currentConfig.servers[server_name], null, 2)}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to update MCP config: ${error.message}`);
    }
  }

  async getServiceStatus(args = {}) {
    const { service_name = 'geoclue-prometheus-exporter' } = args;
    
    try {
      const { stdout, stderr } = await execAsync(`systemctl status ${service_name} --no-pager`);
      
      return {
        content: [
          {
            type: 'text',
            text: `Service Status for ${service_name}:\n\n${stdout}${stderr ? '\nErrors:\n' + stderr : ''}`,
          },
        ],
      };
    } catch (error) {
      // systemctl returns non-zero exit code for inactive services
      return {
        content: [
          {
            type: 'text',
            text: `Service Status for ${service_name}:\n\n${error.stdout || error.message}`,
          },
        ],
      };
    }
  }

  async getServiceArgs(args = {}) {
    try {
      // Try to get the current running process arguments
      const { stdout } = await execAsync(`pgrep -f geoclue-prometheus-exporter | head -1 | xargs ps -p | tail -1`);
      
      if (stdout.trim()) {
        const processInfo = stdout.trim();
        return {
          content: [
            {
              type: 'text',
              text: `Current Service Process:\n\n${processInfo}\n\nTo see full command line arguments, check systemd service configuration.`,
            },
          ],
        };
      } else {
        return {
          content: [
            {
              type: 'text',
              text: `Service is not currently running. Check systemd service configuration for configured arguments.`,
            },
          ],
        };
      }
    } catch (error) {
      throw new Error(`Failed to get service args: ${error.message}`);
    }
  }

  async validateConfig(args = {}) {
    const { config_type = 'mcp' } = args;
    
    try {
      let validationResults = [];

      if (config_type === 'mcp') {
        try {
          const mcpConfig = JSON.parse(await this.readConfigFile(this.configPaths.mcpConfig));
          
          // Validate MCP configuration structure
          if (!mcpConfig.mcpVersion) {
            validationResults.push('❌ Missing mcpVersion field');
          } else {
            validationResults.push('✅ mcpVersion present');
          }

          if (!mcpConfig.servers || Object.keys(mcpConfig.servers).length === 0) {
            validationResults.push('❌ No servers configured');
          } else {
            validationResults.push(`✅ ${Object.keys(mcpConfig.servers).length} servers configured`);
            
            for (const [serverName, serverConfig] of Object.entries(mcpConfig.servers)) {
              if (!serverConfig.command) {
                validationResults.push(`❌ Server ${serverName}: Missing command`);
              } else {
                validationResults.push(`✅ Server ${serverName}: Command configured`);
              }
            }
          }
        } catch (error) {
          validationResults.push(`❌ Invalid JSON in MCP config: ${error.message}`);
        }
      }

      return {
        content: [
          {
            type: 'text',
            text: `Configuration Validation (${config_type}):\n\n${validationResults.join('\n')}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to validate config: ${error.message}`);
    }
  }

  async readConfigFile(filePath) {
    try {
      return await fs.readFile(filePath, 'utf8');
    } catch (error) {
      throw new Error(`Failed to read ${filePath}: ${error.message}`);
    }
  }

  async writeConfigFile(filePath, content) {
    try {
      await fs.writeFile(filePath, content, 'utf8');
    } catch (error) {
      throw new Error(`Failed to write ${filePath}: ${error.message}`);
    }
  }

  async readSystemdConfig() {
    try {
      // Try multiple possible locations for systemd service file
      const possiblePaths = [
        '/etc/systemd/system/geoclue-prometheus-exporter.service',
        '/lib/systemd/system/geoclue-prometheus-exporter.service',
        '/usr/lib/systemd/system/geoclue-prometheus-exporter.service'
      ];

      for (const path of possiblePaths) {
        try {
          return await fs.readFile(path, 'utf8');
        } catch (error) {
          // Continue to next path
        }
      }

      // If no systemd file found, try to get from systemctl
      const { stdout } = await execAsync('systemctl cat geoclue-prometheus-exporter.service');
      return stdout;
    } catch (error) {
      return `Systemd service configuration not found or accessible: ${error.message}`;
    }
  }

  async readNixosConfig() {
    try {
      // Check the current directory for NixOS module files
      const possiblePaths = [
        path.join(process.cwd(), 'nixos-module.nix'),
        path.join(process.cwd(), 'nixos-module-alloy.nix'),
        '/etc/nixos/modules/geoclue-prometheus-exporter.nix'
      ];

      let configs = [];
      for (const configPath of possiblePaths) {
        try {
          const content = await fs.readFile(configPath, 'utf8');
          configs.push(`=== ${configPath} ===\n${content}`);
        } catch (error) {
          // File doesn't exist, skip
        }
      }

      if (configs.length === 0) {
        return 'NixOS configuration files not found in expected locations';
      }

      return configs.join('\n\n');
    } catch (error) {
      return `Failed to read NixOS configuration: ${error.message}`;
    }
  }

  async run() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error('GeoClue Config MCP server running on stdio');
  }
}

if (require.main === module) {
  const server = new ConfigServer();
  server.run().catch(console.error);
}

module.exports = ConfigServer;