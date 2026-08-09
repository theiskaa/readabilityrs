# Contributing Guidelines
First of all, thank you for considering contributing!
Here you will find some guidelines on how to contribute. Feel free to propose changes to this document if you think something is missing or needs to be clarified.

## Contributing Guides
First time contributing? You can learn more about contributing to open source projects through GitHub's [wonderful open source guides](https://opensource.guide/how-to-contribute/).

### Reporting a Problem

Before you open a new issue, check to see if the problem has [already been reported](https://github.com/theiskaa/readabilityrs/issues).
If it has and **the issue is still open**, add a comment to the existing issue instead of creating a new one.
**Note:** _If you find a **closed** issue that seems to address the same thing that you've found, open a new issue and include a link to the original one._

When you open an issue, try to be as descriptive as possible. Add any relevant screenshots so that the problem can be identified quickly.

### Opening a Pull Request

> A lot of the following information was inspired by [opensource.guide](https://opensource.guide/how-to-contribute/), a great site to learn all about open source software.

If you've fixed a bug, started working on an enhancement, or done something else, you can [create a pull request](https://github.com/theiskaa/readabilityrs/pulls) to start a conversation about your changes.
A pull request doesn't necessarily have to include finished work. It may be better to open a PR early on so that you can get feedback on your contribution. Just mention that it's a work in progress and keep adding commits.

Here's how to contribute and submit your pull request:
1. [**Fork the repository**](https://help.github.com/articles/fork-a-repo/) and clone it locally. Add the original repository as a remote and pull in changes every so often so that you stay up to date with the project.
2. [**Create a branch**](https://guides.github.com/introduction/flow/) from `main` for your changes.
3. **Add, commit, and push** your changes to your branch.
4. **Test out your changes.** Make sure that they work as you intended.
5. [**Open a pull request**](https://github.com/theiskaa/readabilityrs/pulls) to merge your branch into `main`.
   - Reference any issues related to your PR (e.g. "Resolves #42").
   - Describe your changes in detail and include screenshots if necessary.
6. Sit back, relax, and wait for your PR to be reviewed. You might have to tweak your contribution or elaborate on your changes. That's OK, don't be afraid to justify your reasoning and ask questions.

## Development

This is a Rust library (edition 2021, MSRV 1.83). Everything runs through Cargo.

| Task | Command |
|------|---------|
| Build | `cargo build` |
| Run all tests | `cargo test` |
| Check formatting | `cargo fmt --check` |
| Apply formatting | `cargo fmt` |
| Lint | `cargo clippy --all-targets -- -D warnings` |
| Benchmarks | `cargo bench` |

`cargo test` covers the unit tests, the Markdown integration tests, the doctests, and the Mozilla compatibility suite. The Mozilla suite (`cargo test --test mozilla_test_suite`) replays the corpus under `tests/test-pages/` and asserts extraction against each page's `expected.html` and `expected-metadata.json`.

**Never hand-edit fixtures under `tests/test-pages/` to make a test pass.** They are the reference corpus. If your change moves the corpus, that is a real behavior change to explain in the PR — not a fixture to patch.

Before opening a PR, confirm all three gates are green locally (they are what CI enforces):

```
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

`benches/` also holds a Node.js harness that compares against Mozilla's original `@mozilla/readability` (`npm install && node benchmark.js` inside `benches/`).

### Branches and commits

- Branch off `main` with a typed prefix: `feature/…`, `bugfix/…`, `refactor/…`, `chore/…`, `docs/…`.
- Commit messages are single-line [conventional commits](https://www.conventionalcommits.org): `type(scope): what changed` — for example `feat(readerable): implement full isProbablyReaderable checks`. Types in use: `feat`, `fix`, `refactor`, `chore`, `docs`, `test`, `perf`, `ci`. No body.

### Source layout

- `src/readability.rs` — top-level orchestration (`Readability::new` → `parse()` → `Article`).
- `src/content_extractor.rs` — scoring and candidate selection.
- `src/cleaner.rs`, `src/post_processor.rs` — pre- and post-processing cleanup.
- `src/metadata/` — title, byline, language, image, and JSON-LD / meta-tag extraction.
- `src/elements/`, `src/markdown/` — the optional HTML→Markdown output path.
- `src/constants.rs` — shared regexes and scoring flags.

Thank you for reading through this contributing guide and welcome to the project!
