# adapted from polkadot Dockerfile
# this Dockerfile build the node within docker and copy the compiled binary into another minimal image
FROM docker.io/paritytech/ci-linux:production as builder

WORKDIR /laguna-chain
COPY . /laguna-chain

RUN cargo build --locked --release

# This is the 2nd stage: a very small image where we copy the Polkadot binary."
FROM docker.io/library/ubuntu:20.04

COPY --from=builder /laguna-chain/target/release/laguna-node /usr/local/bin

RUN useradd -m -u 1000 -U -s /bin/sh -d /laguna-chain laguna-chain && \
	mkdir -p /data /laguna-chain/.local/share && \
	chown -R laguna-chain:laguna-chain /data && \
	ln -s /data /laguna-chain/.local/share/laguna-chain && \
# unclutter and minimize the attack surface
	rm -rf /usr/bin /usr/sbin && \
# check if executable works in this container
	/usr/local/bin/laguna-node --version

USER laguna-chain

EXPOSE 30333 9933 9944 9615
VOLUME ["/data"]

ENTRYPOINT ["/usr/local/bin/laguna-node"]