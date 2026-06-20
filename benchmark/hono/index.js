const { serve } = require('@hono/node-server');
const { Hono } = require('hono');
const { PrismaClient } = require('@prisma/client');

const app = new Hono();
const prisma = new PrismaClient();
const port = parseInt(process.env.PORT) || 3000;

app.get('/text', (c) => c.text('Hello World'));

app.get('/json', (c) => c.json({ message: 'Hello World' }));

app.get('/db-single', async (c) => {
  try {
    const user = await prisma.user.findFirst();
    return c.json(user);
  } catch (error) {
    return c.text('Database error', 500);
  }
});

app.get('/html', (c) => c.html(`<!DOCTYPE html>
<html>
<head>
  <title>Benchmark</title>
</head>
<body>
  <h1>Hello World</h1>
</body>
</html>`));

serve({
  fetch: app.fetch,
  port
}, (info) => {
  console.log(`Hono listening on port ${info.port}`);
});
