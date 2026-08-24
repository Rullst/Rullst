# Tutorial 01: Hello Rullst! 🚀

Welcome to Rullst! In this tutorial, you will build your very first Rullst web application, render high-performance HTML using the `rullst::html!` macro, and return JSON responses.

---

## 🛠️ Step 1: Create a New Project

Run the Rullst CLI command to scaffold your first application:

```bash
cargo rullst new my_first_app
cd my_first_app
```

Select the **Blank Starter** blueprint when prompted by the wizard.

---

## 💻 Step 2: Define your First Route

Open `src/main.rs`. You will see the standard Rullst application setup:

```rust
use rullst::{html, response::Html, routes, AppError, Server};

pub async fn home() -> Result<Html<String>, AppError> {
    Ok(Html(html! {
        <div class="min-h-screen bg-slate-900 text-emerald-400 flex flex-col items-center justify-center font-sans">
            <h1 class="text-5xl font-extrabold mb-4">"Hello, Rullst! 📜🦀"</h1>
            <p class="text-slate-400 text-lg">"Built for Emotional Productivity and Extreme Security."</p>
        </div>
    }))
}

#[tokio::main]
async fn main() {
    let app = routes![
        get("/" => home)
    ];

    Server::new(app)
        .run(3000)
        .await
        .unwrap();
}
```

---

## 🧪 Step 3: Run the Development Server

Start the live development server with hot-reloading:

```bash
cargo rullst dev
```

Open your browser at `http://localhost:3000` to view your rendered HTML page!

---

## 💡 Key Takeaways
- The `rullst::html!` macro compiles HTML templates down to zero-cost string concatenation at compile time.
- All boolean HTML attributes inside `html!` must be explicitly quoted (e.g. `required="true"`).
- Handlers return `Result<Response, AppError>` so expected failures are typed instead of converted into framework-originated panics.
