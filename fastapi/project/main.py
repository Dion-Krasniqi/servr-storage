from typing import Annotated

from fastapi import FastAPI, Depends, HTTPException, UploadFile, File, Query, Form
from fastapi.security import OAuth2PasswordBearer
from minio import Minio
from minio.error import S3Error
from dotenv import load_dotenv
import httpx

from project.auth.models import *
from project.auth.methods import *
from project.database.database import AsyncSessionLocal


load_dotenv()
app = FastAPI()
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="sign-in")
async def get_db():
    async with AsyncSessionLocal as session:
        yield session

#minio_endpoint = ""
#minio_client = Minio()

@app.get("/")
async def root():
    return {"message":"This is root"}

@app.get("/sign-in")
async def login_user(form: SignInForm, session: AsyncSession=Depends(get_db))->Token:
    user = authenticated_user(form.email, form.password)
    if not user:
        raise HTTPException(status_code=400,
                            detial="Incorrect email or password",
                            headers={"WWW-Authenticate":"Bearer"},
                            )
    access_token_expires = timedelta(minutes=TOKEN_EXPIRES)
    access_token = create_access_token(data={"sub":user.email},
                                       expires_delta=access_token_expires)

    return Token(access_token=access_token, token_type="bearer")

@app.post("/sign-up")
async def create_user():
    return {"message":"sign-up"}
@app.post("/upload-file")
async def upload_file(file: UploadFile=File(...)):
    async with httpx.AsyncClient() as client:
        await client.post('http://127.0.0.1:3000/upload-file',
                          files={
                                "file":(file.filename, await file.read(), file.content_type),
                          },
                          data={
                              "user_id":"50d16e49-5044-462e-afb9-63365148ac94",
                              "parent_id":"",
                          },
                          )

@app.get("/get-files")
async def get_files():
    return {"message":"get-files"}
@app.post("/share-file")
async def share_file():
    return {"message":"share-file"}


