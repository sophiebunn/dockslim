# dockslim

A CLI tool for analyzing and slimming down Docker images.

## Usage

Requires Docker running locally.

```sh
cargo run -- analyze <image>
```

Example:

```sh
cargo run -- analyze python:3.11
```

To try a test image build
[`examples/bloated.Dockerfile`](examples/bloated.Dockerfile):

```sh
docker build -f examples/bloated.Dockerfile -t dockslim-test .
cargo run -- analyze dockslim-test
```