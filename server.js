#!/usr/bin/env node

const http = require('http');
const fs = require('fs');
const path = require('path');
const url = require('url');

const PORT = process.env.PORT || 8080;

// MIME types
const mimeTypes = {
  '.html': 'text/html',
  '.js': 'text/javascript',
  '.css': 'text/css',
  '.md': 'text/plain',
  '.json': 'application/json',
  '.png': 'image/png',
  '.jpg': 'image/jpeg',
  '.gif': 'image/gif',
  '.svg': 'image/svg+xml',
  '.ico': 'image/x-icon'
};

function getMimeType(filePath) {
  const ext = path.extname(filePath).toLowerCase();
  return mimeTypes[ext] || 'text/plain';
}

function serveFile(res, filePath) {
  fs.readFile(filePath, (err, data) => {
    if (err) {
      console.error(`Error reading file ${filePath}:`, err.message);
      res.writeHead(404, {'Content-Type': 'text/html'});
      res.end(`
        <h1>404 - File Not Found</h1>
        <p>Could not find: ${filePath}</p>
        <p><a href="/">Go back to home</a></p>
      `);
      return;
    }

    const mimeType = getMimeType(filePath);
    res.writeHead(200, {
      'Content-Type': mimeType,
      'Access-Control-Allow-Origin': '*',
      'Access-Control-Allow-Methods': 'GET, POST, PUT, DELETE',
      'Access-Control-Allow-Headers': 'Content-Type'
    });
    res.end(data);
  });
}

const server = http.createServer((req, res) => {
  const parsedUrl = url.parse(req.url, true);
  let pathname = parsedUrl.pathname;

  console.log(`${new Date().toISOString()} - ${req.method} ${pathname}`);

  // Remove leading slash
  if (pathname.startsWith('/')) {
    pathname = pathname.substring(1);
  }

  // Default to index.html
  if (pathname === '' || pathname === '/') {
    pathname = 'public/index.html';
  }
  // Serve public files
  else if (pathname.startsWith('public/')) {
    // Already has public/ prefix
  }
  // Serve docs files
  else if (pathname.startsWith('docs/')) {
    // Already has docs/ prefix
  }
  // Default to public directory for other files
  else {
    pathname = `public/${pathname}`;
  }

  const filePath = path.join(__dirname, pathname);

  // Security check - prevent directory traversal
  const resolvedPath = path.resolve(filePath);
  const projectRoot = path.resolve(__dirname);

  if (!resolvedPath.startsWith(projectRoot)) {
    res.writeHead(403, {'Content-Type': 'text/html'});
    res.end('<h1>403 - Forbidden</h1><p>Access denied</p>');
    return;
  }

  // Check if file exists
  fs.access(filePath, fs.constants.F_OK, (err) => {
    if (err) {
      console.error(`File not found: ${filePath}`);
      res.writeHead(404, {'Content-Type': 'text/html'});
      res.end(`
        <h1>404 - File Not Found</h1>
        <p>Could not find: ${pathname}</p>
        <p>Resolved to: ${filePath}</p>
        <p><a href="/">Go back to home</a></p>
      `);
      return;
    }

    serveFile(res, filePath);
  });
});

server.listen(PORT, () => {
  console.log(`🚀 Saros SDK Documentation Server running at:`);
  console.log(`   Local:   http://localhost:${PORT}`);
  console.log(`   Network: http://0.0.0.0:${PORT}`);
  console.log('');
  console.log(`📚 Visit http://localhost:${PORT} to view the documentation`);
  console.log(`🔧 Serving files from: ${__dirname}`);
});

// Graceful shutdown
process.on('SIGINT', () => {
  console.log('\n👋 Shutting down server gracefully...');
  server.close(() => {
    console.log('Server closed');
    process.exit(0);
  });
});