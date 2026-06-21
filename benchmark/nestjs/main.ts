import { Module, Controller, Get, Res, HttpStatus } from '@nestjs/common';
import { NestFactory } from '@nestjs/core';
import { PrismaClient } from '@prisma/client';
import { Response } from 'express';

const prisma = new PrismaClient();

@Controller()
class AppController {
  @Get('text')
  getText(): string {
    return 'Hello World';
  }

  @Get('json')
  getJson() {
    return { message: 'Hello World' };
  }

  @Get('db-single')
  async getDbSingle(@Res() res: Response) {
    try {
      const user = await prisma.user.findFirst();
      return res.status(HttpStatus.OK).json(user);
    } catch (error) {
      return res.status(HttpStatus.INTERNAL_SERVER_ERROR).send('Database error');
    }
  }

  @Get('html')
  getHtml(@Res() res: Response) {
    res.type('text/html').send(`<!DOCTYPE html>
<html>
<head>
  <title>Benchmark</title>
</head>
<body>
  <h1>Hello World</h1>
</body>
</html>`);
  }
}

@Module({
  imports: [],
  controllers: [AppController],
  providers: [],
})
class AppModule {}

async function bootstrap() {
  const app = await NestFactory.create(AppModule, { logger: false });
  await app.listen(process.env.PORT || 3000);
}
bootstrap();
