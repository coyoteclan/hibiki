use font_kit::source::SystemSource;
use once_cell::sync::Lazy;
use std::sync::Arc;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum FontError {
    #[error("Failed to discover system fonts")]
    DiscoveryFailed,
    #[error("Font configuration initialization failed")]
    ConfigFailed,
    #[error("Font family not found: {0}")]
    FontNotFound(String),
}

static SYSTEM_FONTS: Lazy<Result<Arc<Vec<String>>, String>> = Lazy::new(|| {
    let source = SystemSource::new();
    match source.all_families() {
        Ok(mut families) => {
            families.sort();
            families.dedup();
            families.shrink_to_fit();
            Ok(Arc::new(families))
        }
        Err(e) => Err(format!("Font discovery failed: {e}")),
    }
});

pub struct FontProvider;

impl FontProvider {
    /// Fetch system fonts without blocking the calling thread.
    /// Returns a sorted list of unique font families.
    ///
    /// # Errors
    ///
    /// Returns `FontError::DiscoveryFailed` if the underlying font system fails to list families.
    /// Returns `FontError::ConfigFailed` if the async channel operation fails.
    pub async fn load_system_fonts_asynchronous() -> Result<Arc<Vec<String>>, FontError> {
        tokio::task::spawn_blocking(move || match &*SYSTEM_FONTS {
            Ok(fonts) => Ok(fonts.clone()),
            Err(_) => Err(FontError::DiscoveryFailed),
        })
        .await
        .map_err(|_| FontError::ConfigFailed)?
    }

    /// Validate if a font family exists
    ///
    /// # Errors
    ///
    /// Returns `FontError::FontNotFound` if the specified family is not present in the system.
    /// Can also return errors from `load_system_fonts_asynchronous`.
    pub async fn validate_font_family(family: String) -> Result<(), FontError> {
        let fonts = Self::load_system_fonts_asynchronous().await?;
        if fonts.binary_search(&family).is_ok() {
            Ok(())
        } else {
            Err(FontError::FontNotFound(family))
        }
    }
}
