//! Error handling for JNI bindings.

use jni::errors::Error as JniError;
use jni::JNIEnv;
use thiserror::Error;
use liteforge::ForgeError;

#[derive(Debug, Error)]
pub enum JavaBindingError {
    #[error("JNI error: {0}")]
    Jni(#[from] JniError),

    #[error("LiteForge error: {0}")]
    Forge(#[from] ForgeError),

    #[error("Null pointer error: {0}")]
    NullPointer(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),
}

pub type Result<T> = std::result::Result<T, JavaBindingError>;

pub fn throw_exception(env: &mut JNIEnv, error: JavaBindingError) {
    let class_name = match &error {
        JavaBindingError::NullPointer(_) => "java/lang/NullPointerException",
        JavaBindingError::InvalidArgument(_) => "java/lang/IllegalArgumentException",
        _ => "java/lang/RuntimeException",
    };

    let message = error.to_string();
    let _ = env.throw_new(class_name, message);
}

pub fn handle_result<T>(env: &mut JNIEnv, result: Result<T>, default: T) -> T {
    match result {
        Ok(value) => value,
        Err(e) => {
            throw_exception(env, e);
            default
        }
    }
}
