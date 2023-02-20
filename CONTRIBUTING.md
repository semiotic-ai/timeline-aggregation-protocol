<!-- omit in toc -->
# Contributing to TAP

First off, thanks for taking the time to contribute! ❤️

All types of contributions are encouraged and valued. See the [Table of Contents](#table-of-contents) for different ways to help and details about how this project handles them. Please make sure to read the relevant section before making your contribution. It will make it a lot easier for us maintainers and smooth out the experience for all involved. The community looks forward to your contributions. 🎉

> And if you like the project, but just don't have time to contribute, that's fine. There are other easy ways to support the project and show your appreciation, which we would also be very happy about:
>
> - Star the project
> - Tweet about it
> - Refer this project in your project's readme
> - Mention the project at local meetups and tell your friends/colleagues

<!-- omit in toc -->
## Table of Contents

- [I Have a Question](#i-have-a-question)
- [I Want To Contribute](#i-want-to-contribute)
  - [Reporting Bugs](#reporting-bugs)
  - [Suggesting Enhancements](#suggesting-enhancements)
  - [Contributing PRs](#contributing-prs)
  - [Reviewing, Approving and Merging PRs](#reviewing-approving-and-merging-prs)
- [Styleguides](#styleguides)
  - [Code style](#code-style)
  - [Linting](#linting)
  - [Package and dependencies](#package-and-dependencies)
  - [Testing](#testing)
  - [Commit Messages](#commit-messages)

## I Have a Question

> If you want to ask a question, we assume that you have read the available [Documentation]().

Before you ask a question, it is best to search for existing [Issues](https://github.com/semiotic-ai/timeline_aggregation_protocol/issues) that might help you. In case you have found a suitable issue and still need clarification, you can write your question in this issue. It is also advisable to search the internet for answers first.

If you then still feel the need to ask a question and need clarification, we recommend the following:

- Open an [Issue](https://github.com/semiotic-ai/timeline_aggregation_protocol/issues/new).
- Provide as much context as you can about what you're running into.
- Provide project and platform versions (nodejs, npm, etc), depending on what seems relevant.

We will then take care of the issue as soon as possible.

<!--
You might want to create a separate issue tag for questions and include it in this description. People should then tag their issues accordingly.

Depending on how large the project is, you may want to outsource the questioning, e.g. to Stack Overflow or Gitter. You may add additional contact and information possibilities:
- IRC
- Slack
- Gitter
- Stack Overflow tag
- Blog
- FAQ
- Roadmap
- E-Mail List
- Forum
-->

## I Want To Contribute

> ### Legal Notice <!-- omit in toc -->
>
> When contributing to this project, you must agree to the [Developer Certificate of Origin](https://developercertificate.org/) (DCO):
>
> ```
> Developer Certificate of Origin
> Version 1.1
> 
> Copyright (C) 2004, 2006 The Linux Foundation and its contributors.
> 
> Everyone is permitted to copy and distribute verbatim copies of this
> license document, but changing it is not allowed.
> 
> 
> Developer's Certificate of Origin 1.1
> 
> By making a contribution to this project, I certify that:
> 
> (a) The contribution was created in whole or in part by me and I
>     have the right to submit it under the open source license
>     indicated in the file; or
> 
> (b) The contribution is based upon previous work that, to the best
>     of my knowledge, is covered under an appropriate open source
>     license and I have the right under that license to submit that
>     work with modifications, whether created in whole or in part
>     by me, under the same open source license (unless I am
>     permitted to submit under a different license), as indicated
>     in the file; or
> 
> (c) The contribution was provided directly to me by some other
>     person who certified (a), (b) or (c) and I have not modified
>     it.
> 
> (d) I understand and agree that this project and the contribution
>     are public and that a record of the contribution (including all
>     personal information I submit with it, including my sign-off) is
>     maintained indefinitely and may be redistributed consistent with
>     this project or the open source license(s) involved.
> ```
>
> We require a [sign-off](https://git-scm.com/docs/git-commit#Documentation/git-commit.txt---signoff) message in every commit in your pull requests to signal your agreement with the DCO.
>
> Note: contributing code "generated" by artificial intelligence tools, such as [Github Copilot](https://github.com/features/copilot)
> would be a **violation of the DCO**, as it is known to plagiarize snippets of code, without the possibility of assessing license or copyright compatibility with the current project, nor complying with original license attribution clauses.

### Reporting Bugs

<!-- omit in toc -->
#### Before Submitting a Bug Report

A good bug report shouldn't leave others needing to chase you up for more information. Therefore, we ask you to investigate carefully, collect information and describe the issue in detail in your report. Please complete the following steps in advance to help us fix any potential bug as fast as possible.

- Make sure that you are using the latest version.
- Determine if your bug is really a bug and not an error on your side e.g. using incompatible environment components/versions (Make sure that you have read the [documentation](). If you are looking for support, you might want to check [this section](#i-have-a-question)).
- To see if other users have experienced (and potentially already solved) the same issue you are having, check if there is not already a bug report existing for your bug or error in the [bug tracker](https://github.com/semiotic-ai/timeline_aggregation_protocol/issues?q=label%3Abug).
- Also make sure to search the internet (including Stack Overflow) to see if users outside of the GitHub community have discussed the issue.
- Collect information about the bug:
  - Stack trace (Traceback)
  - OS, Platform and Version (Windows, Linux, macOS, x86, ARM)
  - Version of the interpreter, compiler, SDK, runtime environment, package manager, depending on what seems relevant.
  - Possibly your input and the output
  - Can you reliably reproduce the issue? And can you also reproduce it with older versions?

<!-- omit in toc -->
#### How Do I Submit a Good Bug Report?

> You must never report security related issues, vulnerabilities or bugs including sensitive information to the issue tracker, or elsewhere in public. Instead sensitive bugs must be submitted through this form: https://forms.gle/iWhjJPiBGcLDqw2T8.
<!-- You may add a PGP key to allow the messages to be sent encrypted as well. -->

We use GitHub issues to track bugs and errors. If you run into an issue with the project:

- Open an [Issue](https://github.com/semiotic-ai/timeline_aggregation_protocol/issues/new). (Since we can't be sure at this point whether it is a bug or not, we ask you not to talk about a bug yet and not to label the issue.)
- Explain the behavior you would expect and the actual behavior.
- Please provide as much context as possible and describe the *reproduction steps* that someone else can follow to recreate the issue on their own. This usually includes your code. For good bug reports you should isolate the problem and create a reduced test case.
- Provide the information you collected in the previous section.

Once it's filed:

- The project team will label the issue accordingly.
- A team member will try to reproduce the issue with your provided steps. If there are no reproduction steps or no obvious way to reproduce the issue, the team will ask you for those steps and mark the issue as `needs-repro`. Bugs with the `needs-repro` tag will not be addressed until they are reproduced.
- If the team is able to reproduce the issue, it will be marked `needs-fix`, as well as possibly other tags (such as `critical`), and the issue will be left to be [implemented by someone](#contributing-prs).

<!-- You might want to create an issue template for bugs and errors that can be used as a guide and that defines the structure of the information to be included. If you do so, reference it here in the description. -->

### Suggesting Enhancements

This section guides you through submitting an enhancement suggestion for H2S2, **including completely new features and minor improvements to existing functionality**. Following these guidelines will help maintainers and the community to understand your suggestion and find related suggestions.

<!-- omit in toc -->
#### Before Submitting an Enhancement

- Make sure that you are using the latest version.
- Read the [documentation]() carefully and find out if the functionality is already covered, maybe by an individual configuration.
- Perform a [search](https://github.com/semiotic-ai/timeline_aggregation_protocol/issues) to see if the enhancement has already been suggested. If it has, add a comment to the existing issue instead of opening a new one.
- Find out whether your idea fits with the scope and aims of the project. It's up to you to make a strong case to convince the project's developers of the merits of this feature. Keep in mind that we want features that will be useful to the majority of our users and not just a small subset. If you're just targeting a minority of users, consider writing an add-on/plugin library.

<!-- omit in toc -->
#### How Do I Submit a Good Enhancement Suggestion?

Enhancement suggestions are tracked as [GitHub issues](https://github.com/semiotic-ai/timeline_aggregation_protocol/issues).

- Use a **clear and descriptive title** for the issue to identify the suggestion.
- Provide a **step-by-step description of the suggested enhancement** in as many details as possible.
- **Describe the current behavior** and **explain which behavior you expected to see instead** and why. At this point you can also tell which alternatives do not work for you.
- You may want to **include screenshots and animated GIFs** which help you demonstrate the steps or point out the part which the suggestion is related to. You can use [this tool](https://www.cockos.com/licecap/) to record GIFs on macOS and Windows, and [this tool](https://github.com/colinkeenan/silentcast) or [this tool](https://github.com/GNOME/byzanz) on Linux. <!-- this should only be included if the project has a GUI -->
- **Explain why this enhancement would be useful** to most H2S2 users. You may also want to point out the other projects that solved it better and which could serve as inspiration.

<!-- You might want to create an issue template for enhancement suggestions that can be used as a guide and that defines the structure of the information to be included. If you do so, reference it here in the description. -->

### Contributing PRs

- PRs should match the existing code style present in the file.
- PRs affecting the public API, including adding new features, must update the public documentation.
- Comments and (possibly internal) docstrings should make the code accessible.
- You should usually open an issue about a bug or possible improvement before opening a PR with a solution.
- PRs should do a single thing, so that they are easier to review.
  - For example, fix one bug, or update compatibility, rather than fixing a bunch of bugs and updating compatibility and adding a new feature.
- PRs should add tests which cover the new or fixed functionality.
- PRs that move code should not also change code, so that they are easier to review.
  - If only moving code, review for correctness is not required.
  - If only changing code, then the diff makes it clear what lines have changed.
- PRs with large improvements to style should not also change functionality.
  - This is to avoid making large diffs that are not the focus of the PR.
  - While it is often helpful to fix a few typos in comments on the way past, it is different to using a regex or formatter on the whole project to fix spacing around operators.
- PRs introducing breaking changes should make this clear when opening the PR.
- You should not push commits which commented-out tests.
  - If pushing a commit for which a test is broken, use the `@test_broken` macro.
  - Commenting out tests while developing locally is okay, but committing a commented-out test increases the risk of it silently not being run when it should be.
- You should not squash down commits while review is still on-going.
  - Squashing commits prevents the reviewer being able to see what commits are added since the last review.
- You should help **review** your PRs, even though you cannot **approve** your own PRs.
  - For instance, start the review process by commenting on why certain bits of the code changed, or highlighting places where you would particularly like reviewer feedback.

### Reviewing, Approving and Merging PRs

- PRs must have 1 approval before they are merged.
- PR authors should not approve their own PRs.
- PRs should pass CI tests before being merged.
- PRs by people without merge rights must have approval from someone who has merge rights (who will usually then merge the PR).
- PRs by people with merge rights must have approval from someone else, who may or may not have merge rights (and then may merge their own PR).
- PRs by people with merge rights should not be merged by people other than the author (just approved).
- Review comments should be phrased as questions, as it shows you are open to new ideas.
  - For instance, “Why did you change this to X? Doesn’t that prevent Y?” rather than “You should not have changed this, it will prevent Y”.
  Small review suggestions, such as typo fixes, should make use of the `suggested change` feature.
  - This makes it easier and more likely for all the smaller changes to be made.
- Reviewers should continue acting as a reviewer until the PR is merged.

## Styleguides

### Code style

We use `rustfmt` for code styling. Before committing, make sure that:

```sh
cargo fmt --all -- --check
```

exits without errors (code 0).

### Linting

We use `cargo clippy` with `--all-features` for linting. Before committing, run:

```sh
cargo clippy --all-features
```

and fix as many issues as possible, and make sure that it exits without errors (code 0).

### Package and dependencies

We use `cargo check` to check the workspace's packages and their dependencies.
Before committing, make sure that:

```sh
cargo check
```

exits without errors (code 0)

### Testing

Use `cargo test`. It is strongly encouraged to write tests for all new code. New tests for existing code are equally welcome.
Before submitting a PR, make sure that the tests pass:

```sh
cargo test
```

### Commit Messages

Use [Conventional Commits v1.0.0](https://www.conventionalcommits.org/en/v1.0.0/). The commit types will be used to automate [SemVer](https://semver.org/).

[Sign-off](https://git-scm.com/docs/git-commit#Documentation/git-commit.txt---signoff) on all commits if you accept the [Developer Certificate of Origin](https://developercertificate.org/). Pull requests containing commits without the sign-off will be rejected.

## Attribution

This guide is based on the **contributing-gen**. [Make your own](https://github.com/bttger/contributing-gen)!

With additions from [SciML/ColPrac](https://github.com/SciML/ColPrac).
