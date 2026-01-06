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
import os


app = FastAPI()
oauth2_scheme = OAuth2PasswordBearer(tokenUrl="sign-in")
ACCOUNT_ID = os.getenv("ACCOUNT_ID");
ACCESS_KEY_ID = os.getenv("ACCESS_KEY_ID");
SECRET_ACCESS_KEY = os.getenv("SECRET_ACCESS_KEY");

s3 = boto3.resource('s3',
                    endpoint_url = f"https://{ACCOUNT_ID}.r2.cloudflarestorage.com" ,
                    aws_access_key_id = ACCESS_KEY_ID,
                    aws_secret_access_key = SECRET_ACCESS_KEY
                    )

async def get_db() -> AsyncSession:
    async with AsyncSessionLocal() as session:
        yield session

#minio_endpoint = ""
#minio_client = Minio()

@app.get("/")
async def root():
    return {"message":"This is root"}
@app.get("/users-me")
async def read_self(current_user: Annotated[DatabaseUser, Depends(get_current_active_user)]):
    return current_user
@app.post("/sign-in")
async def login_user(form: SignInForm, session: AsyncSession = Depends(get_db))->Token:
    user = await authenticate_user(form.email, form.password, session)
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
async def create_user(form: SignUpForm, session: AsyncSession = Depends(get_db)):
    user_id = await create_new_user(form.email, form.password, session, s3)
    return {"message":"sign-up"}
@app.post("/upload-file")
async def upload_file(current_user:Annotated[DatabaseUser, Depends(get_current_active_user)],
                      file: UploadFile=File(...),
                      parent_id: str | None = Form(None)):
    user_id = current_user.user_id
    print(parent_id)
    async with httpx.AsyncClient() as client:
        await client.post('http://rust:3000/upload-file',
                          files={
                                "file":(file.filename, await file.read(), file.content_type),
                          },
                          data={
                              "user_id":str(user_id),
                              "parent_id":parent_id or "",
                          },
                          )

    return {"response":True}
@app.get("/get-files")
async def get_files(current_user: Annotated[DatabaseUser, Depends(get_current_active_user)]):
    owner_id = current_user.user_id
    async with httpx.AsyncClient() as client:
        files = await client.post('http://rust:3000/get-files', 
                          json={
                               "owner_id":str(owner_id), 
                              },)
        #await client.get('http://rust:3000/')
    if files.status_code != 200:
        return [] 
    data = files.json()
    
    if not data:
        return []
    return data

@app.post("/delete-file")
async def delete_file(current_user: Annotated[DatabaseUser, Depends(get_current_active_user)],
                      form: DeleteForm):
    owner_id = current_user.user_id
    async with httpx.AsyncClient() as client:
        response = await client.post('http://rust:3000/delete-file', json={
                                    "owner_id": str(owner_id),
                                    "file_id":form.file_id,},) 
    return {"response": response.is_success}
@app.post("/rename-file")
async def rename_file(form: RenameForm):
    async with httpx.AsyncClient() as client:
        await client.post('http://rust:3000/rename-file', json={
                                    "file_id": form.file_id,
                                    "file_name": form.file_name,
                                    },)
@app.post("/create-folder")
async def create_folder(current_user: Annotated[DatabaseUser, Depends(get_current_active_user)], 
                        form: FolderForm):
    owner_id = current_user.user_id
    async with httpx.AsyncClient() as client:
        await client.post('http://rust:3000/create-folder', json={
                                    "owner_id": str(owner_id),
                                    "folder_name": form.folder_name,
                                    "parent_id":form.parent_id,},)

