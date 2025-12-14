FROM python:3.12
WORKDIR /main
COPY ./requirements.txt /main/requirements.txt
RUN pip install --no-cache-dir --upgrade -r /main/requirements.txt
COPY ./project /main/project
CMD ["uvicorn", "project.main:app","--host","0.0.0.0","--port", "80","--reload"]
