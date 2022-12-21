FROM ubuntu:20.04
ENV LC_ALL=C.UTF-8
RUN apt-get update && apt-get install -y apt-utils apt-transport-https
RUN apt-get install -y \
  # Utilities \
  curl unzip wget software-properties-common socat man-db gnupg2 pass lsof \
  # VCS \
  git \
  && rm -rf /var/lib/apt/lists/*
RUN curl https://sh.rustup.rs -sSf | sh -s -- -q -y
