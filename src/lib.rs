#![feature(proc_macro_hygiene, decl_macro, custom_attribute)]

#[macro_use]
extern crate lazy_static;
extern crate regex;

#[macro_use]
extern crate rocket;
extern crate rocket_contrib;

#[macro_use]
extern crate serde_derive;
extern crate serde;

extern crate diesel;
extern crate r2d2;
extern crate r2d2_diesel;

use rocket_contrib::templates::Template;

pub mod config;
pub mod db;

mod routes {
    pub mod auth;
    pub mod index;
    pub mod assets;
    pub mod errors;
}

pub fn app(env_name: &str) -> rocket::Rocket {
    let config_name = match env_name {
        "test" => "testing",
        _ => env_name,
    };

    let config = config::Config::from(config_name).unwrap();
    let pool = db::init_pool(&config.database_url);

    rocket::ignite()
        .manage(pool)
        .mount(
            "/",
            routes![
                routes::auth::login_get,
                routes::auth::login_post,
                routes::index::index,
                routes::assets::assets
            ],
        )
        .attach(Template::fairing())
        .register(catchers![routes::errors::not_found])
}
