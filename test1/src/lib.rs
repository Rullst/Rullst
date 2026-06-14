use rullst::{routes, Router};

pub mod controllers;
pub mod pages;

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {
    let router = routes![
        get("/" => controllers::portfolio_controller::index),
    ];
    Box::into_raw(Box::new(router))
}
