use actix_web::{Scope, web};

pub mod agents;
pub mod logs;
pub mod messages;

pub fn scope() -> Scope {
    web::scope("/tenants/{tenant_id}")
        .service(agents::scope())
        .service(logs::scope())
        .service(messages::scope())
}
