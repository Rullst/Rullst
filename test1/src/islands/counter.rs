use rullst::{island, view};
use wasm_bindgen::prelude::*;
use serde::{Serialize, Deserialize};
use web_sys::HtmlElement;

#[derive(Serialize, Deserialize, Clone)]
pub struct CounterProps {
    pub initial: i32,
}

#[island]
pub fn counter(props: CounterProps, container: HtmlElement) {
    let doc = web_sys::window().unwrap().document().unwrap();
    let btn = doc.create_element("button").unwrap();
    btn.set_inner_html(&format!("Click to increment! Current: {}", props.initial));
    
    // In a real app, you would call your RPC server function here!
    let closure = Closure::wrap(Box::new(move || {
        web_sys::console::log_1(&"Calling server via RPC...".into());
    }) as Box<dyn FnMut()>);
    
    btn.add_event_listener_with_callback("click", closure.as_ref().unchecked_ref()).unwrap();
    closure.forget();
    
    container.append_child(&btn).unwrap();
}
