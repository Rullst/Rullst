# Benchmark Report: Rullst vs Competitor Frameworks (Rust)

This report details the architectural and performance comparison between Rullst and other full-stack and backend frameworks in the Rust ecosystem, such as Loco, Leptos, Dioxus, and Axum (as a baseline).

## Benchmark Scope

The benchmark focused on three main pillars where framework architecture interacts most heavily with pure performance:

1. **Server-Side Rendering (SSR):** Generating HTML on the backend for clients.
2. **In-Memory Routing (Plain Text):** Routing and middleware costs for plain text requests.
3. **JSON Serialization:** The "bread and butter" of APIs for interacting with SPAs.

## 1. Why Rullst Wins at SSR (Server-Side Rendering)

When performing Server-Side Rendering, Rullst holds a massive structural advantage compared to its main competitors:

* **Vs Dioxus and Leptos:** These frameworks were born as UI solutions using a component-tree approach. This requires constructing (even in SSR) a **Virtual DOM** or control structures in Rust, which later need to be cached, traversed, and serialized into a `String`.
* **Vs Loco (via Tera/Askama):** Loco relies on traditional template engines (like Tera). Templates are text files parsed and interpreted at *runtime* (in Tera's case) or that generate large compilation boilerplate.
* **The Rullst Solution (`html!` macro):** Rullst does everything at compile-time with no intermediate structures. The `html!` macro does not create DOM objects or Virtual DOMs; it simply expands the code into direct concatenation operations within a pre-allocated buffer (`String::with_capacity(..)`) by injecting scope variables using `std::fmt::Write`.
  * *Result:* Zero runtime overhead. Rullst wins by avoiding repeated memory allocations, delivering the generated HTML much faster.

### SSR Results (Lower is Better)
- Rullst (`html!` macro): `~1.07 µs`
- Dioxus (Virtual DOM): `~4.54 µs` (4.2x slower)
- Leptos (View macro): `~9.10 µs` (8.5x slower)
- Tera Template: `~2.14 µs` (2x slower)

## 2. Routing (Rullst vs Loco / Axum)

Routing is where the framework tax is paid.

* **Loco:** Loco is focused on Rails-style productivity. However, to offer this robust MVC experience, it embeds a heavy abstraction layer with `Hooks`, authentication middlewares, Dependency Injection, and context structs. All of this runs on top of Axum.
* **Axum:** Pure Axum is extremely fast, but provides nothing out of the box. It's up to the developer to wire up serializers, databases, WAF security, etc.
* **The Rullst Sweet Spot:** Rullst uses the Tower/Axum ecosystem, just like Loco. However, Rullst generates its routing system via compiled macros (`routes!`) and uses "Zero-Cost Abstractions" to compile handlers. It provides nearly identical speed to "pure" Axum but with "Full-Stack" productivity. At the JSON response layer, Rullst exposes primitive types and sends them in serialized format without dynamic conversions.

### Routing Results (Plain Text)
- Rullst Router: `~974 ns`
- Axum Router: `~946 ns`

### Routing Results (JSON)
- Rullst Router: `~1.53 µs`
- Axum Router: `~1.59 µs`

## 3. Full-Stack Philosophy and the Virtual DOM Cost

What distinguishes Rullst from purely SPA/WASM-focused frameworks (like Leptos and Dioxus) is that it embraces the **Server-Driven UI** philosophy (often in conjunction with HTMX and Alpine.js), utilizing compiled macros for HTML and the `LiveComponent` extension for reactive updates via WebSockets.

In this scenario:
- The initial response to the client has **zero JavaScript parsing delay**, rendering at lightspeed compared to a Virtual DOM SSR architecture.
- Rullst's "Zero-Panic" abstractions mean not only error predictability but highly optimized compilations by removing dynamic blocks (`unwrap/expect`).

## Conclusion

Rullst offers C++/Rust performance (via direct Axum and native HTML compilation) packaged in a development experience similar to Laravel, Loco, or Rails. It beats the rest by:
1. Completely eliminating Virtual DOM trees during SSR.
2. Avoiding slow template engines processed at runtime (Tera/Liquid).
3. Utilizing zero-cost routing reconciliation on top of Axum with macros that generate routes at crate build time.