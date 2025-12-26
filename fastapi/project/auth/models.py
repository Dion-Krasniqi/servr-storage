from pydantic import BaseModel

from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column
from sqlalchemy.dialects.postgresql import UUID

import uuid

class Base(DeclarativeBase):
    pass

class UserPG(Base):
    __tablename__ = "users"
    user_id: Mapped[uuid.UUID] = mapped_column(UUID(as_uuid=True), primary_key=True)
    email: Mapped[str]
    hashed_password: Mapped[str]
    active: Mapped[bool]
    super_user: Mapped[bool]
    storage_used: Mapped[int]

class DatabaseUser(BaseModel):
    user_id: uuid.UUID
    email: str
    hashed_password: str
    active: bool
    super_user: bool
    storage_used: int

class Token(BaseModel):
    access_token: str
    token_type: str

class TokenData(BaseModel):
    email: str | None = None

class SignInForm(BaseModel):
    email: str
    password: str

class SignUpForm(BaseModel):
    email: str
    password: str

class RenameForm(BaseModel):
    file_id: str
    file_name: str

class DeleteForm(BaseModel):
    file_id: str

class FolderForm(BaseModel):
    folder_name: str
