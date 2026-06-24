# dockslim

A CLI tool for analyzing and slimming down Docker images.

## Usage

```sh
dockslim analyze <image>
```

This saves the image to `image.tar` via `docker save` as a first step toward analysis.

## Status

Early days — just image export is wired up so far.

## Build

```sh
cargo build --release
```
