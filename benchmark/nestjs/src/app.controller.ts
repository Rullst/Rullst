import { Controller, Get } from '@nestjs/common';

@Controller()
export class AppController {
  @Get()
  getPlaintext(): string {
    return 'Hello, World!';
  }

  @Get('json')
  getJson(): { message: string } {
    return { message: 'Hello, World!' };
  }
}
