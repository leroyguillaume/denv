# Contributing

## Code

Pull requests are welcome. We're happy to help contributors.

To make changes to the code, fork it first and clone it:
```bash
git clone git@github.com:your-username/denv.git
```

Now, install [pre-commit](https://pre-commit.com/) hooks:
```bash
npm install
pre-commit install --hook-type pre-commit
pre-commit install --hook-type commit-msg
```

The hooks check:
  - if your code is well formatted (`cargo fmt`)
  - if your code compiles (`cargo check`)
  - if your code is clean (`cargo clippy`)
  - if your commit respects [Conventional Commits](https://www.conventionalcommits.org/en/v1.0.0/) specification

Tu run tests, run:
```bash
cargo test
```
