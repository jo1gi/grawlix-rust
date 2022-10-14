# grawlix
CLI tool for downloading comic books.

grawlix supports downloading comics from:
- [Flipp](https://flipp.dk)
- [League of Legends](https://universe.leagueoflegends.com/en_US/comic/)
- [Manga Plus](https://mangaplus.shueisha.co.jp/)
- [Webtoons](https://www.webtoons.com)

## Installation
grawlix can currently only be installed by building from source.
```shell
git clone git@gitlab.com:jo1gi/grawlix.git
cd grawlix
cargo build --release
```
Building requires the [rust](https://www.rust-lang.org/) compiler.

## Usage

- [Automatic updates](#automatic-updates)
- [Download single issues or series](#download-single-issues-or-series)
- [Arguments and configuration options](#arguments-and-configuration-options)
- [File Output](#file-output)

### Automatic updates
Add series to be automatically updated:
```shell
grawlix add <url>
```
where `url` is a link to a series.

To update all series added:
```shell
grawlix update
```

All series managed by grawlix is stored in `.grawlix-update` in the current
directory. Another file can be used the with `--update-location` argument or the
`update_location` option in the config.

### Download single issues or series
```shell
grawlix download <url>
```
`url` can be a link to an issue or a series.

### Configuration file
grawlix uses a configuration file stored at
`$XDG_CONFIG_HOME/grawlix/grawlix.toml`. Available options can be seen in
[Argument and Configuration Options](#arguments-and-configuration-options).

### Arguments and Configuration Options
| Argument            | Configuration     | Description                                                                                                                                                         |
|---------------------|-------------------|---------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| -f/--file           |                   | Path to file containing links to comics                                                                                                                             |
| --log-level         |                   | Log level (either trace, debug, info, warning, or error)                                                                                                            |
| --output-template   | output_template   | Output location of comics (See [File output](#file-output))                                                                                                         |
| --output-format     | output_format     | Format of output comic book (Either cbz or dir)                                                                                                                     |
| --overwrite         | overwrite         | Overwrite already existing files                                                                                                                                    |
| --info              | info              | Print additional information about comics to stdout                                                                                                                 |
| --json              | json              | Print information as json                                                                                                                                           |
| --update-location   | update_location   | Path to update file (See [Automatic updates](#automatic-updates))                                                                                                   |

### File Output
By default grawlix saves all comics as `{series}/{title}.cbz` relative to the
current path. This can be changed with the `--output-template` argument or the
`output_template` configuration option.

Available fields are:
- `title` Comic title
- `series` Comic series
- `publisher` Comic publisher
- `issuenumber` Issue number in series
- `year` Release year
- `month` Release month
- `day` Release day
- `pages` Number of pages

Not all fields are available for all comics.

## Contributing
Issues, bug reports, pull requests or ideas for features and improvements are
**very welcome**.

## Donations
If you like the project please consider donating.
- [Kofi](https://ko-fi.com/jo1gi)
- [Buy me a Coffee](https://www.buymeacoffee.com/joakimholm)
- BTC: bc1qrh8hcnw0fd22y7rmljlmrztwrz2nd5tqckrt44
- ETH: 0x8f5d2eb6d2a4d4615d2b9b1cfa28b4c5b9d18f9f
- LTC: ltc1qfz2936a04m2h7t0srxftygjrq759auav7ndfd3
- XMR: 8AX32z3DMYbXsjLY1nvhBj5QUJdgxJsBU3nnQXSfr9zPQxsitxrBbzLjofz7tDX6KGascC2fcbzDGB4uTPpHG1fjTtnMNie
