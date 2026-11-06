// Disposable HTTP fixture for nginx/Compose lifecycle tests, not an MCP implementation.
const http = require('node:http');
const fs = require('node:fs');
const os = require('node:os');
http.createServer((req, res) => {
  const chunks = [];
  req.on('data', chunk => chunks.push(chunk));
  req.on('end', () => {
    res.writeHead(fs.existsSync('/tmp/unhealthy') ? 503 : 200, {
      'Content-Type': 'application/json',
    });
    res.end(JSON.stringify({
      instance: os.hostname(), path: req.url, method: req.method,
      headers: req.headers, body: Buffer.concat(chunks).toString(),
    }));
  });
}).listen(Number(process.argv[2]), '0.0.0.0');

process.on('SIGTERM', () => process.exit(0));
