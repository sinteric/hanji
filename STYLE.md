# Rust style

hanji's Rust follows the [Google Rust Style Guide][g], and through it the public
[Rust Style Guide](https://doc.rust-lang.org/nightly/style-guide/) and
[Rust API Guidelines](https://rust-lang.github.io/api-guidelines/). **Every rule
in the guide applies unless the table below marks it excluded or changed, and
each of those rows carries the reason.** A rule the guide adds later applies
until someone records it here. Why one external base: a written base ends the
"which way is right" review argument before it starts, and the guide gives a
reason for each rule.

Why some rules do not fit: the guide serves google3, Chromium, Android and
Fuchsia, where Rust ships in operating systems and browsers built with GN or
Bazel, with `panic = abort`. hanji is a set of crates built with Cargo and
linked into other people's programs, so the rules that follow from that setting
are excluded.

[g]: https://google.github.io/styleguide/rustguide.html

## Library code and application code

The guide's API rules bind library code and are a strong recommendation for
application code ([§api-design-context][adc]). **Every hanji crate is library
code except `hanji-cli` and `hanji-testkit`**: the others are published, and
other programs build against them.

[adc]: https://google.github.io/styleguide/rustguide.html#api-design-context

## The rules

"Held by" names what catches a break: rustfmt (`python3 scripts/fmt.py
--check`, a CI step), a clippy lint (CI runs `cargo clippy --locked --workspace
--all-targets -- -D warnings`; the root `Cargo.toml`'s `[workspace.lints]` adds
the lints beyond clippy's default set), or review. ⚠ A rule held by review is
held only as well as the reviewer remembers it.

| Guide rule | Here | Held by | Why, where it is not "adopted" |
| --- | --- | --- | --- |
| [Follow public Rust style rules](https://google.github.io/styleguide/rustguide.html#public-rules) | adopted | rustfmt, clippy, review | |
| [Apply API design rules in context](https://google.github.io/styleguide/rustguide.html#api-design-context) | adopted | review | Library code is defined above. |
| [Prefer small crates](https://google.github.io/styleguide/rustguide.html#small-crates) [chrome] | **excluded** |  | It exists for Chromium's GN build graph and caching. hanji's crates follow its formats and layers (core, package, one crate per format, preview, CLI), which is what a reader of DESIGN.md expects to find. |
| [Flat file layout](https://google.github.io/styleguide/rustguide.html#structure) [chrome] | **excluded** | | The guide itself keeps Cargo's layout for code built with Cargo, which all of ours is. |
| [Name files after the primary item](https://google.github.io/styleguide/rustguide.html#userdefined-item) | adopted | review | Older files move only when they are already being restructured: a sweep would collide with every open branch. |
| [No APIs in non-leaf modules](https://google.github.io/styleguide/rustguide.html#non-leaf-modules) | adopted | review | Same as the row above. |
| [No glob imports](https://google.github.io/styleguide/rustguide.html#glob-imports) | adopted | clippy `wildcard_imports` (not yet on), review | The guide's own exceptions apply (`use super::*` in tests, `use Enum::*` in a function about that enum). |
| [One module per `use`](https://google.github.io/styleguide/rustguide.html#use-statement) | adopted | rustfmt `imports_granularity = "Module"` | |
| [Locally scoped trait imports](https://google.github.io/styleguide/rustguide.html#prefer-local-trait-imports) | adopted | review | |
| [Module-qualified calls to top-level functions](https://google.github.io/styleguide/rustguide.html#toplevel-functions) | adopted | review | |
| [Type names imported and unqualified](https://google.github.io/styleguide/rustguide.html#type-names) | adopted | review | |
| [`chromium::import!`](https://google.github.io/styleguide/rustguide.html#import-macro) [chrome] | **excluded** | | It is a Chromium build macro; Cargo resolves crates for us. |
| [Use the formatting tool](https://google.github.io/styleguide/rustguide.html#formatting-other) | **changed** | rustfmt (`scripts/fmt.py --check`, a CI step) | The guide's three non-default options exist only in nightly rustfmt, so `python3 scripts/fmt.py` runs a pinned nightly rustfmt (the script names it). The compiler stays on stable (`rust-toolchain.toml`) and nothing else uses nightly. Before rebasing a branch older than the one-time reformat, run the script on it. `.git-blame-ignore-revs` lists that commit; `git config blame.ignoreRevsFile .git-blame-ignore-revs` makes local blame skip it, and GitHub reads the file itself. |
| [Opting out of formatting](https://google.github.io/styleguide/rustguide.html#formatting-optout) | adopted | review | A test that reads source text is not a reason to skip formatting: such a test should parse, not count lines. |
| [Avoid `Rc` and `Arc`](https://google.github.io/styleguide/rustguide.html#avoid-rc-arc) | adopted | review |  |
| [Borrows over deep copies](https://google.github.io/styleguide/rustguide.html#prefer-borrows) | adopted | review | |
| [Flexible references for immutable data](https://google.github.io/styleguide/rustguide.html#flexible-references) | adopted | clippy `ptr_arg`, `borrowed_box` | |
| [Iterators vs owned collections in returns](https://google.github.io/styleguide/rustguide.html#return-iterators) | adopted | review | |
| [Iterators vs slices in parameters](https://google.github.io/styleguide/rustguide.html#accept-iterators) | adopted | review | |
| [Accepting `Into`](https://google.github.io/styleguide/rustguide.html#accepting-into) | adopted | review | The guide's own note binds here: application code almost never takes `impl Into<T>`. |
| [`enum` over `bool` in APIs](https://google.github.io/styleguide/rustguide.html#enum-vs-bool) | adopted | review | |
| [Reporting errors](https://google.github.io/styleguide/rustguide.html#reporting-errors) | **changed** (one addition) | review | Added: a document is outside input, never a precondition. Bytes from a file (a cell value, a number format, a part name, a relationship) fail with `Result`, never a panic. Why: hanji reads files someone else wrote, and a panic on one is a crash any file can trigger; #121 fixed exactly that, an `unwrap` reachable from a cell formatted `0%`. We do not take the guide's option of avoiding panics entirely: a panic still marks a bug in hanji's own code. |
| [Newtypes vs type aliases](https://google.github.io/styleguide/rustguide.html#newtypes-vs-type-aliases) | adopted | review |  |
| [Named fields over tuples](https://google.github.io/styleguide/rustguide.html#prefer-named-fields) | adopted | review | |
| [Add trait impls when needed](https://google.github.io/styleguide/rustguide.html#traits-yagni) | adopted | review | |
| [Implement standard traits](https://google.github.io/styleguide/rustguide.html#known-traits) | adopted | clippy `should_implement_trait` | |
| [`#[derive]` when possible](https://google.github.io/styleguide/rustguide.html#derive) | adopted | clippy `derivable_impls` | |
| [Generics vs `dyn Trait`](https://google.github.io/styleguide/rustguide.html#generics-vs-dyn-trait) | adopted | review |  |
| [Trait vs inherent impl](https://google.github.io/styleguide/rustguide.html#trait-vs-inherent-impl) | adopted | review | |
| [Extension traits](https://google.github.io/styleguide/rustguide.html#extension-traits) | adopted | review | |
| [Guidance for important traits](https://google.github.io/styleguide/rustguide.html#important-traits) | adopted | clippy `new_without_default`, review | |
| [Pair `From` with a constructor](https://google.github.io/styleguide/rustguide.html#from-impls) | adopted | review | |
| [Options structs implement `Default`](https://google.github.io/styleguide/rustguide.html#options-should-implement-default) | adopted | review | Use an options struct where clippy's `too_many_arguments` would fire, instead of `#[allow]`ing it. |
| [Handling poison](https://google.github.io/styleguide/rustguide.html#poison) | **changed** | review | A lock that only serializes (`Mutex<()>`), or that guards a step that cannot stop halfway (one map insert), may ignore poison with `PoisonError::into_inner`. A lock over data with invariants either restores them on poison or panics. Why: the guide's concern is reading state a panic left broken, and a lock that panics forever on a healthy value turns one failed call into a dead library instance. |
| [No `async`/`.await`](https://google.github.io/styleguide/rustguide.html#async-await-chrome) [chrome] | **excluded** |  | Chromium-only. hanji has no `async` today: the library crates do no I/O. |
| [No `std::thread`](https://google.github.io/styleguide/rustguide.html#std-thread) [chrome] | **excluded** |  | Chromium-only: it has its own sequence scheduler. |
| [`fuchsia_sync::Mutex`](https://google.github.io/styleguide/rustguide.html#fuchsia-mutex) [fuchsia] | **excluded** |  | Fuchsia-only. Which mutex to use follows the poison row above. |
| [Macros](https://google.github.io/styleguide/rustguide.html#macros) | adopted | review | |
| [Naming methods](https://google.github.io/styleguide/rustguide.html#naming-methods) | adopted | clippy `wrong_self_convention` | |
| [Naming lifetimes](https://google.github.io/styleguide/rustguide.html#lifetimes) | adopted | review | |
| [Naming generic parameters](https://google.github.io/styleguide/rustguide.html#generic-parameters) | adopted | review | |
| [Naming extension traits](https://google.github.io/styleguide/rustguide.html#naming-extension-traits) | adopted | review | |
| [Naming initialization functions](https://google.github.io/styleguide/rustguide.html#naming-initialization-functions) | adopted | review | A constructor that returns `Result` is `try_new`. Older ones are renamed when their file is next touched. |
| [Avoid abbreviations](https://google.github.io/styleguide/rustguide.html#avoid-abbreviations) | **changed** (one addition) | review | Format vocabulary counts as known: the format names (`docx`, `pptx`, `xlsx`, `hwpx`, `ooxml`) and the terms in DESIGN.md §3 "Terms and non-goals". |
| [When and how to write documentation](https://google.github.io/styleguide/rustguide.html#writing-doc) | adopted | review; `missing_docs` once the tree meets it |  |
| [Unsafe code](https://google.github.io/styleguide/rustguide.html#unsafe) | adopted | clippy `undocumented_unsafe_blocks` (deny), review | |
| [No unstable features](https://google.github.io/styleguide/rustguide.html#unstable-features) | adopted | the stable compiler | Nightly rustfmt is a formatter, not a language feature. |
| [Getters and setters](https://google.github.io/styleguide/rustguide.html#getters-and-setters) | adopted | review | |
| [Accessors that accept closures](https://google.github.io/styleguide/rustguide.html#closure-accessors) | adopted | review | |
| [Implement expected API sets together](https://google.github.io/styleguide/rustguide.html#expected-api-sets) | adopted | clippy `len_without_is_empty` | |
| [Don't use others' `#[doc(hidden)]`](https://google.github.io/styleguide/rustguide.html#doc-hidden) | adopted | review | |
| [`const fn` responsibly](https://google.github.io/styleguide/rustguide.html#const-functions) | adopted | review |  |
| [Operator overloads](https://google.github.io/styleguide/rustguide.html#operator-overloads) | adopted | review | |
| [`#[non_exhaustive]` on public enums](https://google.github.io/styleguide/rustguide.html#non-exhaustive-enums) | adopted | review | Library code. Adding `#[non_exhaustive]` to a public type is itself a breaking change, so the types that lack it gain it in one minor release (0.6.0), not one by one. |
| [`#[non_exhaustive]` on public structs](https://google.github.io/styleguide/rustguide.html#non-exhaustive-structs) | adopted | review | Same as the row above. |
| [`#[must_use]` on functions](https://google.github.io/styleguide/rustguide.html#must-use-on-functions) | adopted | review | Every library crate is library code, so this binds there in full. `hanji-cli` and `hanji-testkit` are application code. |
| [`#[must_use]` on types](https://google.github.io/styleguide/rustguide.html#must-use-on-types) | adopted | review | |
| [No code before `main`](https://google.github.io/styleguide/rustguide.html#global-initialization) | adopted | review | |
| [Don't overuse type inference](https://google.github.io/styleguide/rustguide.html#type-inference) | adopted | review | |
| [Pattern match over check-then-unwrap](https://google.github.io/styleguide/rustguide.html#dont-check-then-unwrap) | adopted | clippy `unnecessary_unwrap` | |
| [`expect` over `unwrap`](https://google.github.io/styleguide/rustguide.html#expect-over-unwrap) | adopted | review (clippy `unwrap_used` not yet on) | The rule is about panics that stand for an invariant of our code. A test's `unwrap` on its own setup is not one: the test name already says what failed, so tests may `unwrap`. |
| [Don't omit patterns on `non_exhaustive` enums](https://google.github.io/styleguide/rustguide.html#non-exhaustive-enum-matching) | adopted | review |  |
| [Context strings on assertions](https://google.github.io/styleguide/rustguide.html#assertion-context) | adopted | review | |
| [Combinator chains vs direct iteration](https://google.github.io/styleguide/rustguide.html#combinator-chains) | adopted | clippy `needless_for_each`, review | |
| [Don't call `catch_unwind`](https://google.github.io/styleguide/rustguide.html#catch-unwind) | **changed** | review | Allowed in tests that assert a panic, and in a host that embeds hanji (the CLI, a WASM binding) to turn a panic into an error at its own boundary, saying why at the call. The library crates never catch: a caller chooses its own panic strategy. Why: the guide's ban rests on `panic = abort` in Chrome, Android and Fuchsia, and hanji cannot choose that for the programs that link it. |
| [`format!`/`write!` for concatenation](https://google.github.io/styleguide/rustguide.html#prefer-format-or-write) | adopted | review | |
| [Per-codebase test framework](https://google.github.io/styleguide/rustguide.html#test-framework) [chrome, fuchsia] | **excluded** |  | Those frameworks plug Rust tests into C++ and Fuchsia harnesses. hanji uses `#[test]` under `cargo test`, natively and on wasm32-wasip1. |

## What the tools cannot see

Most of the guide is judgement, and only review holds it: module-qualified
calls, `enum` over `bool`, options structs, newtypes, `expect` with a reason,
explicit types on `.into()` and `.parse()`, combinator chains, `Rc`/`Arc` and
`.clone()` against borrowing, assertion messages, poison handling. An
`#[allow]` silences any lint, and nothing yet requires it to give a reason. A
lint is added to `[workspace.lints]` only once the tree meets it, because a lint
with open sites is a red build.
