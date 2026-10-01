//! Project-specific Markdown fences and inline image shortcodes.
//! The calling Markdown compiler owns document parsing, HTML assembly, and metadata.

pub mod embed;
mod emoji;
pub use emoji::render_emojis;
mod source;
pub use source::normalize_embed_examples;

pub use embed::{ArticleMetadata, EmbedError, article_ids, article_urls, collect_media_paths};
