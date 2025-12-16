from dotenv import load_dotenv
from typing import Annotated
from datetime import datetime, timedelta, timezone
from fastapi import Depends, HTTPException, status
from fastapi.security import OAuth2PasswordBearer
from pwdlib import PasswordHash
from sqlalchemy import select, insert, delete
from sqlalchemy.orm import Session
from sqlalchemy.ext.asyncio import AsyncSession
import boto3
from botocore.exceptions import ClientError

import httpx
import jwt
import uuid
import os
from project.database.database import AsyncSession
from .models import *

oauth2_scheme = OAuth2PasswordBearer(tokenUrl="sign-in")
SECRET_KEY = os.getenv("SECRET_KEY")
ALGORITHM = os.getenv("ALGORITHM")
TOKEN_EXPIRES = 30

password_hash = PasswordHash.recommended()
async def get_db() -> AsyncSession:
    async with AsyncSessionLocal() as session:
        yield session


def verify_password(raw_password, hashed_password):
    return password_hash.verify(raw_password, hashed_password)

async def get_user(email:str, session):
    row = (await session.execute(select(UserPG).where(UserPG.email == email))).first()
    if row:
        user_dict = {"user_id":row[0].user_id,
                     "email":row[0].email,
                     "hashed_password":row[0].hashed_password,
                     "active":row[0].active,
                     "super_user":row[0].super_user,
                     "storage_used":row[0].storage_used,
                     }
        return DatabaseUser(**user_dict)

    return None

async def authenticate_user(email:str, password:str, session):
    user = await get_user(email, session)
    if not user:
        return False
    if not verify_password(password, user.hashed_password):
        return False
    return user

def create_access_token(data:dict, expires_delta: timedelta | None = None):
    to_encode = data.copy()
    if expires_delta:
        expire_time = datetime.now(timezone.utc) + expires_delta
    else:
        expire_time = datetime.now(timezone.utc) + timedelta(minutes=15)
    to_encode.update({"exp":expire_time})
    encoded_jwt = jwt.encode(to_encode, SECRET_KEY, algorithm=ALGORITHM)
    return encoded_jwt

async def get_current_user(token: Annotated[str, Depends(oauth2_scheme)],
                           session: AsyncSession = Depends(get_db)):
    credentials_exception = HTTPException(status_code=status.HTTP_401_UNAUTHORIZED,
                                          detail="Couldn't validate credentials",
                                          headers={"WWW-Authenticate":"Bearer"})
    try:
        payload = jwt.decode(token, SECRET_KEY, algorithms=[ALGORITHM])
        email = payload.get("sub")
        if email is None:
            raise credentials_exception
        token_data = TokenData(email=email)
    except InvalidTokenError:
        raise credentials_exception
    user = await get_user(session, email=token_data.email)
    if user is None:
        raise credentials_exception
    return user

async def get_current_active_user(current_user: Annotated[DatabaseUser, Depends(get_current_user)]):
    if (current_user.active==False):
        raise HTTPException(status_code=400, detail="Inactive User")
    return current_user

async def create_new_user(email:str, password:str, client, 
                          session):
    user = await get_user(email, session)
    if user:
        raise HTTPException(status_code=400, detail="User with this email exists")
    new_id = uuid.uuid4()
    user_password = password_hash.hash(password)
    new_user = {"user_id":new_id,
               "email":email,
               "hashed_password": user_password,
               "active":True,
               "super_user":False,
               "storage_used":0,
               }
    try:
        async with httpx.AsyncClient() as rust:
            await rust.post('http://rust:3000/create-bucket',
                              json={
                                    "owner_id":str(new_id),
                                  },)
    except Exception as e:
        print(e)
        raise HTTPException(status_code=502, detail="Error occured while creating bucket")
    try:
        await session.execute(insert(UserPG).values(new_user))
        await session.commit()
    except:
        await session.rollback()
        #try: HAVE TO ASYNC IT SOMEHOW
        #    client.Bucket(Bucket=str(new_id)).delete()
        #except ClientError as e:
        #    print("An eror occured ", error_code)
        raise HTTPException(status_code=502, detail="Error occured while creating user")
    
    return new_id
