//! Project-specific Markdown fences: validation, provider data, and rendered embeds.
//! The calling Markdown compiler owns document parsing, HTML assembly, and metadata.

mod embed;

pub use embed::{
    Data as EmbedData, EmbedError, add_styles as add_embed_styles, collect_media_paths,
    load as load_embeds, render as render_embed,
};
