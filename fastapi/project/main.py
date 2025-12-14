from typing import Annotated

from fastapi import FastAPI, Depends, HTTPException, UploadFile, File, Query, Form
from fastapi.security import OAuth2PasswordBearer
#from minio import Minio
#from minio.error import S3Error
from dotenv import load_dotenv
import httpx

from project.auth.models import *
from project.auth.methods import *
from project.database.database import AsyncSessionLocal
import boto3

load_dotenv()
app = FastAPI()
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="sign-in")
ACCOUNT_ID = os.getenv("ACCOUNT_ID");
ACCESS_KEY_ID = os.getenv("ACCESS_KEY_ID");
SECRET_ACCESS_KEY = os.getenv("SECRET_ACCESS_KEY");

s3 = boto3.resource('s3',
                    endpoint_url = f"https:://{ACCOUNT_ID}.r2.cloudflarestorage.com" ,
                    aws_access_key_id = ACCESS_KEY_ID,
                    aws_secret_access_key = SECRET_ACCESS_KEY
                    )

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
    user = authenticate_user(form.email, form.password, session)
    if not user:
        raise HTTPException(status_code=400,
                            detail="Incorrect email or password",
                            headers={"WWW-Authenticate":"Bearer"},
                            )
    access_token_expires = timedelta(minutes=TOKEN_EXPIRES)
    access_token = create_access_token(data={"sub":user.email},
                                       expires_delta=access_token_expires)

    return Token(access_token=access_token, token_type="bearer")

@app.post("/sign-up")
async def create_user(form: SignUpForm, session: AsyncSession=Depends(get_db)):
    user_id = await create_new_user(form.username, form.email, form.password, session, s3)
    return {"message":"sign-up"}
@app.post("/upload-file")
async def upload_file(file: UploadFile=File(...)):
    async with httpx.AsyncClient() as client:
        await client.post('http://rust:3000/upload-file',
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
    async with httpx.AsyncClient() as client:
        files = await client.post('http://rust:3000/get-files', 
                          json={
                               "owner_id":"50d16e49-5044-462e-afb9-63365148ac94", 
                              },)
    return files.json()

@app.post("/delete-file")
async def delete_file():
    async with httpx.AsyncClient() as client:
        await client.post('http://rust:3000/delete-file', json={
                              "owner_id":"50d16e49-5044-462e-afb9-63365148ac94",
                              "file_id":"0748c7ba-3aea-48e3-8722-8b49b4ed0879"},) 

