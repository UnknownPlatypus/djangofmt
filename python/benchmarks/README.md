# Benchmarks

Documentation about how to run various performance benchmark for `djangofmt` versus similar tools:

- `djlint`: Same scope as `djangofmt` - fully featured template formatter
- `djhtml`: Only an indenter, it will never add/remove newlines
- `djade`: Only format django template syntax - HTML is not formatted
- `prettier`: Does not support Django and only format HTML

## Setup

First `cd ./python/benchmarks` and then run this to build `djangofmt` & install other tools needed to benchmark:

```bash
cargo build --release &&
uv sync --project . -p 3.11 &&
npm i
```

Then activate the python env

```shell
. .venv/bin/activate
```

## Running Benchmarks

Simply run this command, providing a directory containing django templates.

```bash
./run_formatter.sh ~/templates
```

A setup step will discover every html files inside and the run the various tools on it.
This will cause destructive operations, be sure to target a safe directory (tracked with git or temporary)
You can change the print width with the `LINE_LENGTH` env variable (default: 120)

## Color Palette for charts

Built using [vega playground](https://vega.github.io/editor/#/url/vega-lite/N4IgJAzgxgFgpgWwIYgFwhgF0wBwqgegIDc4BzJAOjIEtMYBXAI0poHsDp5kTykBaADZ04JAKyUAVhDYA7EABoQAEySYUqUMSSCGcCGgDaoTGzaC0IACKSkssmwBmCTIpCYaCOGgAMlHwCMAJxKHl4AYmwATsiu6H6BQQYAvgomZhboNkjK3qGe3qgJAOwATPkR0bGWJaUpae4ZljZYCBYVhQGUACw+ABwdkTFqll299enmzZLCsq4daKWUAMxixYNVI+hLqxONU+gAClFw2DRwUW5hhcuUAQPuBUPV6Lf3KQC6qSBQco40ZDQoBwSCiSAQBlQxhAsnBhRUcEcSAYgkwkTmbm0unh-CQOBwgjg-AgAE8IJhEAoAEKzADWAFkkFAAMpkikIdGYBQAHRAzPIbDgAAIAKoASV5CgAEnBBKQPFAkAoAIJRGg6BQQOwQYkXGiOHkgZX4wlCgDC5miQoAogg2JIaJLefyHMLxTa7Q7eSBUqBYV5LIIkExZRbBNFMTo9JYAMRQILKALKII+hr++FMUFhiNKLHR9Ax+7FRxiPo+j5KJAADxokNAQZDgmzl00IDgVZwLZADdDlsu3x7gk5QLbHa7uSRKLRclcyW+xHOAHcR+SomxaYVZCjBN9M1BaWQ1wxZMpLJgwbIICCThjd0yD0eT2eL1fQXAMUp379lDR7COSSO-yyqe6CmFMoQkjg8KyGwCC-jobjVrWI6HjQIFIoIEBwPkmCEmgW6CIISiDpyzI0AAXp05SPPukIYVhxHBrKhw5D+f6oAEPhKMosFIL+aD0dh3ZMUOM4AOpwACWAjr8J50Ow8ithS5KWKomAMAglB5sKAC8elCgA5Nk9hOC4BmRtilhMOYp7zlG0GbBY3weLhcDNiO7adoGInuXOSgyFEcQETuShVoB5yCCBjwBhBUGWAAjgwdguWoNCkIhNZ1o8rn4dujGNtaY6qWoGlafZQoANSGRA5n5PuFrHnEyx1bSZGUb4+XMaxv6AqgABsnWCMqwhkIpPzvhSlyDaRFFUS17noFEZCZgAFAEpTrOtm0bQofilGIACUbiocoC0gEtq1bQoV1XXth1uDxyD8agglzt88BSXE61cSAi5ofQliyeov4XG4QYkqDUKgMgUS0lZoJuF+bBsb1oAQLNI7aWgATdN8vzhi2Wj2R5RXoJmUS+W9DTntqjhVEYoD-KikMqCVmlgYIQoAIT6UZtgmc4mDmckFYgDDcNKZB8IUlW8wgDoAJjYSjhy5mWGzPCcHKMoeHcWF-VKHTcxtVR3xIyjI4y3EjMRVF1zPCMeN9pjxOtp5XaDpTfkmC+dMxAzIBM5NxXqezGRCnpOmGcZDiC8Lovi5bUtnu2csK6NgaIqrSDqyDlhazreQqPrDxG5gJvY9RZcSZ9Vk2ambayN+PWW6n4XAWeTybLOSj4xGrZY27pPCY2Xsi8kQA).

Color palette:

- bars -> `#187f58`
- labels (light mode) -> `#333333`
- labels (dark mode) -> `#c9d1d9`
