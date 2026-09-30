use rullst_macros::html;

fn main() {
    let _ = html! {
        <button onclick={"alert(1)"}>"Go"</button>
    };
}
