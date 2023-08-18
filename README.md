# parse-git-url

[![Build Status]](https://github.com/EricCrosson/parse-git-url/actions/workflows/release.yml)

[build status]: https://github.com/EricCrosson/parse-git-url/actions/workflows/release.yml/badge.svg?event=push

Supports common protocols as specified by the [Pro Git book](https://git-scm.com/book/en/v2)

See: [4.1 Git on the Server - The Protocols](https://git-scm.com/book/en/v2/Git-on-the-Server-The-Protocols)

Supports parsing SSH/HTTPS repo urls for:

- Github
- Bitbucket
- Azure Devops

See [tests/parse.rs](tests/parse.rs) for expected output for a variety of inputs.

---

URLs that use the `ssh://` protocol (implicitly or explicitly) undergo a small normalization process in order to be parsed.

Internally uses `Url::parse()` from the [Url](https://crates.io/crates/url) crate after normalization.

## Examples

### Run example with debug output

```shell
$ RUST_LOG=parse_git_url cargo run --example multi
$ RUST_LOG=parse_git_url cargo run --example trim_auth
```

### Simple usage and output

```bash
$ cargo run --example readme
```

```rust
use parse_git_url::GitUrl;

fn main() {
    println!("SSH: {:#?}", GitUrl::parse("git@github.com:tjtelan/git-url-parse-rs.git"));
    println!("HTTPS: {:#?}", GitUrl::parse("https://github.com/tjtelan/git-url-parse-rs"));
}
```

### Example Output

```bash
SSH: Ok(
    GitUrl {
        host: Some(
            "github.com",
        ),
        name: "git-url-parse-rs",
        owner: Some(
            "tjtelan",
        ),
        organization: None,
        fullname: "tjtelan/git-url-parse-rs",
        scheme: Ssh,
        user: Some(
            "git",
        ),
        token: None,
        port: None,
        path: "tjtelan/git-url-parse-rs.git",
        git_suffix: true,
        scheme_prefix: false,
    },
)
HTTPS: Ok(
    GitUrl {
        host: Some(
            "github.com",
        ),
        name: "git-url-parse-rs",
        owner: Some(
            "tjtelan",
        ),
        organization: None,
        fullname: "tjtelan/git-url-parse-rs",
        scheme: Https,
        user: None,
        token: None,
        port: None,
        path: "/tjtelan/git-url-parse-rs",
        git_suffix: false,
        scheme_prefix: true,
    },
)
```

## Acknowledgments

This repository has been forked from [tjtelan/git-url-parse-rs].
All credit goes to the original author.

[tjtelan/git-url-parse-rs]: https://github.com/tjtelan/git-url-parse-rs
