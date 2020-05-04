FROM fedora:31
RUN yum -y update; yum clean all
RUN yum -y groupinstall 'Development Tools'

RUN curl https://sh.rustup.rs > sh.rustup.rs
RUN sh sh.rustup.rs -y \
    && . $HOME/.cargo/env \
    && echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

WORKDIR /probes
