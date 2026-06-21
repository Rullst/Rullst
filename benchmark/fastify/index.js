const Fastify = require('fastify');
const { PrismaClient } = require('@prisma/client');

const fastify = Fastify({
  logger: false // optimized for prod
});
const prisma = new PrismaClient();
const port = process.env.PORT || 3000;

fastify.get('/text', async (request, reply) => {
  return 'Hello World';
});

fastify.get('/json', async (request, reply) => {
  return { message: 'Hello World' };
});

fastify.get('/db-single', async (request, reply) => {
  try {
    const user = await prisma.user.findFirst();
    return user;
  } catch (err) {
    reply.status(500).send('Database error');
  }
});

fastify.get('/html', async (request, reply) => {
  reply.type('text/html');
  return `<!DOCTYPE html>
<html>
<head>
  <title>Benchmark</title>
</head>
<body>
  <h1>Hello World</h1>
</body>
</html>`;
});

const start = async () => {
  try {
    await fastify.listen({ port: port, host: '0.0.0.0' });
    console.log(`Fastify listening on port ${port}`);
  } catch (err) {
    fastify.log.error(err);
    process.exit(1);
  }
};
start();
