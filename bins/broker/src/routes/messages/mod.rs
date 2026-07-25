use actix_web::{Scope, web};

mod create;

pub fn scope() -> Scope {
    web::scope("/messages").service(create::create)
}
