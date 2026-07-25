use actix_web::{Scope, web};

mod connect;

pub fn scope() -> Scope {
    web::scope("/agents").service(connect::connect)
}
