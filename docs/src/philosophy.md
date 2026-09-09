# The Rullst philosophy

Rullst exists to make ambitious software feel possible in Rust without hiding
the language, the generated code, or the boundaries that keep an application
safe. It aims to be productive and broad, but its convenience must remain
inspectable: static dispatch, compile-time generation, typed errors, explicit
middleware, and ordinary Rust escape hatches are deliberate choices.

## A story that stayed with us

A widely reported account of Rust's origin begins with a broken elevator in
Graydon Hoare's apartment building. Its software had crashed, leaving him to
climb the stairs. Rust itself began as Hoare's personal project in 2006 and
grew into a language designed to combine systems-level performance with much
stronger memory-safety guarantees.

The elevator account is an origin story reported by
[MIT Technology Review](https://www.technologyreview.com/2023/02/14/1067869/rust-worlds-fastest-growing-programming-language/),
not a claim that every elevator failure was a memory bug or that Rust can make
all software infallible. The lesson Rullst takes from it is narrower and more
useful: when an entire class of failure can be prevented by design, prevention
is better than asking every developer to rediscover the same danger.

## Why Venelouis started Rullst

Rullst also has its own practical origin. Long before modern AI systems became
capable coding partners, Venelouis wanted to build an education platform. He
first learned what was possible by operating a Moodle installation on a VPS.
Later, Laravel and AI helped him create a far more ambitious education product
than he had previously believed he could build.

That experience made Laravel's greatest strength tangible: a framework can let
one person concentrate on the product instead of repeatedly assembling its
foundations. Venelouis then wanted to rebuild that kind of product in Rust to
use fewer resources, gain performance, and inherit Rust's stronger compile-time
guarantees. He could not find a Rust framework that combined the particular
Laravel-like product workflow, breadth, approachability, and explicit security
boundaries he wanted. Rullst began as an attempt to build that missing bridge.

Rullst is not a Laravel clone, and Rust should not be forced to behave like
PHP. The inspiration is the feeling that a complete product is within reach;
the implementation follows Rust's strengths rather than concealing them.

## Why Rullst jumps from version 5 to version 12

The version jump records the history of the ecosystem rather than six hidden
major releases of the unified framework. When Venelouis began recreating
`creio.eu` in Rust with AI assistance, the Gemini model he was using at the
time told him that there was no direct Rust counterpart to Laravel Socialite
for the workflow he needed. Instead of abandoning that part of the product,
he created the first independent Rullst crate. That project became
`rullst-connect`, which still carries the identity and social-login mission of
that original work.

The crates initially evolved in separate repositories and at different
speeds. By the time the umbrella Rullst framework was at version 5,
`rullst-connect` had already reached version 11. Bringing the projects into one
monorepo made their dependency changes, cross-crate compatibility tests, and
coordinated releases easier to maintain. Aligning every publishable package on
one version line was the natural next step.

A clean reset to 1.0 might have described the beginning of this unified era,
but a registry history cannot be reset that way: crates.io does not permit a
published version to be overwritten or removed, and lower new versions would
not erase the already published higher ones. Rullst therefore advances the
whole ecosystem directly to version 12, monotonically beyond Connect's version
11. From the version 12 line onward, coordinated releases can follow the usual
Semantic Versioning sequence. The jump is thus an act of package alignment and
honest continuity, not a claim that standalone Rullst versions 6 through 11
were released as complete framework generations.

## Core tenets

1. **Productive, not magical.** Rullst coordinates routing, data, identity,
   security helpers, background work, AI, and developer tooling through APIs
   and generated source that users can inspect and replace.

2. **Simple to begin, explicit when it matters.** Good defaults should remove
   repetitive setup. Authorization, tenancy, provider behavior, persistence,
   deployment, and recovery must still expose their real application-owned
   decisions.

3. **Prevent failure classes by design.** Typed errors, bounded inputs,
   parameterized queries, compile-time generation, and fail-closed behavior are
   preferred to conventions that only work when every caller remembers them.
   This reduces risk; it does not create a universal security guarantee.

4. **Built for humans and AIs.** Stable vocabulary, focused modules, static
   contracts, executable examples, and documented limits give human developers
   and coding agents a shared map of the system. AI assistance never replaces
   review, testing, or accountable approval.

5. **Evidence before claims.** A feature claim must name its implemented scope
   and residual boundary. A test belongs to the revision and environment it
   exercised. Benchmarks describe measured workloads, not universal rankings.

6. **Emotional productivity matters.** The framework should make builders feel
   capable, curious, and supported. Removing boilerplate is valuable when it
   leaves more attention for users, learning, and the purpose of the product.

## The ambition and the boundary

The long-term ambition is deliberately large: make Rullst a foundation from
which people can build many kinds of applications without surrendering Rust's
performance or explicitness. That ambition is a direction, not a promise that
one framework can finish every product, operate every provider, or replace the
judgment of its developers.

Rullst will therefore grow through bounded, reviewable capabilities. Where the
repository has proof, the documentation should show it. Where real devices,
provider accounts, production operations, independent review, or application
policy are still required, the documentation should say so plainly. The goal
is not to look complete. It is to keep becoming more useful without losing the
trust of the people building with it.
