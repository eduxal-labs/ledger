pub mod user {
    tonic::include_proto!("user");
}
pub mod verification {
    tonic::include_proto!("verification");
}
#[allow(dead_code)]
pub mod role {
    tonic::include_proto!("role");
}
pub mod member {
    tonic::include_proto!("member");
}
pub mod event {
    tonic::include_proto!("event");
}
pub mod paper {
    tonic::include_proto!("paper");
}
