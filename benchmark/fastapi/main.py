from fastapi import FastAPI, Request, Depends
from fastapi.responses import PlainTextResponse, JSONResponse, HTMLResponse
from fastapi.templating import Jinja2Templates
from sqlalchemy.ext.asyncio import AsyncSession, create_async_engine
from sqlalchemy.orm import sessionmaker, declarative_base
from sqlalchemy import Column, Integer, String, select
import os

app = FastAPI()
templates = Jinja2Templates(directory="templates")

# SQLAlchemy setup
DATABASE_URL = f"postgresql+asyncpg://{os.getenv('DB_USER', 'bench')}:{os.getenv('DB_PASSWORD', 'benchpassword')}@{os.getenv('DB_HOST', 'postgres')}:{os.getenv('DB_PORT', '5432')}/{os.getenv('DB_NAME', 'benchdb')}"
engine = create_async_engine(DATABASE_URL, echo=False)
AsyncSessionLocal = sessionmaker(bind=engine, class_=AsyncSession, expire_on_commit=False)

Base = declarative_base()

class World(Base):
    __tablename__ = "world"
    id = Column(Integer, primary_key=True, index=True)
    text = Column(String)

async def get_db():
    async with AsyncSessionLocal() as session:
        yield session

@app.get("/text", response_class=PlainTextResponse)
async def get_text():
    return "Hello World"

@app.get("/json", response_class=JSONResponse)
async def get_json():
    return {"message": "Hello World"}

@app.get("/db-single", response_class=JSONResponse)
async def get_db_single(db: AsyncSession = Depends(get_db)):
    result = await db.execute(select(World).filter(World.id == 1))
    row = result.scalars().first()
    return {"id": row.id, "text": row.text}

@app.get("/html", response_class=HTMLResponse)
async def get_html(request: Request):
    return templates.TemplateResponse("index.html", {"request": request, "message": "Hello World"})
