# Contributing

Contributions are highly appreciated. Please follow the rules and guidelines listed below to make the review proces easier and get your changes merged faster!

## Reporting issues

Open an issue using one of the templates:

- [Bug report](https://github.com/Ggjorven/FlareSolverr-Aggregate/issues/new?template=BUG-REPORT.yml)
- [Feature request](https://github.com/Ggjorven/FlareSolverr-Aggregate/issues/new?template=FEATURE-REQUEST.yml)
- [Other](https://github.com/Ggjorven/FlareSolverr-Aggregate/issues/new?template=BLANK_ISSUE)

GitHub presents these automatically when you click **New issue**.

## Opening a pull request

Fill in the pull request template that matches your target branch. GitHub does not show it
automatically when multiple templates exist, so manually copy its markdown contents: 

- Targeting `dev`: [`dev.md`](./.github/PULL_REQUEST_TEMPLATE/dev.md)
- Targeting `main`: [`main.md`](./.github/PULL_REQUEST_TEMPLATE/main.md)

Complete the checklist in the template before requesting a merge.

## Backend (Rust, Docker)

To contribute to the backend follow these steps:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/my-change`)
3. Make your changes and ensure everything compiles (`cargo build` && `docker build .`)
4. Run tests (`cargo test`)
5. Run the linter (`cargo clippy`)
6. Format your code (`cargo +nightly fmt` from [`rustfmt`](https://github.com/rust-lang/rustfmt))
7. Open a pull request with a clear description of what you changed and why
