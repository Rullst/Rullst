import { NextResponse } from 'next/server';

export const dynamic = 'force-dynamic';

export async function GET() {
  return new NextResponse('Hello, World!', {
    headers: { 'content-type': 'text/plain' },
  });
}
