FROM rust:1.83.0
WORKDIR /app
COPY . /app
RUN cargo build --release
ENV ROCKET_ENV=prod
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=80
ENV ROCKET_LOG=critical
EXPOSE 80
CMD ["target/release/brunnylol"]
