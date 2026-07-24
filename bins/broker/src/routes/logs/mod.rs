mod get;

use actix_web::{Scope, web};

pub fn scope() -> Scope {
    web::scope("/tenants/{tenant_id}").service(get::get)
}
