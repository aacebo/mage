use actix_web::{Scope, web};

mod create;
mod get;

pub fn scope() -> Scope {
    web::scope("/agents").service(get::get).service(create::create)
}
