mod connect;
mod index;

use actix_web::{Scope, web};

pub fn scope() -> Scope {
    web::scope("/console").service(index::page).service(connect::connect)
}
