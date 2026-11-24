//! What a layout paints with: fills and strokes.
//!
//! A pane background is a plain colour, a gradient or an image, and a pane border is a line with a
//! width and a colour. Each is read from the layout file and written back with every key and word
//! it does not name, so a layout that uses a newer herdr-app feature is not damaged by being read.
//!
//! Responsibilities:
//!
//! - [`Paint`], a colour, a gradient, an image or an unrecognised object, tried in that order
//! - [`Fit`], how an image fills its area, keeping any word it does not know
//! - [`Line`], an optional width and colour, keeping any other key
//!
//! Non-responsibilities:
//!
//! - deciding whether an image path exists or a colour name is defined, which herdr-app does
//! - drawing anything

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::files::{Color, Number};

/// How an image fills its area.
///
/// A word this crate does not know is kept as [`Fit::Other`] and written back as it was read.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Fit;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// assert_eq!(serde_json::from_str::<Fit>("\"cover\"")?, Fit::Cover);
/// assert_eq!(serde_json::from_str::<Fit>("\"tile\"")?, Fit::Other("tile".to_owned()));
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fit {
    /// The image covers the whole area.
    Cover,
    /// The image is contained within the area.
    Contain,
    /// A word this crate does not know, as written.
    #[serde(untagged)]
    Other(String),
}

/// A stroke: an optional width and an optional colour.
///
/// Every key other than `width` and `color` is kept in `extra`. An absent field is not written. The
/// default is an empty line.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::Line;
///
/// # fn main() -> Result<(), serde_json::Error> {
/// let line = Line::default();
/// assert_eq!(line.width, None);
/// assert_eq!(line.color, None);
/// assert_eq!(serde_json::to_string(&line)?, "{}");
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Line {
    /// The stroke width.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<Number>,
    /// The stroke colour.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<Color>,
    /// Every other key of the object, as written.
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

/// A fill: a colour, a gradient, an image, or an object this crate does not recognise.
///
/// Variants are tried in the order declared. An object that fits none of the first three, including
/// one whose known key holds an invalid value, is kept whole as [`Paint::Other`] and written back as
/// it was read. A value that is neither a matching variant nor an object is refused.
///
/// # Examples
///
/// ```
/// use herdrify_app::files::{Color, Paint};
///
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// let paint: Paint = serde_json::from_str("\"border\"")?;
/// assert_eq!(paint, Paint::color(Color::new("border")?));
/// assert!(serde_json::from_str::<Paint>("7").is_err());
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Paint {
    /// A single colour, by name or hex value.
    Color(Color),
    /// A gradient through the listed colours.
    Gradient {
        /// The colours, in order.
        gradient: Vec<Color>,
        /// The gradient angle.
        #[serde(skip_serializing_if = "Option::is_none")]
        angle: Option<Number>,
        /// Every other key of the object, as written.
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
    /// An image, by path.
    Image {
        /// The image path, as written.
        image: String,
        /// How the image fills its area.
        #[serde(skip_serializing_if = "Option::is_none")]
        fit: Option<Fit>,
        /// Every other key of the object, as written.
        #[serde(flatten)]
        extra: Map<String, Value>,
    },
    /// An object that is none of the above, kept whole.
    Other(Map<String, Value>),
}

impl Paint {
    /// A fill of the single colour `color`.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Color, Paint};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let paint = Paint::color(Color::new("canvas")?);
    /// assert_eq!(serde_json::to_string(&paint)?, "\"canvas\"");
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub const fn color(color: Color) -> Self {
        Self::Color(color)
    }

    /// A gradient through `colors`, with no angle and no other keys.
    ///
    /// # Examples
    ///
    /// ```
    /// use herdrify_app::files::{Color, Paint};
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let paint = Paint::gradient(vec![Color::new("#2a6cbd")?, Color::new("#0b2a57")?]);
    /// assert_eq!(
    ///     serde_json::to_string(&paint)?,
    ///     r##"{"gradient":["#2a6cbd","#0b2a57"]}"##,
    /// );
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn gradient(colors: Vec<Color>) -> Self {
        Self::Gradient {
            gradient: colors,
            angle: None,
            extra: Map::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    /// Reads `text` as `T` and checks that writing it back gives the same JSON.
    fn read_back<T>(text: &str) -> Result<T, serde_json::Error>
    where
        T: Serialize + for<'de> Deserialize<'de>,
    {
        let value: T = serde_json::from_str(text)?;
        let source: Value = serde_json::from_str(text)?;
        assert_eq!(serde_json::to_value(&value)?, source, "{text} changed");
        Ok(value)
    }

    #[test]
    fn paint_parses_each_shape() -> Result<(), serde_json::Error> {
        assert!(matches!(read_back::<Paint>("\"border\"")?, Paint::Color(_)));
        assert!(matches!(
            read_back::<Paint>(r##"{"gradient":["#2a6cbd","#0b2a57"],"angle":165}"##)?,
            Paint::Gradient { angle: Some(_), .. }
        ));
        assert!(matches!(
            read_back::<Paint>(r#"{"image":"a.jpg","fit":"cover"}"#)?,
            Paint::Image {
                fit: Some(Fit::Cover),
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn paint_variants_keep_extra() -> Result<(), serde_json::Error> {
        let Paint::Gradient { extra, .. } =
            read_back::<Paint>(r##"{"gradient":["#000000"],"x":1}"##)?
        else {
            return Err(serde::de::Error::custom("not a gradient"));
        };
        assert_eq!(extra.get("x"), Some(&json!(1)));
        let Paint::Image { extra, .. } = read_back::<Paint>(r#"{"image":"a.jpg","x":1}"#)? else {
            return Err(serde::de::Error::custom("not an image"));
        };
        assert_eq!(extra.get("x"), Some(&json!(1)));
        Ok(())
    }

    #[test]
    fn paint_keeps_unknown_objects() -> Result<(), serde_json::Error> {
        assert!(matches!(
            read_back::<Paint>(r#"{"board":1}"#)?,
            Paint::Other(_)
        ));
        assert!(matches!(
            read_back::<Paint>(r##"{"gradient":["#12"]}"##)?,
            Paint::Other(_)
        ));
        Ok(())
    }

    #[test]
    fn paint_refuses_a_non_object_non_colour() -> Result<(), String> {
        let error = serde_json::from_str::<Paint>("7")
            .err()
            .ok_or("7 was accepted as a paint")?;
        assert!(
            error
                .to_string()
                .starts_with("data did not match any variant of untagged enum Paint"),
            "{error}"
        );
        Ok(())
    }

    #[test]
    fn line_keeps_extra() -> Result<(), serde_json::Error> {
        let line = read_back::<Line>(r#"{"width":1,"color":"border","dash":2}"#)?;
        assert_eq!(line.extra.get("dash"), Some(&json!(2)));
        assert!(line.width.is_some() && line.color.is_some());
        Ok(())
    }

    #[test]
    fn fit_keeps_unknown_words() -> Result<(), serde_json::Error> {
        assert_eq!(read_back::<Fit>("\"contain\"")?, Fit::Contain);
        assert_eq!(read_back::<Fit>("\"tile\"")?, Fit::Other("tile".to_owned()));
        Ok(())
    }

    #[test]
    fn absent_fields_are_not_written() -> Result<(), serde_json::Error> {
        let image = read_back::<Paint>(r#"{"image":"a.jpg"}"#)?;
        assert_eq!(serde_json::to_string(&image)?, r#"{"image":"a.jpg"}"#);
        assert_eq!(serde_json::to_string(&Line::default())?, "{}");
        Ok(())
    }
}
