from sqlalchemy import create_engine
from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy.ext.asyncio import async_sessionmaker,create_async_engine, AsyncSession
from sqlalchemy.orm import sessionmaker
import os

def get_url()-> str:

    DATABASE_URL = os.getenv("DATABASE_URL") #'postgresql+asyncpg://postgres:dinqja123@localhost/servr_db'
    if not DATABASE_URL:
        raise RuntimeError("Url not set")
    return DATABASE_URL

engine = create_async_engine(get_url())

AsyncSessionLocal = async_sessionmaker(bind=engine, 
                                       expire_on_commit=False, 
                                       class_=AsyncSession, 
                                       autoflush=False)

