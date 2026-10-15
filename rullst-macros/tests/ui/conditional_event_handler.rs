use rullst_macros::html;

fn main() {
    let handler = Some("alert(1)");
    let _ = html! {
        <button onclick?={handler}>"Go"</button>
    };
}
