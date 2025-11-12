// Library exports for testing and external use

pub mod db;
pub mod entities;
pub mod routes;

// Re-export commonly used types
pub use db::Database;
pub use entities::{Note, Branch, Attribute, Blob};
