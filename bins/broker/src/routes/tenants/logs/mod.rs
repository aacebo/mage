mod get;

use actix_web::{Scope, web};

pub fn scope() -> Scope {
    web::scope("/logs").service(get::get)
}
