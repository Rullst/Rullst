# 💡 The Rullst Philosophy

Rullst aims to coordinate the parts of a production Rust web application while
preserving Rust's explicitness. The framework favors static dispatch,
compile-time generation, typed errors, and APIs whose security boundaries can
be inspected. Convenience is useful only when the generated result remains
understandable and reviewable.

### Our Core Tenets

1. **Coordinated, not magical:** Rullst integrates routing, auth, ORM, bounded
   background jobs, and developer tooling behind explicit APIs. Generated
   defaults reduce setup, but every deployed application's security and
   operations still require review.

2. **Built for Humans and AIs:** Rullst is architected to be legible and explicit, with static dispatch and compile-time generation where practical. This helps human developers and coding agents collaborate on systems whose production boundaries can be reviewed and tested.

3. **Evidence before claims:** Features, performance, and security statements
   must name their scope and limits. Tests and workflows are evidence for a
   specific revision, not certification of every downstream deployment.
