use rullst::{routes, Router};

pub mod migrations;
pub mod models;
pub mod controllers;
pub mod pages;

#[unsafe(no_mangle)]
pub extern "C" fn rullst_router_init() -> *mut Router {
    let nexus = rullst::nexus::Nexus::new()
        .with_auth("admin", "password")
        .with_brand("Blog Admin")
        .register::<models::post::Post>()
        .build();

    let router = routes![
        get("/" => controllers::blog_controller::index),
        get("/posts/{slug}" => controllers::blog_controller::show),
    ].nest_axum("/nexus", nexus);
    Box::into_raw(Box::new(router))
}
