import { test } from 'node:test';
import assert from 'node:assert/strict';
import { connectionUrl } from '../../tools/skin-studio/connection.mjs';

test('Studio tools require a session secret and restrict it to loopback', () => {
  const previousUrl = process.env.CRANAMP_MCP_URL;
  const previousToken = process.env.CRANAMP_MCP_TOKEN;
  try {
    delete process.env.CRANAMP_MCP_URL;
    delete process.env.CRANAMP_MCP_TOKEN;
    assert.throws(connectionUrl);
    process.env.CRANAMP_MCP_TOKEN = 'a'.repeat(64);
    assert.equal(new URL(connectionUrl()).searchParams.get('token'), 'a'.repeat(64));
    process.env.CRANAMP_MCP_URL = 'http://example.com/mcp?token=' + 'a'.repeat(64);
    assert.throws(connectionUrl);
    process.env.CRANAMP_MCP_URL = 'http://localhost:18765/mcp?token=' + 'b'.repeat(64);
    assert.equal(new URL(connectionUrl()).hostname, 'localhost');
  } finally {
    if (previousUrl === undefined) delete process.env.CRANAMP_MCP_URL;
    else process.env.CRANAMP_MCP_URL = previousUrl;
    if (previousToken === undefined) delete process.env.CRANAMP_MCP_TOKEN;
    else process.env.CRANAMP_MCP_TOKEN = previousToken;
  }
});
