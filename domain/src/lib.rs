use uuid::Uuid;
pub mod address;
pub mod error;
pub mod relay;
pub mod session;
pub fn generate_uuid() -> String {
    Uuid::new_v4().to_string().replace("-", "").to_uppercase()
}
