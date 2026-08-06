//! Message content: [`Content`], [`ContentPart`], [`ContentView`], and
//! [`MediaSource`].

use serde::{Deserialize, Serialize};

/// Stores where a media part's bytes live: at a URL, or inline as base64.
///
/// The MIME type on [`MediaSource::Base64`] is required by construction,
/// and is used by providers to interpret the payload.
///
/// # Serialized examples
///
/// ```json
/// {"kind": "url", "url": "https://example.com/cat.png"}
/// {"kind": "base64", "mime_type": "application/pdf", "data": "azhYI205orcaUHEhMnZMJD="}
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MediaSource {
    /// Referenced by URL (or a `data:` URI).
    Url {
        /// The location.
        url: String,
    },
    /// Carried inline, base64-encoded.
    Base64 {
        /// MIME type of the payload, e.g. `application/pdf`.
        mime_type: String,
        /// The payload, base64-encoded.
        data: String,
    },
}

impl MediaSource {
    /// Builds a [`MediaSource::Url`].
    pub fn url(url: impl Into<String>) -> Self {
        Self::Url { url: url.into() }
    }
    /// Builds a [`MediaSource::Base64`].
    pub fn base64(mime_type: impl Into<String>, data: impl Into<String>) -> Self {
        Self::Base64 {
            mime_type: mime_type.into(),
            data: data.into(),
        }
    }
}

/// One typed part of a multi-part [`Content`].
///
/// Tagged on the wire; providers translate to their own wire shape where it
/// differs.
///
/// # Serialized examples
///
/// ```json
/// {"type": "text", "text": "What do orcas do at night?"}
/// {
///   "type": "image",
///   "source": {
///     "kind": "url",
///     "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/3/37/Killerwhales_jumping.jpg/500px-Killerwhales_jumping.jpg"
///   }
/// }
/// {
///   "type": "file",
///   "source": {
///     "kind": "base64",
///     "mime_type": "application/pdf",
///     "data": "azhYI205orcaUHEhMnZMJD="
///   },
///   "name": "orca.pdf"
/// }
/// ```
// TODO: `Audio` / `Video` variants — deferred until a provider can consume
// them.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentPart {
    /// Plain text.
    Text {
        /// The text.
        text: String,
    },
    /// An image.
    Image {
        /// Where the image bytes live.
        source: MediaSource,
    },
    /// A file (document, PDF, etc).
    File {
        /// Where the file bytes live.
        source: MediaSource,
        /// Filename hint; some providers require one for inline payloads.
        #[serde(skip_serializing_if = "Option::is_none", default)]
        name: Option<String>,
    },
}

impl ContentPart {
    /// Builds a [`ContentPart::Text`].
    pub fn text(text: impl Into<String>) -> Self {
        Self::Text { text: text.into() }
    }
    /// Builds a [`ContentPart::Image`].
    pub fn image(source: MediaSource) -> Self {
        Self::Image { source }
    }
    /// Builds a [`ContentPart::Image`] from a URL.
    pub fn image_url(url: impl Into<String>) -> Self {
        Self::Image {
            source: MediaSource::url(url),
        }
    }
    /// Builds a [`ContentPart::File`] without a filename hint.
    pub fn file(source: MediaSource) -> Self {
        Self::File { source, name: None }
    }
}

/// A message's content: plain text, or multi-part (can be multimodal).
///
/// `untagged` on the wire, so the common case keeps the plain-string shape.
///
/// # Serialized examples
///
/// ```json
/// "hello"
/// [
///   {"type": "text", "text": "hi"},
///   {
///     "type": "image",
///     "source": {
///       "kind": "url",
///       "url": "https://upload.wikimedia.org/wikipedia/commons/thumb/3/37/Killerwhales_jumping.jpg/500px-Killerwhales_jumping.jpg"
///     }
///   }
/// ]
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Content {
    /// Plain text field apart from [`Content::Parts`] for simple text
    /// content responses.
    Text(String),
    /// Typed content parts, in order.
    Parts(Vec<ContentPart>),
}

impl Content {
    /// Pulls a borrowed view of the content: [`ContentView::Text`] for
    /// [`Content::Text`], and [`ContentView::Parts`] for [`Content::Parts`].
    pub fn pull(&self) -> ContentView<'_> {
        match self {
            Self::Text(t) => ContentView::Text(t),
            Self::Parts(p) => ContentView::Parts(p),
        }
    }

    /// Appends text: concatenated onto [`Content::Text`], appended as a new
    /// [`ContentPart::Text`] onto [`Content::Parts`].
    pub fn push_text(&mut self, text: impl Into<String>) {
        match self {
            Self::Text(s) => s.push_str(&text.into()),
            Self::Parts(p) => p.push(ContentPart::text(text)),
        }
    }

    /// Appends a part, promoting [`Content::Text`] to [`Content::Parts`] if
    /// needed. The existing text is carried over as a leading
    /// [`ContentPart::Text`] (dropped when empty).
    pub fn push_part(&mut self, part: ContentPart) {
        match self {
            Self::Parts(p) => p.push(part),
            Self::Text(t) => {
                let text = std::mem::take(t);
                let mut parts = Vec::with_capacity(1 + usize::from(!text.is_empty()));
                if !text.is_empty() {
                    parts.push(ContentPart::text(text));
                }
                parts.push(part);
                *self = Self::Parts(parts);
            }
        }
    }
}

/// A borrowed view of a [`Content`], returned by [`Content::pull`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentView<'a> {
    /// The text of a [`Content::Text`].
    Text(&'a str),
    /// The parts of a [`Content::Parts`].
    Parts(&'a [ContentPart]),
}

impl From<&str> for Content {
    fn from(s: &str) -> Self {
        Self::Text(s.to_owned())
    }
}
impl From<String> for Content {
    fn from(s: String) -> Self {
        Self::Text(s)
    }
}
impl From<Vec<ContentPart>> for Content {
    fn from(parts: Vec<ContentPart>) -> Self {
        Self::Parts(parts)
    }
}
impl From<ContentPart> for Content {
    fn from(part: ContentPart) -> Self {
        Self::Parts(vec![part])
    }
}
