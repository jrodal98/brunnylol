FROM rust:latest
WORKDIR /app
COPY . /app
RUN cargo build --release
ENV BRUNNYLOL_PORT=8000
ENV BRUNNYLOL_DB=/data/brunnylol.db
EXPOSE 8000
CMD ["target/release/brunnylol"]
