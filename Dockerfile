FROM ubuntu:latest AS src

RUN apt-get update

# Get Ubuntu packages
RUN apt-get install -y \
    build-essential \
    curl

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly -y

ENV PATH="/root/.cargo/bin:${PATH}"

COPY . .

RUN rustup override set nightly
RUN cargo install --path .

FROM ubuntu:latest

RUN apt-get update

# Get Ubuntu packages
RUN apt-get install -y \
    build-essential \
    curl

RUN curl https://sh.rustup.rs -sSf | sh -s -- --default-toolchain nightly -y

ENV PATH="/root/.cargo/bin:${PATH}"

RUN echo 'source $HOME/.cargo/env' >> $HOME/.bashrc
RUN rustup override set nightly

COPY --from=src /root/.cargo/bin/cargo-pgo /usr/local/bin/cargo-pgo

RUN apt update \
    && apt install -y wget gnupg \
    && wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add - \
    && echo "deb http://apt.llvm.org/bookworm/ llvm-toolchain-bookworm-18 main" > /etc/apt/sources.list.d/llvm-toolchain.list \
    && echo "deb-src http://apt.llvm.org/bookworm/ llvm-toolchain-bookworm-18 main" >> /etc/apt/sources.list.d/llvm-toolchain.list \
    && apt-get update \
    && apt install -y \
    bolt-18 \
    musl-tools \
    && ln -s /usr/bin/merge-fdata-18 /usr/bin/merge-fdata \
    && ln -s /usr/bin/llvm-bolt-18 /usr/bin/llvm-bolt \
    && ln -s /usr/lib/llvm-18/lib/libbolt_rt_instr.a /usr/lib/libbolt_rt_instr.a \
    && apt autoremove -y wget gnupg \
    && rm -rf /var/lib/apt/lists/* /etc/apt/sources.list.d/llvm-toolchain.list

RUN rustup component add llvm-tools-preview \
    && rustup target add x86_64-unknown-linux-musl

WORKDIR /workdir
