//! Project-specific Markdown fences: validation, provider data, and rendered embeds.
//! The calling Markdown compiler owns document parsing, HTML assembly, and metadata.

mod embed;

pub use embed::{
    ArticleMetadata, Data as EmbedData, EmbedError, add_styles as add_embed_styles, article_ids,
    article_urls, collect_media_paths, load as load_embeds,
    load_with_articles as load_embeds_with_articles, render as render_embed,
};
