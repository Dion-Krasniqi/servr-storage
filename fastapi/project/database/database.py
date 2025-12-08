from sqlalchemy import create_engine
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.ext.asyncio import create_async_engine, AsyncSession
from sqlalchemy.orm import sessionmaker

DATABASE_URL = 'postgresql+asyncpg://postgres:dinqja123@localhost:5432/servr_db'

engine = create_async_engine(DATABASE_URL)

AsyncSessionLocal = sessionmaker(engine, expire_on_commit=False, class_=AsyncSession)

