# STAGE1: Build the binary
FROM rust:bullseye as builder

# Install build dependencies
RUN apt-get update && \
    apt-get install -y libasound2-dev libudev-dev

# Create a new empty shell project
WORKDIR /quantsum

# Copy over the Cargo.toml files to the shell project
COPY Cargo.toml Cargo.lock ./

# Build and cache the dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs
RUN cargo fetch
RUN cargo build --release
RUN rm src/main.rs

# Copy the actual code files and build the application
COPY src ./src/
COPY assets ./assets/
# Update the file date
RUN touch src/main.rs
RUN cargo build --release

# STAGE2: create a slim image with the compiled binary
FROM debian:bullseye as runner

EXPOSE 7777/udp

# Copy the binary from the builder stage
WORKDIR /quantsum
COPY --from=builder /quantsum/target/release/quantsum quantsum
COPY --from=builder /quantsum/assets assets
RUN apt-get update && \
    apt-get install -y libasound2-dev libudev-dev
CMD [ "./quantsum", "dedicated-server" ]
