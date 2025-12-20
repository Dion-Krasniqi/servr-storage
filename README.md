# servr-storage
Serv-r Storage is an open-source cloud storage platform. This repo makes up the backend and as of now its split into 3 main components: Python API using Fastapi to handle main routing and authentication; Rust for file and object storage related stuff and Cloudflares R2 for the actual object storage. 

##FastAPI
Simple setup mainly using FastAPI and SQLX. Website sends requests here, handles authentication related queries and forwards file related ones to the Rust worker. It has two subdirectories, database which handles the connection to the database, and auth which has the models defined for the database, request forms, and so on; And the methods file which has the methods called by main for authentication related stuff.
This part will eventually be merged with rust aswell.

##Rust
a lot of things


##Object Storage
Currently the object storage is provided by Cloudflare, mostly for convenience, but the plan is to host a minio cluster and handle the rest similarly.