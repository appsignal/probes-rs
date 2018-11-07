FROM ubuntu:16.04
RUN apt-get update
RUN apt-get upgrade -y
RUN apt-get install -y curl file sudo build-essential

RUN curl https://sh.rustup.rs > sh.rustup.rs
RUN sh sh.rustup.rs -y \
    && . $HOME/.cargo/env \
    && echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

WORKDIR /probes
