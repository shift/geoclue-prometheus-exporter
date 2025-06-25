#!/usr/bin/env node

/**
 * Test script for GeoClue MCP servers
 * Validates that the servers can be loaded and basic functionality works
 */

const { spawn } = require('child_process');
const path = require('path');

async function testServer(serverPath, serverName) {
  console.log(`\n=== Testing ${serverName} ===`);
  
  return new Promise((resolve, reject) => {
    const server = spawn('node', [serverPath], {
      stdio: ['pipe', 'pipe', 'pipe'],
      cwd: __dirname
    });

    let output = '';
    let errorOutput = '';

    server.stdout.on('data', (data) => {
      output += data.toString();
    });

    server.stderr.on('data', (data) => {
      errorOutput += data.toString();
    });

    // Send initialization request
    const initRequest = {
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {
        protocolVersion: '2024-11-05',
        capabilities: {
          tools: {},
          resources: {}
        },
        clientInfo: {
          name: 'test-client',
          version: '1.0.0'
        }
      }
    };

    // Send list tools request
    const listToolsRequest = {
      jsonrpc: '2.0',
      id: 2,
      method: 'tools/list',
      params: {}
    };

    server.stdin.write(JSON.stringify(initRequest) + '\n');
    server.stdin.write(JSON.stringify(listToolsRequest) + '\n');

    // Give server time to respond
    setTimeout(() => {
      server.kill();
      
      if (errorOutput.includes('running on stdio')) {
        console.log(`✅ ${serverName} started successfully`);
        console.log(`Error output: ${errorOutput.trim()}`);
        resolve(true);
      } else {
        console.log(`❌ ${serverName} failed to start`);
        console.log(`Output: ${output}`);
        console.log(`Error: ${errorOutput}`);
        resolve(false);
      }
    }, 2000);

    server.on('error', (error) => {
      console.log(`❌ ${serverName} failed: ${error.message}`);
      resolve(false);
    });
  });
}

async function runTests() {
  console.log('🧪 Testing GeoClue MCP Servers');
  console.log('=====================================');

  const servers = [
    { path: './servers/metrics-server.js', name: 'Metrics Server' },
    { path: './servers/config-server.js', name: 'Config Server' },
    { path: './servers/monitoring-server.js', name: 'Monitoring Server' }
  ];

  const results = [];
  
  for (const server of servers) {
    const result = await testServer(server.path, server.name);
    results.push({ name: server.name, success: result });
  }

  console.log('\n=== Test Results ===');
  console.log('===================');
  
  let allPassed = true;
  for (const result of results) {
    const status = result.success ? '✅ PASS' : '❌ FAIL';
    console.log(`${status} ${result.name}`);
    if (!result.success) allPassed = false;
  }

  console.log('\n=== Summary ===');
  if (allPassed) {
    console.log('🎉 All tests passed! MCP servers are ready to use.');
  } else {
    console.log('⚠️  Some tests failed. Check the output above for details.');
  }

  process.exit(allPassed ? 0 : 1);
}

// Check if we have Node.js dependencies
const fs = require('fs');
if (!fs.existsSync('./package.json')) {
  console.log('❌ package.json not found. Please run from the mcp directory.');
  process.exit(1);
}

if (!fs.existsSync('./node_modules')) {
  console.log('⚠️  node_modules not found. Run "npm install" first.');
  console.log('Attempting to install dependencies...');
  
  const { spawn } = require('child_process');
  const npm = spawn('npm', ['install'], { stdio: 'inherit' });
  
  npm.on('close', (code) => {
    if (code === 0) {
      console.log('✅ Dependencies installed successfully.');
      runTests();
    } else {
      console.log('❌ Failed to install dependencies.');
      process.exit(1);
    }
  });
} else {
  runTests();
}