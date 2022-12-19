FROM rustembedded/cross:x86_64-pc-windows-gnu

RUN apt update && \
    apt install -y curl && \
    curl -L https://github.com/mozilla/sccache/releases/download/v0.2.15/sccache-v0.2.15-x86_64-unknown-linux-musl.tar.gz | tar xzf -
RUN chmod 775 /sccache-v0.2.15-x86_64-unknown-linux-musl/sccache
ENV RUSTC_WRAPPER=/sccache-v0.2.15-x86_64-unknown-linux-musl/sccache
