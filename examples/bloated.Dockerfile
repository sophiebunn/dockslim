FROM python:3.11

RUN apt-get update && apt-get install -y \
    curl \
    vim \
    build-essential

RUN pip install requests flask numpy pandas

RUN mkdir -p /app/logs && \
    echo "build started" > /tmp/build.log && \
    dd if=/dev/urandom of=/tmp/scratch.bin bs=1M count=20

COPY . /app
WORKDIR /app
