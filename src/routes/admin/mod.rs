/// # Admin module
/// ## Note: New admin user registration not implemented.
/// In the book, we assume the first user is seeded by database migration.
///
mod dashboard;
mod logout;
mod password;

pub use dashboard::admin_dashboard;
pub use logout::log_out;
pub use password::*;
