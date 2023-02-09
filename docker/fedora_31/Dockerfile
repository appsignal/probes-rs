FROM fedora:31
RUN yum -y update; yum clean all
RUN yum -y groupinstall 'Development Tools'

RUN curl https://sh.rustup.rs > sh.rustup.rs
RUN sh sh.rustup.rs --default-toolchain none -y \
    && echo 'source $HOME/.cargo/env' >> $HOME/.bashrc

WORKDIR /probes

COPY rust-toolchain .
RUN . $HOME/.cargo/env && rustup toolchain install "$(cat rust-toolchain)"
RUN rm rust-toolchain
