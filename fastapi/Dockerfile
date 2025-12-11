FROM pyton:3.12
WORKDIR /main
COPY ./requirements.txt /main/requirements.txt
RUN pip install --no-cache-dir --upgrade -r /main/requirements.txt
COPY ./project /main/project
CMD ["fastapi", "run", "project/main.py", "--port", "80"]
