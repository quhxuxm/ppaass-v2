use uuid::Uuid;
pub mod address;
pub mod error;
mod heartbeat;
pub mod tunnel;
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "").to_uppercase()
}
