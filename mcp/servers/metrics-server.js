#!/usr/bin/env node

/**
 * MCP Metrics Server for GeoClue Prometheus Exporter
 * Provides access to Prometheus metrics endpoint and geolocation data
 */

const { Server } = require('@modelcontextprotocol/sdk/server/index.js');
const { StdioServerTransport } = require('@modelcontextprotocol/sdk/server/stdio.js');
const { CallToolRequestSchema, ListToolsRequestSchema, ListResourcesRequestSchema, ReadResourceRequestSchema } = require('@modelcontextprotocol/sdk/types.js');
const http = require('http');
const https = require('https');

class MetricsServer {
  constructor() {
    this.server = new Server(
      {
        name: 'geoclue-metrics-server',
        version: '0.5.0',
      },
      {
        capabilities: {
          tools: {},
          resources: {},
        },
      }
    );
    
    this.setupToolHandlers();
    this.setupResourceHandlers();
  }

  setupToolHandlers() {
    this.server.setRequestHandler(ListToolsRequestSchema, async () => ({
      tools: [
        {
          name: 'get_metrics',
          description: 'Fetch Prometheus metrics from the exporter endpoint',
          inputSchema: {
            type: 'object',
            properties: {
              host: {
                type: 'string',
                description: 'Host address of the metrics server',
                default: '127.0.0.1'
              },
              port: {
                type: 'number',
                description: 'Port of the metrics server', 
                default: 9090
              },
              path: {
                type: 'string',
                description: 'Metrics endpoint path',
                default: '/metrics'
              }
            },
          },
        },
        {
          name: 'get_geolocation_metrics',
          description: 'Get specific geolocation metrics (latitude, longitude, accuracy)',
          inputSchema: {
            type: 'object',
            properties: {
              host: {
                type: 'string',
                description: 'Host address of the metrics server',
                default: '127.0.0.1'
              },
              port: {
                type: 'number', 
                description: 'Port of the metrics server',
                default: 9090
              },
              metric_filter: {
                type: 'string',
                description: 'Filter for specific metrics (e.g., "geoclue_latitude")',
                default: 'geoclue_'
              }
            },
          },
        },
        {
          name: 'check_service_health',
          description: 'Check if the geoclue exporter service is healthy',
          inputSchema: {
            type: 'object',
            properties: {
              host: {
                type: 'string',
                description: 'Host address of the metrics server',
                default: '127.0.0.1'
              },
              port: {
                type: 'number',
                description: 'Port of the metrics server',
                default: 9090
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
          case 'get_metrics':
            return await this.getMetrics(args);
          case 'get_geolocation_metrics':
            return await this.getGeolocationMetrics(args);
          case 'check_service_health':
            return await this.checkServiceHealth(args);
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
          uri: 'geoclue://metrics/current',
          name: 'Current Metrics',
          description: 'Current Prometheus metrics from the exporter',
          mimeType: 'text/plain',
        },
        {
          uri: 'geoclue://location/current',
          name: 'Current Location',
          description: 'Current geolocation data',
          mimeType: 'application/json',
        },
      ],
    }));

    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;

      try {
        switch (uri) {
          case 'geoclue://metrics/current':
            const metrics = await this.fetchMetrics();
            return {
              contents: [
                {
                  uri,
                  mimeType: 'text/plain',
                  text: metrics,
                },
              ],
            };
          case 'geoclue://location/current':
            const location = await this.parseLocationFromMetrics();
            return {
              contents: [
                {
                  uri,
                  mimeType: 'application/json',
                  text: JSON.stringify(location, null, 2),
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

  async getMetrics(args = {}) {
    const { host = '127.0.0.1', port = 9090, path = '/metrics' } = args;
    
    try {
      const metrics = await this.fetchMetrics(host, port, path);
      return {
        content: [
          {
            type: 'text',
            text: `Prometheus Metrics from ${host}:${port}${path}:\n\n${metrics}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to fetch metrics: ${error.message}`);
    }
  }

  async getGeolocationMetrics(args = {}) {
    const { host = '127.0.0.1', port = 9090, metric_filter = 'geoclue_' } = args;
    
    try {
      const metrics = await this.fetchMetrics(host, port);
      const geoMetrics = metrics
        .split('\n')
        .filter(line => line.includes(metric_filter) && !line.startsWith('#'))
        .join('\n');

      const location = await this.parseLocationFromMetrics(metrics);
      
      return {
        content: [
          {
            type: 'text', 
            text: `Geolocation Metrics:\n\n${geoMetrics}\n\nParsed Location Data:\n${JSON.stringify(location, null, 2)}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to fetch geolocation metrics: ${error.message}`);
    }
  }

  async checkServiceHealth(args = {}) {
    const { host = '127.0.0.1', port = 9090 } = args;
    
    try {
      const startTime = Date.now();
      await this.fetchMetrics(host, port);
      const responseTime = Date.now() - startTime;
      
      return {
        content: [
          {
            type: 'text',
            text: `Service Health Check: ✅ HEALTHY\nHost: ${host}:${port}\nResponse Time: ${responseTime}ms\nStatus: Service is responding to metrics requests`,
          },
        ],
      };
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `Service Health Check: ❌ UNHEALTHY\nHost: ${host}:${port}\nError: ${error.message}`,
          },
        ],
      };
    }
  }

  async fetchMetrics(host = '127.0.0.1', port = 9090, path = '/metrics') {
    return new Promise((resolve, reject) => {
      const options = {
        hostname: host,
        port: port,
        path: path,
        method: 'GET',
        timeout: 30000,
      };

      const req = http.request(options, (res) => {
        let data = '';
        res.on('data', (chunk) => {
          data += chunk;
        });
        res.on('end', () => {
          if (res.statusCode === 200) {
            resolve(data);
          } else {
            reject(new Error(`HTTP ${res.statusCode}: ${res.statusMessage}`));
          }
        });
      });

      req.on('error', (error) => {
        reject(error);
      });

      req.on('timeout', () => {
        req.destroy();
        reject(new Error('Request timeout'));
      });

      req.end();
    });
  }

  async parseLocationFromMetrics(metricsText) {
    if (!metricsText) {
      metricsText = await this.fetchMetrics();
    }

    const location = {};
    const lines = metricsText.split('\n');

    for (const line of lines) {
      if (line.startsWith('geoclue_latitude ')) {
        location.latitude = parseFloat(line.split(' ')[1]);
      } else if (line.startsWith('geoclue_longitude ')) {
        location.longitude = parseFloat(line.split(' ')[1]);
      } else if (line.startsWith('geoclue_accuracy ')) {
        location.accuracy = parseFloat(line.split(' ')[1]);
      } else if (line.startsWith('geoclue_altitude ')) {
        location.altitude = parseFloat(line.split(' ')[1]);
      } else if (line.startsWith('up ')) {
        location.service_up = parseFloat(line.split(' ')[1]) === 1;
      }
    }

    location.timestamp = new Date().toISOString();
    return location;
  }

  async run() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error('GeoClue Metrics MCP server running on stdio');
  }
}

if (require.main === module) {
  const server = new MetricsServer();
  server.run().catch(console.error);
}

module.exports = MetricsServer;