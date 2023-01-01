FROM docker.io/library/rust:bullseye as builder

RUN apt-get update && \
    apt-get install -y \
      cmake

COPY . .
RUN cargo install \
  --path . \
  --root /usr/local

FROM docker.io/library/debian:bullseye-slim

RUN apt-get update && \
    apt-get install -y \
      tini

COPY --from=builder \
  /usr/local/bin/makerspace-spaceapi \
  /usr/local/bin/makerspace-spaceapi

ENV API_ADDRESS "0.0.0.0:8080"
ENV OBSERVABILITY_ADDRESS "0.0.0.0:9090"

EXPOSE 8080
EXPOSE 9090

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/makerspace-spaceapi"]
