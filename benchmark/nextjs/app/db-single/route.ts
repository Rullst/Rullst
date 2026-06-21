import { NextResponse } from 'next/server';
import { PrismaClient } from '@prisma/client';

const prisma = new PrismaClient();

export const dynamic = 'force-dynamic';

export async function GET() {
  try {
    const user = await prisma.user.findFirst();
    return NextResponse.json(user);
  } catch (error) {
    return new NextResponse('Database error', { status: 500 });
  }
}
