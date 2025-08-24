#!/usr/bin/env node

/**
 * MCP Monitoring Server for GeoClue Prometheus Exporter
 * Provides health checks and service monitoring capabilities
 */

const { Server } = require('@modelcontextprotocol/sdk/server/index.js');
const { StdioServerTransport } = require('@modelcontextprotocol/sdk/server/stdio.js');
const { CallToolRequestSchema, ListToolsRequestSchema, ListResourcesRequestSchema, ReadResourceRequestSchema } = require('@modelcontextprotocol/sdk/types.js');
const http = require('http');
const { exec } = require('child_process');
const { promisify } = require('util');
const fs = require('fs').promises;

const execAsync = promisify(exec);

class MonitoringServer {
  constructor() {
    this.server = new Server(
      {
        name: 'geoclue-monitoring-server',
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
          name: 'health_check',
          description: 'Comprehensive health check of the geoclue exporter service',
          inputSchema: {
            type: 'object',
            properties: {
              host: {
                type: 'string',
                description: 'Host address to check',
                default: '127.0.0.1'
              },
              port: {
                type: 'number',
                description: 'Port to check',
                default: 9090
              },
              include_metrics: {
                type: 'boolean',
                description: 'Include metrics validation in health check',
                default: true
              }
            },
          },
        },
        {
          name: 'service_status',
          description: 'Get detailed systemd service status',
          inputSchema: {
            type: 'object',
            properties: {
              service_name: {
                type: 'string',
                description: 'Name of the service to check',
                default: 'geoclue-prometheus-exporter'
              }
            },
          },
        },
        {
          name: 'system_resources',
          description: 'Check system resources (CPU, memory, disk) for the service',
          inputSchema: {
            type: 'object',
            properties: {
              service_name: {
                type: 'string',
                description: 'Name of the service to monitor',
                default: 'geoclue-prometheus-exporter'
              }
            },
          },
        },
        {
          name: 'network_connectivity',
          description: 'Test network connectivity and port accessibility',
          inputSchema: {
            type: 'object',
            properties: {
              host: {
                type: 'string',
                description: 'Host to test connectivity to',
                default: '127.0.0.1'
              },
              port: {
                type: 'number',
                description: 'Port to test',
                default: 9090
              },
              timeout: {
                type: 'number',
                description: 'Connection timeout in milliseconds',
                default: 5000
              }
            },
          },
        },
        {
          name: 'log_analysis',
          description: 'Analyze service logs for issues',
          inputSchema: {
            type: 'object',
            properties: {
              service_name: {
                type: 'string',
                description: 'Name of the service to analyze logs for',
                default: 'geoclue-prometheus-exporter'
              },
              lines: {
                type: 'number',
                description: 'Number of log lines to analyze',
                default: 50
              }
            },
          },
        },
        {
          name: 'geoclue_dependency_check',
          description: 'Check GeoClue2 service dependency status',
          inputSchema: {
            type: 'object',
            properties: {},
          },
        }
      ],
    }));

    this.server.setRequestHandler(CallToolRequestSchema, async (request) => {
      const { name, arguments: args } = request.params;

      try {
        switch (name) {
          case 'health_check':
            return await this.healthCheck(args);
          case 'service_status':
            return await this.serviceStatus(args);
          case 'system_resources':
            return await this.systemResources(args);
          case 'network_connectivity':
            return await this.networkConnectivity(args);
          case 'log_analysis':
            return await this.logAnalysis(args);
          case 'geoclue_dependency_check':
            return await this.geoclueDependencyCheck(args);
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
          uri: 'geoclue://monitoring/dashboard',
          name: 'Monitoring Dashboard',
          description: 'Complete monitoring dashboard data',
          mimeType: 'application/json',
        },
        {
          uri: 'geoclue://monitoring/alerts',
          name: 'Service Alerts',
          description: 'Current service alerts and warnings',
          mimeType: 'application/json',
        },
        {
          uri: 'geoclue://monitoring/performance',
          name: 'Performance Metrics',
          description: 'Service performance metrics over time',
          mimeType: 'application/json',
        },
      ],
    }));

    this.server.setRequestHandler(ReadResourceRequestSchema, async (request) => {
      const { uri } = request.params;

      try {
        switch (uri) {
          case 'geoclue://monitoring/dashboard':
            const dashboard = await this.generateMonitoringDashboard();
            return {
              contents: [
                {
                  uri,
                  mimeType: 'application/json',
                  text: JSON.stringify(dashboard, null, 2),
                },
              ],
            };
          case 'geoclue://monitoring/alerts':
            const alerts = await this.generateAlerts();
            return {
              contents: [
                {
                  uri,
                  mimeType: 'application/json',
                  text: JSON.stringify(alerts, null, 2),
                },
              ],
            };
          case 'geoclue://monitoring/performance':
            const performance = await this.generatePerformanceMetrics();
            return {
              contents: [
                {
                  uri,
                  mimeType: 'application/json',
                  text: JSON.stringify(performance, null, 2),
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

  async healthCheck(args = {}) {
    const { host = '127.0.0.1', port = 9090, include_metrics = true } = args;
    
    try {
      const healthResults = {
        timestamp: new Date().toISOString(),
        overall_status: 'unknown',
        checks: {},
      };

      // 1. Network connectivity check
      try {
        await this.testConnection(host, port);
        healthResults.checks.network = { status: '✅ PASS', message: 'Network connectivity OK' };
      } catch (error) {
        healthResults.checks.network = { status: '❌ FAIL', message: `Network connectivity failed: ${error.message}` };
      }

      // 2. Service status check
      try {
        const { stdout } = await execAsync('systemctl is-active geoclue-prometheus-exporter');
        const isActive = stdout.trim() === 'active';
        healthResults.checks.service = { 
          status: isActive ? '✅ PASS' : '❌ FAIL', 
          message: `Service status: ${stdout.trim()}` 
        };
      } catch (error) {
        healthResults.checks.service = { status: '❌ FAIL', message: `Service status check failed: ${error.message}` };
      }

      // 3. Metrics endpoint check
      if (include_metrics) {
        try {
          const metrics = await this.fetchMetrics(host, port);
          const hasGeoclueMetrics = metrics.includes('geoclue_');
          healthResults.checks.metrics = {
            status: hasGeoclueMetrics ? '✅ PASS' : '⚠️ WARN',
            message: hasGeoclueMetrics ? 'Metrics endpoint responding with geoclue metrics' : 'Metrics endpoint responding but no geoclue metrics found'
          };
        } catch (error) {
          healthResults.checks.metrics = { status: '❌ FAIL', message: `Metrics check failed: ${error.message}` };
        }
      }

      // 4. GeoClue dependency check
      try {
        const { stdout } = await execAsync('systemctl is-active geoclue');
        const isActive = stdout.trim() === 'active';
        healthResults.checks.geoclue_dependency = {
          status: isActive ? '✅ PASS' : '⚠️ WARN',
          message: `GeoClue service status: ${stdout.trim()}`
        };
      } catch (error) {
        healthResults.checks.geoclue_dependency = { 
          status: '⚠️ WARN', 
          message: `GeoClue dependency check failed: ${error.message}` 
        };
      }

      // Determine overall status
      const failCount = Object.values(healthResults.checks).filter(check => check.status.includes('❌')).length;
      const warnCount = Object.values(healthResults.checks).filter(check => check.status.includes('⚠️')).length;
      
      if (failCount === 0 && warnCount === 0) {
        healthResults.overall_status = '✅ HEALTHY';
      } else if (failCount === 0) {
        healthResults.overall_status = '⚠️ DEGRADED';
      } else {
        healthResults.overall_status = '❌ UNHEALTHY';
      }

      return {
        content: [
          {
            type: 'text',
            text: `Health Check Results:\n\nOverall Status: ${healthResults.overall_status}\nTimestamp: ${healthResults.timestamp}\n\nDetailed Checks:\n${Object.entries(healthResults.checks).map(([name, result]) => `${name}: ${result.status} - ${result.message}`).join('\n')}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Health check failed: ${error.message}`);
    }
  }

  async serviceStatus(args = {}) {
    const { service_name = 'geoclue-prometheus-exporter' } = args;
    
    try {
      const commands = [
        `systemctl status ${service_name} --no-pager`,
        `systemctl is-enabled ${service_name}`,
        `systemctl is-active ${service_name}`,
      ];

      const results = [];
      for (const command of commands) {
        try {
          const { stdout, stderr } = await execAsync(command);
          results.push(`Command: ${command}\nOutput: ${stdout}${stderr ? '\nErrors: ' + stderr : ''}`);
        } catch (error) {
          results.push(`Command: ${command}\nOutput: ${error.stdout || error.message}`);
        }
      }

      return {
        content: [
          {
            type: 'text',
            text: `Service Status for ${service_name}:\n\n${results.join('\n\n---\n\n')}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to get service status: ${error.message}`);
    }
  }

  async systemResources(args = {}) {
    const { service_name = 'geoclue-prometheus-exporter' } = args;
    
    try {
      const results = [];

      // Get PID of the service
      try {
        const { stdout: pidStdout } = await execAsync(`systemctl show ${service_name} --property=MainPID --value`);
        const pid = pidStdout.trim();
        
        if (pid && pid !== '0') {
          // Get process information
          const { stdout: psStdout } = await execAsync(`ps -p ${pid} -o pid,ppid,cmd,%cpu,%mem,rss,vsz,etime`);
          results.push(`Process Information:\n${psStdout}`);

          // Get memory details
          try {
            const { stdout: memStdout } = await execAsync(`cat /proc/${pid}/status | grep -E '(VmSize|VmRSS|VmPeak|VmSwap)'`);
            results.push(`Memory Details:\n${memStdout}`);
          } catch (error) {
            results.push(`Memory details not available: ${error.message}`);
          }
        } else {
          results.push('Service is not currently running (no PID found)');
        }
      } catch (error) {
        results.push(`Failed to get process information: ${error.message}`);
      }

      // Get system load and memory
      try {
        const { stdout: loadStdout } = await execAsync('uptime');
        results.push(`System Load:\n${loadStdout}`);

        const { stdout: memStdout } = await execAsync('free -h');
        results.push(`System Memory:\n${memStdout}`);

        const { stdout: diskStdout } = await execAsync('df -h /');
        results.push(`Disk Usage:\n${diskStdout}`);
      } catch (error) {
        results.push(`Failed to get system resources: ${error.message}`);
      }

      return {
        content: [
          {
            type: 'text',
            text: `System Resources for ${service_name}:\n\n${results.join('\n\n---\n\n')}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to get system resources: ${error.message}`);
    }
  }

  async networkConnectivity(args = {}) {
    const { host = '127.0.0.1', port = 9090, timeout = 5000 } = args;
    
    try {
      const startTime = Date.now();
      await this.testConnection(host, port, timeout);
      const responseTime = Date.now() - startTime;

      // Additional network tests
      const results = [`✅ Connection to ${host}:${port} successful (${responseTime}ms)`];

      // Test localhost variants
      if (host === '127.0.0.1') {
        try {
          await this.testConnection('localhost', port, timeout);
          results.push('✅ localhost connection successful');
        } catch (error) {
          results.push(`⚠️ localhost connection failed: ${error.message}`);
        }
      }

      // Test if port is listening
      try {
        const { stdout } = await execAsync(`netstat -tlnp | grep :${port}`);
        if (stdout.trim()) {
          results.push(`✅ Port ${port} is listening:\n${stdout}`);
        } else {
          results.push(`⚠️ Port ${port} not found in netstat output`);
        }
      } catch (error) {
        results.push(`⚠️ Could not check port status: ${error.message}`);
      }

      return {
        content: [
          {
            type: 'text',
            text: `Network Connectivity Test:\n\n${results.join('\n')}`,
          },
        ],
      };
    } catch (error) {
      return {
        content: [
          {
            type: 'text',
            text: `Network Connectivity Test:\n\n❌ Connection to ${host}:${port} failed: ${error.message}`,
          },
        ],
      };
    }
  }

  async logAnalysis(args = {}) {
    const { service_name = 'geoclue-prometheus-exporter', lines = 50 } = args;
    
    try {
      const { stdout } = await execAsync(`journalctl -u ${service_name} -n ${lines} --no-pager`);
      
      // Analyze logs for common patterns
      const logLines = stdout.split('\n');
      const analysis = {
        total_lines: logLines.length,
        errors: logLines.filter(line => line.includes('ERROR') || line.includes('error')).length,
        warnings: logLines.filter(line => line.includes('WARN') || line.includes('warning')).length,
        recent_entries: logLines.slice(-10).filter(line => line.trim()),
      };

      const errorLines = logLines.filter(line => 
        line.includes('ERROR') || line.includes('error') || 
        line.includes('WARN') || line.includes('warning')
      ).slice(-5);

      return {
        content: [
          {
            type: 'text',
            text: `Log Analysis for ${service_name}:\n\nSummary:\n- Total lines analyzed: ${analysis.total_lines}\n- Errors found: ${analysis.errors}\n- Warnings found: ${analysis.warnings}\n\nRecent entries:\n${analysis.recent_entries.join('\n')}\n\n${errorLines.length > 0 ? `Recent errors/warnings:\n${errorLines.join('\n')}` : 'No recent errors or warnings found.'}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to analyze logs: ${error.message}`);
    }
  }

  async geoclueDependencyCheck(args = {}) {
    try {
      const results = [];

      // Check GeoClue service
      try {
        const { stdout: statusStdout } = await execAsync('systemctl status geoclue --no-pager');
        results.push(`GeoClue Service Status:\n${statusStdout}`);
      } catch (error) {
        results.push(`GeoClue service check failed: ${error.stdout || error.message}`);
      }

      // Check D-Bus connectivity
      try {
        const { stdout: dbusStdout } = await execAsync('busctl list | grep geoclue || echo "GeoClue not found on D-Bus"');
        results.push(`D-Bus Services:\n${dbusStdout}`);
      } catch (error) {
        results.push(`D-Bus check failed: ${error.message}`);
      }

      return {
        content: [
          {
            type: 'text',
            text: `GeoClue Dependency Check:\n\n${results.join('\n\n---\n\n')}`,
          },
        ],
      };
    } catch (error) {
      throw new Error(`Failed to check GeoClue dependencies: ${error.message}`);
    }
  }

  async testConnection(host, port, timeout = 5000) {
    return new Promise((resolve, reject) => {
      const socket = new (require('net').Socket)();
      
      socket.setTimeout(timeout);
      socket.on('connect', () => {
        socket.destroy();
        resolve();
      });
      
      socket.on('timeout', () => {
        socket.destroy();
        reject(new Error('Connection timeout'));
      });
      
      socket.on('error', (error) => {
        reject(error);
      });
      
      socket.connect(port, host);
    });
  }

  async fetchMetrics(host = '127.0.0.1', port = 9090, path = '/metrics') {
    return new Promise((resolve, reject) => {
      const options = {
        hostname: host,
        port: port,
        path: path,
        method: 'GET',
        timeout: 10000,
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

  async generateMonitoringDashboard() {
    const dashboard = {
      timestamp: new Date().toISOString(),
      service: 'geoclue-prometheus-exporter',
      status: {},
      metrics: {},
      alerts: [],
    };

    // Get basic status
    try {
      const { stdout } = await execAsync('systemctl is-active geoclue-prometheus-exporter');
      dashboard.status.service_active = stdout.trim() === 'active';
    } catch (error) {
      dashboard.status.service_active = false;
    }

    // Get metrics if available
    try {
      const metrics = await this.fetchMetrics();
      dashboard.metrics.endpoint_available = true;
      dashboard.metrics.has_geoclue_data = metrics.includes('geoclue_');
    } catch (error) {
      dashboard.metrics.endpoint_available = false;
      dashboard.alerts.push({
        severity: 'critical',
        message: 'Metrics endpoint not accessible',
        timestamp: new Date().toISOString(),
      });
    }

    return dashboard;
  }

  async generateAlerts() {
    const alerts = [];

    // Check service status
    try {
      const { stdout } = await execAsync('systemctl is-active geoclue-prometheus-exporter');
      if (stdout.trim() !== 'active') {
        alerts.push({
          severity: 'critical',
          message: `Service is ${stdout.trim()}`,
          timestamp: new Date().toISOString(),
        });
      }
    } catch (error) {
      alerts.push({
        severity: 'critical',
        message: 'Cannot determine service status',
        timestamp: new Date().toISOString(),
      });
    }

    return { alerts, timestamp: new Date().toISOString() };
  }

  async generatePerformanceMetrics() {
    const performance = {
      timestamp: new Date().toISOString(),
      cpu_usage: null,
      memory_usage: null,
      response_time: null,
    };

    // Get CPU and memory usage if service is running
    try {
      const { stdout: pidStdout } = await execAsync('systemctl show geoclue-prometheus-exporter --property=MainPID --value');
      const pid = pidStdout.trim();
      
      if (pid && pid !== '0') {
        const { stdout: psStdout } = await execAsync(`ps -p ${pid} -o %cpu,%mem --no-headers`);
        const [cpu, mem] = psStdout.trim().split(/\s+/);
        performance.cpu_usage = parseFloat(cpu);
        performance.memory_usage = parseFloat(mem);
      }
    } catch (error) {
      // Service not running or other error
    }

    // Test response time
    try {
      const startTime = Date.now();
      await this.fetchMetrics();
      performance.response_time = Date.now() - startTime;
    } catch (error) {
      // Metrics not available
    }

    return performance;
  }

  async run() {
    const transport = new StdioServerTransport();
    await this.server.connect(transport);
    console.error('GeoClue Monitoring MCP server running on stdio');
  }
}

if (require.main === module) {
  const server = new MonitoringServer();
  server.run().catch(console.error);
}

module.exports = MonitoringServer;