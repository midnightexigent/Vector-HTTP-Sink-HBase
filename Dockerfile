####################################################################################################
## Builder
####################################################################################################
FROM rust:latest AS builder

RUN rustup target add x86_64-unknown-linux-musl
RUN apt-get update && apt-get install -y musl-tools musl-dev
RUN update-ca-certificates

# Create appuser
ENV USER=vector-http-sink-hbase
ENV UID=10001

RUN adduser \
    --disabled-password \
    --gecos "" \
    --home "/nonexistent" \
    --shell "/sbin/nologin" \
    --no-create-home \
    --uid "${UID}" \
    "${USER}"


WORKDIR /vector-http-sink-hbase

COPY ./ .

RUN cargo build --target x86_64-unknown-linux-musl --release

####################################################################################################
## Final image
####################################################################################################
FROM scratch

# Import from builder.
COPY --from=builder /etc/passwd /etc/passwd
COPY --from=builder /etc/group /etc/group

WORKDIR /vector-http-sink-hbase

# Copy our build
COPY --from=builder /vector-http-sink-hbase/target/x86_64-unknown-linux-musl/release/vector-http-sink-hbase ./

# Use an unprivileged user.
USER vector-http-sink-hbase:vector-http-sink-hbase

ENTRYPOINT ["/vector-http-sink-hbase/vector-http-sink-hbase"]
