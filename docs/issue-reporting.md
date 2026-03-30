# Writing Good Issues

Good issues make the project easier to review, maintain, and improve.

This guide helps contributors open issues that are clear, actionable, and aligned with the current state of the repository.

## Start With the Right Kind of Issue

Before opening anything, decide what you are actually reporting:

- use a **bug report** when something implemented on `main` is broken, regressed, or behaving incorrectly
- use a **feature request** when you want a new capability, workflow improvement, or missing language/tooling feature
- use a **documentation issue** when something is outdated, missing, misleading, or hard to understand
- use a **design discussion** when the topic is still open-ended and needs alignment before implementation

The repository already ships templates for each of these under [`.github/ISSUE_TEMPLATE/`](../.github/ISSUE_TEMPLATE/).

## Check Existing Context First

Before opening a new issue, quickly check:

- open issues
- recent pull requests
- relevant documentation, especially [README.md](../README.md), [CONTRIBUTING.md](../CONTRIBUTING.md), and [middleend/README.md](../middleend/README.md)

This avoids duplicates and also helps you describe the problem using the same terminology the project already uses.

## Use a Clear Title

The title should say what is wrong or what is being proposed without forcing the reader to guess.

Good examples:

- `[BUG] IR generation hides the original variable name`
- `[FEATURE] Add graph generation validation pass`
- `[DOCS] Clarify current status of the middleend README`
- `[Discussion] Slot defaults and named child regions`

Weak examples:

- `It broke`
- `Help`
- `Question`
- `Something weird`

## Keep One Issue Per Problem

Try not to mix multiple unrelated problems into a single issue.

Good:

- one bug per issue
- one feature request per issue
- one design topic per discussion

Bad:

- bug report + feature request in the same issue
- parser, checker, and docs problems all mixed together without separation

If there really are multiple related problems, split them and cross-link them.

## What Makes a Bug Report Useful

A good bug report should include:

- what happened
- how to reproduce it
- what you expected instead
- the smallest relevant `.slynx` snippet, command, or log excerpt
- environment details such as OS, Rust version, and relevant commit or tag

For compiler or language bugs, small repros help a lot. Prefer this:

```slynx
func main(): int {
    let x = 1;
}
```

over pasting a large project when a tiny snippet already shows the problem.

## What Makes a Feature or Design Issue Useful

For feature requests and design discussions, the most helpful issues explain:

- the actual problem or limitation
- why it matters
- whether the topic is about syntax, typing, IR, tooling, docs, or workflow
- any constraints, tradeoffs, or alternatives already considered

When the topic is still exploratory, prefer a discussion-style issue instead of presenting the first idea as if it were already decided.

## Stay Grounded in the Current Repository

Try to distinguish clearly between:

- behavior implemented on `main`
- behavior only described in specs or design docs
- future direction that is not implemented yet

This matters in Slynx because some documents intentionally describe design direction beyond what the current branch emits today.

## Good Issue Checklist

Before submitting, check that your issue:

- uses the correct template
- has a direct title
- describes one problem or one direction
- includes reproduction or examples when applicable
- stays aligned with the current repo instead of guessing future behavior
- gives enough context that someone else can continue from it later

## Related Documents

- [CONTRIBUTING.md](../CONTRIBUTING.md)
- [README.md](../README.md)
- [middleend/README.md](../middleend/README.md)
