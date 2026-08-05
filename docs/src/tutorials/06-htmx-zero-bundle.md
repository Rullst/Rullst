# Tutorial 06: Zero-Bundle HTMX UI 🎨

Rullst adopts a **Zero-Bundle Frontend** philosophy by default: combining HTML5 SSR, TailwindCSS, and HTMX to build rich reactive interfaces without loading megabytes of JavaScript frameworks.

---

## 🛠️ Step 1: Render HTMX Components

In your controller or view:

```rust
use rullst::{html, Response, AppError};

pub async fn search_users(query: String) -> Result<Response, AppError> {
    let results = vec!["Alice", "Bob", "Charlie"];
    
    Ok(html! {
        <ul id="user-list" class="divide-y divide-slate-700">
            { results.into_iter().map(|name| html! {
                <li class="py-2 text-slate-200">{ name }</li>
            }) }
        </ul>
    })
}
```

---

## 💻 Step 2: Wire HTMX Attributes in Front-End HTML

```html
<div class="max-w-md mx-auto p-6 bg-slate-800 rounded-xl shadow-md">
    <input 
        type="text" 
        name="query" 
        placeholder="Search users..." 
        class="w-full px-4 py-2 bg-slate-900 text-white rounded border border-slate-700 focus:outline-none"
        hx-post="/api/users/search" 
        hx-trigger="keyup changed delay:300ms" 
        hx-target="#user-list" 
        hx-swap="outerHTML" 
    />
    
    <div id="user-list" class="mt-4 text-slate-400">
        "Start typing to search..."
    </div>
</div>
```

---

## 💡 Key Takeaways
- **0KB JavaScript Bundle:** HTMX handles AJAX, WebSockets, and CSS transitions using HTML attributes.
- **Partial View Rendering:** Handlers return tiny HTML fragments instead of full pages, delivering instant initial page load speeds.
