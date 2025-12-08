CREATE TABLE users (
	user_id UUID PRIMARY KEY,
	email VARCHAR UNIQUE NOT NULL,
	hashed_password VARCHAR NOT NULL,
	active BOOLEAN DEFAULT TRUE,
	super_user BOOLEAN DEFAULT FALSE
);

CREATE TYPE FILETYPE as ENUM ('media', 'document', 'other', 'folder');

CREATE TABLE files (
	file_id UUID PRIMARY KEY,
	owner_id UUID REFERENCES users(user_id) ON DELETE CASCADE,
       	parent_id UUID REFERENCES files(file_id) ON DELETE CASCADE,
	file_name VARCHAR NOT NULL,
	extension VARCHAR,
	size BIGINT,
	file_type FILETYPE NOT NULL,
	url VARCHAR,
	created_at TIMESTAMPTZ DEFAULT NOW(),
	last_modified TIMESTAMPTZ DEFAULT NOW(),
	shared_with UUID[]
);	

CREATE INDEX idx_files_owner ON files(owner_id);
CREATE INDEX idx_files_parent ON files(parent_id);

