# Laguna Chain

WIP

## Environment Setup

### Install Rust and the Rust Toolchain

1. Install rustup by running the following command:

```bash
curl https://sh.rustup.rs -sSf | sh
```

2. Configure your current shell to reload your PATH environment variable so that it includes the Cargo bin directory by running the following command:

```bash
source ~/.cargo/env
```

3. Configure the Rust toolchain to default to the latest stable version by running the following commands:

```bash
rustup default stable
rustup update
```

4. Add the nightly release and the nightly WebAssembly (wasm) targets by running the following commands:

```bash
rustup update nightly
rustup target add wasm32-unknown-unknown --toolchain nightly
```

5. Verify your installation by running the following commands:

```bash
rustc --version
rustup show
```

### Clone Repo, Build, and Start the Chain

1. Clone the `laguna-chain` repo by running the following command:

```bash
git clone https://github.com/Laguna-Chain/laguna-chain.git
```

2. Change to the root of the `laguna-chain` directory by running the following command:

```bash
cd laguna-chain
```

3. Compile the chain by running the following command:

```bash
cargo build --release
```

Building with the `--release` flag results in optimized artifacts.

4. To start the chain, run the following command:

```bash
./target/release/laguna-node --dev
```
