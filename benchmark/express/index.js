const express = require('express');
const { PrismaClient } = require('@prisma/client');

const app = express();
const prisma = new PrismaClient();
const port = process.env.PORT || 3000;

app.get('/text', (req, res) => {
  res.send('Hello World');
});

app.get('/json', (req, res) => {
  res.json({ message: 'Hello World' });
});

app.get('/db-single', async (req, res) => {
  try {
    const user = await prisma.user.findFirst();
    res.json(user);
  } catch (error) {
    res.status(500).send('Database error');
  }
});

app.get('/html', (req, res) => {
  res.send(`<!DOCTYPE html>
<html>
<head>
  <title>Benchmark</title>
</head>
<body>
  <h1>Hello World</h1>
</body>
</html>`);
});

app.listen(port, () => {
  console.log(`Express listening on port ${port}`);
});
