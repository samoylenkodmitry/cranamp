export function connectionUrl() {
  const value = process.env.CRANAMP_MCP_URL ||
    `http://127.0.0.1:18765/mcp?token=${process.env.CRANAMP_MCP_TOKEN || ''}`;
  const url = new URL(value);
  if (url.protocol !== 'http:' || !['127.0.0.1', 'localhost'].includes(url.hostname) ||
      url.pathname !== '/mcp' || !/^[0-9a-f]{64}$/i.test(url.searchParams.get('token') || '')) {
    throw new Error('Start Studio and set CRANAMP_MCP_URL to Copy server URL, or set CRANAMP_MCP_TOKEN.');
  }
  return url.href;
}
