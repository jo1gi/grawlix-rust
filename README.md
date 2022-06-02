# grawlix
CLI tool for downloading comic books.

grawlix supports downloading comics from:
- [Flipp](https://flipp.dk)
- [League of Legends](https://universe.leagueoflegends.com/en_US/comic/)
- [Webtoons](https://www.webtoons.com)

## Installation
grawlix can currently only be installed by building from source.
```shell
git clone git@gitlab.com:jo1gi/grawlix.git
cd grawlix
cargo build
```
Building requires the [rust](https://www.rust-lang.org/) compiler.

## Usage
```shell
grawlix <url>
```
`url`can be a link to a specific comic or a series.
