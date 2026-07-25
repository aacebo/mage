use actix_web::{Scope, web};

mod connect;
mod create;

pub fn scope() -> Scope {
    web::scope("/agents").service(connect::connect).service(create::create)
}
